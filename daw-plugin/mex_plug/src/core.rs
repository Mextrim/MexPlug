//! Real-time-safe DSP core for MexPlug (ported from the offline WAV tool).
//!
//! Chain: DC-block -> EQ (HP 28 Hz, mud-cut 320 Hz, air 8.2 kHz) -> mono bass
//! (-> tanh saturation -> M/S width -> punch -> wow/flutter -> room -> smooth
//! (de-harsh) -> tape noise -> glue 2:1 -> output trim -> limiter.
//!
//! Differences vs the offline version (honest list):
//! - Wow/flutter uses a bounded delay line instead of whole-buffer resampling.
//! - The final "normalize whole file to -1 dBFS" (non-causal) is replaced by a
//!   simple peak limiter with adjustable ceiling.
//! - Per-sample drive micro-variation from the offline version is dropped;
//!   `drive` itself is sample-smoothed, which gives the same class of effect.
//! - No allocation after `set_sample_rate()`: safe for the audio thread.

use std::f64::consts::TAU;

/// Smoothed parameter snapshot for one sample frame.
#[derive(Clone, Copy)]
pub struct LiveParams {
    pub drive: f32,
    pub width: f32,
    pub room: f32,
    pub human: f32,
    /// Output trim as linear gain (dB -> gain conversion happens in lib.rs).
    pub out_gain: f32,
    /// Transient emphasis 0..1.
    pub punch: f32,
    /// HF harshness tamer 0..1.
    pub smooth: f32,
    /// Fold bass below ~120 Hz to mono.
    pub monobass: bool,
    /// Limiter ceiling in dBFS (e.g. -1.0).
    pub ceil_db: f32,
}

#[derive(Clone, Copy, Default)]
struct BiquadCoef {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

#[derive(Default)]
struct Biquad {
    c: BiquadCoef,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    fn run(&mut self, x: f64) -> f64 {
        let y = self.c.b0 * x + self.c.b1 * self.x1 + self.c.b2 * self.x2
            - self.c.a1 * self.y1
            - self.c.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    fn clear(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

fn coef(b0: f64, b1: f64, b2: f64, a0: f64, a1: f64, a2: f64) -> BiquadCoef {
    BiquadCoef {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

fn lowpass(sr: f64, fc: f64, q: f64) -> BiquadCoef {
    let w = TAU * fc / sr;
    let (s, c) = w.sin_cos();
    let a = s / (2.0 * q);
    coef(
        (1.0 - c) / 2.0,
        1.0 - c,
        (1.0 - c) / 2.0,
        1.0 + a,
        -2.0 * c,
        1.0 - a,
    )
}

fn highpass(sr: f64, fc: f64, q: f64) -> BiquadCoef {
    let w = TAU * fc / sr;
    let (s, c) = w.sin_cos();
    let a = s / (2.0 * q);
    coef(
        (1.0 + c) / 2.0,
        -(1.0 + c),
        (1.0 + c) / 2.0,
        1.0 + a,
        -2.0 * c,
        1.0 - a,
    )
}

fn peak(sr: f64, fc: f64, q: f64, gdb: f64) -> BiquadCoef {
    let a_pow = 10.0f64.powf(gdb / 40.0);
    let w = TAU * fc / sr;
    let (s, c) = w.sin_cos();
    let a = s / (2.0 * q);
    coef(
        1.0 + a * a_pow,
        -2.0 * c,
        1.0 - a * a_pow,
        1.0 + a / a_pow,
        -2.0 * c,
        1.0 - a / a_pow,
    )
}

fn highshelf(sr: f64, fc: f64, slope: f64, gdb: f64) -> BiquadCoef {
    let a_pow = 10.0f64.powf(gdb / 40.0);
    let w = TAU * fc / sr;
    let (s, c) = w.sin_cos();
    let a = s / 2.0 * ((a_pow + 1.0 / a_pow) * (1.0 / slope - 1.0) + 2.0).sqrt();
    let sq = a_pow.sqrt();
    coef(
        a_pow * ((a_pow + 1.0) + (a_pow - 1.0) * c + 2.0 * sq * a),
        -2.0 * a_pow * ((a_pow - 1.0) + (a_pow + 1.0) * c),
        a_pow * ((a_pow + 1.0) + (a_pow - 1.0) * c - 2.0 * sq * a),
        (a_pow + 1.0) - (a_pow - 1.0) * c + 2.0 * sq * a,
        2.0 * ((a_pow - 1.0) - (a_pow + 1.0) * c),
        (a_pow + 1.0) - (a_pow - 1.0) * c - 2.0 * sq * a,
    )
}

#[derive(Default)]
struct Channel {
    dc_x1: f64,
    dc_y1: f64,
    hp: Biquad,
    mud: Biquad,
    air: Biquad,
    mhp1: Biquad,
    mhp2: Biquad,
    sustain: f64,
    smooth_lp: f64,
    wow_buf: Vec<f32>,
    comb: Vec<Vec<f32>>,
    comb_idx: [usize; 4],
    ap_buf: Vec<f32>,
    ap_idx: usize,
    noise_lp: f64,
}

pub struct Core {
    sr: f64,
    chan: [Channel; 2],
    mlp1: Biquad,
    mlp2: Biquad,
    sustain_c: f64,
    smooth_lp_c: f64,
    smooth_atk: f64,
    smooth_rel: f64,
    smooth_env: f64,
    wow_pos: usize,
    wow_len: usize,
    max_d: usize,
    wow_p1: f64,
    wow_p2: f64,
    glue_env: f64,
    glue_atk: f64,
    glue_rel: f64,
    lim_env: f64,
    lim_rel: f64,
    rng: u64,
}

// Fixed chain constants (match the offline tool).
const GLUE_THRESH: f64 = 0.1258925411794167; // 10^(-18/20)
const GLUE_MAKEUP: f64 = 1.4125375446227544; // 10^(+3/20)
const SMOOTH_THRESH: f64 = 0.1; // ~-20 dBFS HF detector threshold
const DC_R: f64 = 0.995;

impl Core {
    pub fn new() -> Self {
        Self {
            sr: 0.0,
            chan: [Channel::default(), Channel::default()],
            mlp1: Biquad::default(),
            mlp2: Biquad::default(),
            sustain_c: 0.0,
            smooth_lp_c: 0.0,
            smooth_atk: 0.0,
            smooth_rel: 0.0,
            smooth_env: 0.0,
            wow_pos: 0,
            wow_len: 0,
            max_d: 0,
            wow_p1: 0.0,
            wow_p2: 0.0,
            glue_env: 0.0,
            glue_atk: 0.0,
            glue_rel: 0.0,
            lim_env: 0.0,
            lim_rel: 0.0,
            rng: 0xA11CE77D9A25u64,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        let sr = sample_rate as f64;
        self.sr = sr;

        let hp = highpass(sr, 28.0, 0.707);
        let mud = peak(sr, 320.0, 0.9, -1.2);
        let air = highshelf(sr, 8200.0, 0.8, 1.6);
        for ch in self.chan.iter_mut() {
            ch.hp.c = hp;
            ch.mud.c = mud;
            ch.air.c = air;
        }

        // Mono bass LR4 crossover at ~120 Hz (Q=0.5 stages).
        let mlp = lowpass(sr, 120.0, 0.5);
        let mhp = highpass(sr, 120.0, 0.5);
        self.mlp1.c = mlp;
        self.mlp2.c = mlp;
        for ch in self.chan.iter_mut() {
            ch.mhp1.c = mhp;
            ch.mhp2.c = mhp;
        }
        // Punch sustain ~30 ms, smooth detector at 6 kHz.
        self.sustain_c = (-1.0 / (sr * 0.030)).exp();
        self.smooth_lp_c = (-TAU * 6000.0 / sr).exp();
        self.smooth_atk = (-1.0 / (sr * 0.001)).exp();
        self.smooth_rel = (-1.0 / (sr * 0.060)).exp();

        // Glue 2:1, attack 10 ms, release 140 ms.
        self.glue_atk = (-1.0 / (sr * 10.0 / 1000.0)).exp();
        self.glue_rel = (-1.0 / (sr * 140.0 / 1000.0)).exp();
        // Limiter release ~80 ms, attack instantaneous.
        self.lim_rel = (-1.0 / (sr * 80.0 / 1000.0)).exp();

        // Wow delay line: same ~2.5 ms window as offline.
        self.max_d = (sr / 400.0).round() as usize;
        if self.max_d < 8 {
            self.max_d = 8;
        }
        self.wow_len = self.max_d + 4;
        for ch in self.chan.iter_mut() {
            ch.wow_buf = vec![0.0f32; self.wow_len];
        }

        // Small room: same comb/allpass lengths as offline.
        const BASE_MS: [usize; 4] = [29, 37, 44, 53];
        for (c, ch) in self.chan.iter_mut().enumerate() {
            let seed_off = c * 97;
            ch.comb = Vec::with_capacity(4);
            for k in 0..4 {
                let mut len = (sr as usize * (BASE_MS[k] + seed_off % 7)) / 1000;
                if len < 16 {
                    len = 16;
                }
                ch.comb.push(vec![0.0f32; len]);
            }
            let mut ap_len = (sr as usize * 7) / 1000;
            if ap_len < 16 {
                ap_len = 16;
            }
            ch.ap_buf = vec![0.0f32; ap_len];
        }

        self.reset();
    }

    pub fn reset(&mut self) {
        for ch in self.chan.iter_mut() {
            ch.dc_x1 = 0.0;
            ch.dc_y1 = 0.0;
            ch.hp.clear();
            ch.mud.clear();
            ch.air.clear();
            ch.mhp1.clear();
            ch.mhp2.clear();
            ch.sustain = 0.0;
            ch.smooth_lp = 0.0;
            for b in ch.wow_buf.iter_mut() {
                *b = 0.0;
            }
            for b in ch.comb.iter_mut() {
                for s in b.iter_mut() {
                    *s = 0.0;
                }
            }
            ch.comb_idx = [0; 4];
            for s in ch.ap_buf.iter_mut() {
                *s = 0.0;
            }
            ch.ap_idx = 0;
            ch.noise_lp = 0.0;
        }
        self.smooth_env = 0.0;
        self.mlp1.clear();
        self.mlp2.clear();
        self.wow_pos = 0;
        self.wow_p1 = 0.0;
        self.wow_p2 = 0.0;
        self.glue_env = 0.0;
        self.lim_env = 0.0;
    }

    fn next_u01(&mut self) -> f64 {
        // xorshift64*, no dependency, no allocation.
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        ((x >> 11) as f64) * (1.0 / 9007199254740992.0)
    }

    /// Process one frame in place. `n` is 1 (mono) or 2 (stereo).
    pub fn process_frame(&mut self, s: &mut [f32; 2], n: usize, p: &LiveParams) {
        if self.sr <= 0.0 || n == 0 {
            return;
        }
        let n = n.min(2);
        let drive = p.drive.max(0.5) as f64;
        let sat_norm = (drive * 0.9).tanh().max(1e-6);

        // Stage 1 per channel: DC-block + EQ.
        for c in 0..n {
            let ch = &mut self.chan[c];
            let mut x = s[c] as f64;
            let y = x - ch.dc_x1 + DC_R * ch.dc_y1;
            ch.dc_x1 = x;
            ch.dc_y1 = y;
            x = y;
            x = ch.hp.run(x);
            x = ch.mud.run(x);
            x = ch.air.run(x);
            s[c] = x as f32;
        }

        // Stage 2: mono bass via LR4 crossover at ~120 Hz.
        // Lows go mono (tight, mastering-style low end), highs stay stereo.
        if p.monobass {
            let mid_in = if n == 2 {
                (s[0] as f64 + s[1] as f64) * 0.5
            } else {
                s[0] as f64
            };
            let m = self.mlp2.run(self.mlp1.run(mid_in));
            for c in 0..n {
                let ch = &mut self.chan[c];
                let x = s[c] as f64;
                let h = ch.mhp2.run(ch.mhp1.run(x));
                s[c] = (h + m) as f32;
            }
        }

        // Stage 3 per channel: tanh saturation.
        for c in 0..n {
            let x = s[c] as f64;
            s[c] = ((x * drive).tanh() / sat_norm * 0.92) as f32;
        }

        // Stage 4: M/S width (stereo only).
        if n == 2 {
            let w = p.width as f64;
            let mid = (s[0] as f64 + s[1] as f64) * 0.5;
            let side = (s[0] as f64 - s[1] as f64) * 0.5 * w;
            s[0] = (mid + side) as f32;
            s[1] = (mid - side) as f32;
        }

        // Stage 5: punch — transient emphasis (sustain/transient split).
        let punch = p.punch.clamp(0.0, 1.0) as f64;
        if punch > 0.001 {
            let sc = self.sustain_c;
            for c in 0..n {
                let ch = &mut self.chan[c];
                let x = s[c] as f64;
                ch.sustain = sc * ch.sustain + (1.0 - sc) * x;
                let tr = x - ch.sustain;
                s[c] = (ch.sustain + tr * (1.0 + 2.0 * punch)) as f32;
            }
        }

        // Stage 6: wow/flutter via shared-LFO delay line.
        let human = p.human.clamp(0.0, 1.0) as f64;
        if human > 0.01 && self.wow_len > 0 {
            self.wow_p1 = (self.wow_p1 + 0.45 / self.sr) % 1.0;
            self.wow_p2 = (self.wow_p2 + 1.1 / self.sr) % 1.0;
            let lfo = (TAU * self.wow_p1).sin() + 0.5 * (TAU * self.wow_p2 + 1.3).sin();
            let depth = self.max_d as f64 * 0.55 * human;
            let dly = self.max_d as f64 * 0.5 + depth * 0.5 * lfo;
            let len = self.wow_len as f64;
            for c in 0..n {
                let ch = &mut self.chan[c];
                let buf = &mut ch.wow_buf;
                buf[self.wow_pos] = s[c];
                let mut rpos = self.wow_pos as f64 - dly;
                while rpos < 0.0 {
                    rpos += len;
                }
                let p0 = rpos.floor() as usize % self.wow_len;
                let p1 = (p0 + 1) % self.wow_len;
                let fr = (rpos - rpos.floor()) as f32;
                s[c] = buf[p0] + (buf[p1] - buf[p0]) * fr;
            }
            self.wow_pos = (self.wow_pos + 1) % self.wow_len;
        } else if self.wow_len > 0 {
            // Keep the delay line fed so toggling `human` does not click hard.
            for c in 0..n {
                self.chan[c].wow_buf[self.wow_pos] = s[c];
            }
            self.wow_pos = (self.wow_pos + 1) % self.wow_len;
        }

        // Stage 7: small room.
        let wet = p.room.clamp(0.0, 0.25) as f64;
        if wet > 0.001 {
            for c in 0..n {
                s[c] = self.room_run(c, s[c], wet);
            }
        }

        // Stage 8: smooth — dynamic HF tamer against harsh AI highs.
        let smooth = p.smooth.clamp(0.0, 1.0) as f64;
        if smooth > 0.001 {
            let lc = self.smooth_lp_c;
            let mut hs = [0.0f64; 2];
            for c in 0..n {
                let ch = &mut self.chan[c];
                let x = s[c] as f64;
                ch.smooth_lp = lc * ch.smooth_lp + (1.0 - lc) * x;
                hs[c] = x - ch.smooth_lp;
            }
            let mut det = 0.0f64;
            for c in 0..n {
                det = det.max(hs[c].abs());
            }
            if det > self.smooth_env {
                self.smooth_env =
                    self.smooth_atk * self.smooth_env + (1.0 - self.smooth_atk) * det;
            } else {
                self.smooth_env =
                    self.smooth_rel * self.smooth_env + (1.0 - self.smooth_rel) * det;
            }
            let gr = if self.smooth_env > SMOOTH_THRESH {
                (SMOOTH_THRESH / self.smooth_env).powf(0.7)
            } else {
                1.0
            };
            let cut = (1.0 - gr) * smooth;
            for c in 0..n {
                s[c] = (s[c] as f64 - hs[c] * cut) as f32;
            }
        }

        // Stage 9: tape noise.
        if human > 0.01 {
            let noise_amp = 10.0f64.powf(-70.0 / 20.0) * (0.4 + human);
            for c in 0..n {
                let w = self.next_u01() * 2.0 - 1.0;
                let ch = &mut self.chan[c];
                ch.noise_lp = 0.94 * ch.noise_lp + 0.06 * w;
                s[c] += (ch.noise_lp * noise_amp * 2.0) as f32;
            }
        }

        // Stage 10: glue compressor, shared envelope (matches offline).
        {
            let mut det = 0.0f64;
            for c in 0..n {
                det = det.max(s[c].abs() as f64);
            }
            if det > self.glue_env {
                self.glue_env = self.glue_atk * self.glue_env + (1.0 - self.glue_atk) * det;
            } else {
                self.glue_env = self.glue_rel * self.glue_env + (1.0 - self.glue_rel) * det;
            }
            let mut gr = 1.0;
            if self.glue_env > GLUE_THRESH {
                gr = (GLUE_THRESH / self.glue_env).powf(1.0 - 1.0 / 2.0);
            }
            let g = (gr * GLUE_MAKEUP * p.out_gain as f64) as f32;
            for c in 0..n {
                s[c] *= g;
            }
        }

        // Stage 11: peak limiter at the adjustable ceiling.
        {
            let ceil = 10.0f64.powf(p.ceil_db.clamp(-3.0, -0.1) as f64 / 20.0);
            let mut det = 0.0f64;
            for c in 0..n {
                det = det.max(s[c].abs() as f64);
            }
            if det > self.lim_env {
                self.lim_env = det;
            } else {
                self.lim_env = self.lim_rel * self.lim_env + (1.0 - self.lim_rel) * det;
            }
            let g = if self.lim_env > ceil {
                ceil / self.lim_env
            } else {
                1.0
            };
            for c in 0..n {
                s[c] = (s[c] as f64 * g) as f32;
            }
        }
    }

    fn room_run(&mut self, c: usize, x: f32, wet: f64) -> f32 {
        let ch = &mut self.chan[c];
        let mut r = 0.0f64;
        for k in 0..4 {
            let idx = ch.comb_idx[k];
            let d = ch.comb[k][idx] as f64;
            ch.comb[k][idx] = (x as f64 + d * 0.72) as f32;
            ch.comb_idx[k] = (idx + 1) % ch.comb[k].len();
            r += d;
        }
        r *= 0.25;
        let ad = ch.ap_buf[ch.ap_idx] as f64;
        let y = -r + ad * 0.5;
        ch.ap_buf[ch.ap_idx] = (r + ad * 0.5) as f32;
        ch.ap_idx = (ch.ap_idx + 1) % ch.ap_buf.len();
        (x as f64 * (1.0 - wet) + (r * 0.6 + y * 0.4) * wet * 2.0) as f32
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_params() -> LiveParams {
        LiveParams {
            drive: 2.0,
            width: 1.18,
            room: 0.07,
            human: 0.6,
            out_gain: 1.0,
            punch: 0.3,
            smooth: 0.25,
            monobass: true,
            ceil_db: -1.0,
        }
    }

    fn render(n_ch: usize, frames: usize, p: &LiveParams) -> Vec<Vec<f32>> {
        render_freq(n_ch, frames, p, 440.0, 0.5, 0.45)
    }

    fn render_freq(
        n_ch: usize,
        frames: usize,
        p: &LiveParams,
        freq: f32,
        amp_l: f32,
        amp_r: f32,
    ) -> Vec<Vec<f32>> {
        let mut core = Core::new();
        core.set_sample_rate(44100.0);
        let mut out = vec![vec![0.0f32; frames]; n_ch];
        for i in 0..frames {
            let t = i as f32 / 44100.0;
            let v = (2.0 * std::f32::consts::PI * freq * t).sin();
            let mut s = [v * amp_l, v * amp_r];
            core.process_frame(&mut s, n_ch, p);
            for c in 0..n_ch {
                out[c][i] = s[c];
            }
        }
        out
    }

    fn peak(out: &[Vec<f32>]) -> f32 {
        let mut peak = 0.0f32;
        for ch in out {
            for &v in ch {
                assert!(v.is_finite(), "non-finite sample");
                peak = peak.max(v.abs());
            }
        }
        peak
    }

    #[test]
    fn output_is_finite_and_limited() {
        let p = default_params();
        for n_ch in [1, 2] {
            let pk = peak(&render(n_ch, 44100, &p));
            assert!(pk > 0.05, "signal lost, peak={pk}");
            assert!(pk <= 0.892, "limiter ceiling violated, peak={pk}");
        }
    }

    #[test]
    fn effect_is_active() {
        let p = default_params();
        let out = render(2, 44100, &p);
        // Processed sine must differ from a clean sine (EQ+sat+width change it).
        let mut diff = 0.0f32;
        for i in 0..44100 {
            let t = i as f32 / 44100.0;
            let clean = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
            diff = diff.max((out[0][i] - clean).abs());
        }
        assert!(diff > 1e-3, "effect seems bypassed, diff={diff}");
    }

    #[test]
    fn silence_stays_sane() {
        let p = default_params();
        let mut core = Core::new();
        core.set_sample_rate(48000.0);
        let mut pk = 0.0f32;
        for _ in 0..48000 {
            let mut s = [0.0f32, 0.0];
            core.process_frame(&mut s, 2, &p);
            pk = pk.max(s[0].abs()).max(s[1].abs());
            assert!(s[0].is_finite() && s[1].is_finite());
        }
        // Only low tape noise should be present.
        assert!(pk < 0.01, "silence blew up, peak={pk}");
    }

    #[test]
    fn monobass_folds_low_end_to_mono() {
        // L = 60 Hz, R = 90 Hz: with mono bass the low bands of both
        // outputs must become (nearly) identical.
        use std::f64::consts::TAU;
        let mut base = default_params();
        base.drive = 1.5;
        base.width = 1.0;
        base.room = 0.0;
        base.human = 0.0;
        base.punch = 0.0;
        base.smooth = 0.0;
        let mut on = default_params();
        let mut off = default_params();
        for (dst, src) in [(&mut on, &base), (&mut off, &base)] {
            dst.drive = src.drive;
            dst.width = src.width;
            dst.room = src.room;
            dst.human = src.human;
            dst.punch = src.punch;
            dst.smooth = src.smooth;
        }
        on.monobass = true;
        off.monobass = false;
        let meas = |p: &LiveParams| {
            let mut core = Core::new();
            core.set_sample_rate(44100.0);
            let c = (-TAU * 120.0 / 44100.0).exp();
            let (mut lp_l, mut lp_r) = (0.0f64, 0.0f64);
            let mut diff = 0.0f64;
            for i in 0..44100 {
                let t = i as f32 / 44100.0;
                let l = (2.0 * std::f32::consts::PI * 60.0 * t).sin() * 0.5;
                let r = (2.0 * std::f32::consts::PI * 90.0 * t).sin() * 0.5;
                let mut s = [l, r];
                core.process_frame(&mut s, 2, p);
                lp_l = c * lp_l + (1.0 - c) * s[0] as f64;
                lp_r = c * lp_r + (1.0 - c) * s[1] as f64;
                if i > 22050 {
                    diff = diff.max((lp_l - lp_r).abs());
                }
            }
            diff
        };
        let d_on = meas(&on);
        let d_off = meas(&off);
        assert!(d_off > 0.25, "test signal has no L/R low diff, {d_off}");
        assert!(d_on < 0.12, "mono bass did not fold lows, diff={d_on}");
    }

    #[test]
    fn punch_boosts_transients() {
        // Short quiet burst (so the limiter doesn't mask the effect).
        // Room/wow are zeroed so they don't smear the transient.
        let mut hi = default_params();
        hi.punch = 1.0;
        hi.room = 0.0;
        hi.human = 0.0;
        hi.width = 1.0;
        let mut lo = default_params();
        lo.punch = 0.0;
        lo.room = 0.0;
        lo.human = 0.0;
        lo.width = 1.0;
        let burst = |p: &LiveParams| {
            let mut core = Core::new();
            core.set_sample_rate(44100.0);
            let mut pk = 0.0f32;
            for i in 0..44100 {
                let v = if i < 64 {
                    let t = i as f32 / 44100.0;
                    (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.15
                } else {
                    0.0
                };
                let mut s = [v, v];
                core.process_frame(&mut s, 2, p);
                pk = pk.max(s[0].abs()).max(s[1].abs());
            }
            pk
        };
        let pk_hi = burst(&hi);
        let pk_lo = burst(&lo);
        assert!(
            pk_hi > pk_lo * 1.15,
            "punch has no effect, hi={pk_hi} lo={pk_lo}"
        );
    }

    #[test]
    fn smooth_tames_harsh_highs() {
        // Quieter signal + no room/wow (no reverb buildup, no peak wobble);
        // punch is zeroed so it doesn't boost the test tone.
        let mut hi = default_params();
        hi.smooth = 1.0;
        hi.punch = 0.0;
        hi.room = 0.0;
        hi.human = 0.0;
        hi.width = 1.0;
        hi.monobass = false;
        let mut lo = default_params();
        lo.smooth = 0.0;
        lo.punch = 0.0;
        lo.room = 0.0;
        lo.human = 0.0;
        lo.width = 1.0;
        lo.monobass = false;
        let meas = |p: &LiveParams| {
            // Skip the first 250 ms so the HF detector can engage:
            // the peak would otherwise come from the pre-engagement transient.
            let mut core = Core::new();
            core.set_sample_rate(44100.0);
            let mut pk = 0.0f32;
            for i in 0..44100 {
                let t = i as f32 / 44100.0;
                let v = (2.0 * std::f32::consts::PI * 10000.0 * t).sin() * 0.25;
                let mut s = [v, v];
                core.process_frame(&mut s, 2, p);
                if i > 11025 {
                    pk = pk.max(s[0].abs()).max(s[1].abs());
                }
            }
            assert!(pk.is_finite());
            pk
        };
        let pk_hi = meas(&hi);
        let pk_lo = meas(&lo);
        assert!(
            pk_hi < pk_lo * 0.9,
            "smooth has no effect, hi={pk_hi} lo={pk_lo}"
        );
    }

    #[test]
    fn ceiling_is_respected() {
        let mut p = default_params();
        p.ceil_db = -3.0;
        let mut core = Core::new();
        core.set_sample_rate(44100.0);
        let mut pk = 0.0f32;
        for i in 0..44100 {
            let t = i as f32 / 44100.0;
            let v = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.9;
            let mut s = [v, v * 0.9];
            core.process_frame(&mut s, 2, &p);
            pk = pk.max(s[0].abs()).max(s[1].abs());
        }
        assert!(pk > 0.5, "signal lost, peak={pk}");
        assert!(pk <= 0.710, "ceiling violated, peak={pk}");
    }
}
