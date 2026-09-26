//! MexPlug — punchy auto-mix + analog liveliness as VST3/CLAP.
//! Custom egui editor (dark console UI). Falls back to nothing (no generic UI)
//! only if the editor backend is unavailable — hosts always get the custom GUI.

mod core;
mod editor;

use atomic_float::AtomicF32;
use core::{Core, LiveParams};
use editor::{EDITOR_SIZE, MexEditor};
use nice_plug::prelude::*;
use nice_plug_egui::{EguiEditor, EguiEditorState, RepaintNotifier};
use std::num::NonZeroU32;
use std::sync::{Arc, atomic::Ordering};

pub struct MexPlug {
    params: Arc<MexPlugParams>,
    core: Core,
    editor_state: Arc<EguiEditorState>,
    peak_l: Arc<AtomicF32>,
    peak_r: Arc<AtomicF32>,
    gr_db: Arc<AtomicF32>,
    peak_decay: f32,
    repaint: RepaintNotifier,
    initial_editor: Option<MexEditor>,
}

#[derive(Params)]
struct MexPlugParams {
    #[id = "drive"]
    pub drive: FloatParam,
    #[id = "width"]
    pub width: FloatParam,
    #[id = "room"]
    pub room: FloatParam,
    #[id = "human"]
    pub human: FloatParam,
    #[id = "punch"]
    pub punch: FloatParam,
    #[id = "smooth"]
    pub smooth: FloatParam,
    #[id = "monobass"]
    pub monobass: BoolParam,
    #[id = "output"]
    pub output: FloatParam,
    #[id = "ceil"]
    pub ceil: FloatParam,
    #[id = "input"]
    pub input: FloatParam,
    #[id = "bass"]
    pub bass: FloatParam,
    #[id = "air"]
    pub air: FloatParam,
    #[id = "glue"]
    pub glue: FloatParam,
    #[id = "style"]
    pub style: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
    #[id = "monitor"]
    pub monitor: FloatParam,
    #[id = "tube"]
    pub tube: FloatParam,
    #[id = "haas"]
    pub haas: FloatParam,
    #[id = "dirt"]
    pub dirt: FloatParam,
    #[id = "schp"]
    pub schp: FloatParam,
    #[id = "balance"]
    pub balance: FloatParam,
    #[id = "theme"]
    pub theme: FloatParam,
}

impl Default for MexPlug {
    fn default() -> Self {
        let params = Arc::new(MexPlugParams::default());
        let peak_l = Arc::new(AtomicF32::new(0.0));
        let peak_r = Arc::new(AtomicF32::new(0.0));
        let gr_db = Arc::new(AtomicF32::new(0.0));
        let initial_editor =
            MexEditor::new(params.clone(), peak_l.clone(), peak_r.clone(), gr_db.clone());
        Self {
            params,
            core: Core::new(),
            editor_state: EguiEditorState::from_size(EDITOR_SIZE, 1.0),
            peak_l,
            peak_r,
            gr_db,
            peak_decay: 1.0,
            repaint: RepaintNotifier::new(),
            initial_editor: Some(initial_editor),
        }
    }
}

impl Default for MexPlugParams {
    fn default() -> Self {
        Self {
            drive: FloatParam::new(
                "Drive",
                2.0,
                FloatRange::Linear { min: 1.0, max: 4.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            width: FloatParam::new(
                "Width",
                1.18,
                FloatRange::Linear { min: 1.0, max: 1.5 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            room: FloatParam::new(
                "Room",
                0.07,
                FloatRange::Linear { min: 0.0, max: 0.25 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            human: FloatParam::new(
                "Human",
                0.6,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            punch: FloatParam::new(
                "Punch",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            smooth: FloatParam::new(
                "Smooth",
                0.25,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            monobass: BoolParam::new("Mono Bass", true),
            output: FloatParam::new(
                "Output",
                0.0,
                FloatRange::Linear {
                    min: -12.0,
                    max: 12.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            ceil: FloatParam::new(
                "Ceiling",
                -1.0,
                FloatRange::Linear {
                    min: -3.0,
                    max: -0.1,
                },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            input: FloatParam::new(
                "Input",
                0.0,
                FloatRange::Linear {
                    min: -12.0,
                    max: 12.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            bass: FloatParam::new(
                "Bass",
                0.0,
                FloatRange::Linear { min: -6.0, max: 6.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            air: FloatParam::new(
                "Air",
                1.6,
                FloatRange::Linear { min: 0.0, max: 3.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            glue: FloatParam::new(
                "Glue",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            style: FloatParam::new(
                "Style",
                1.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_step_size(1.0)
            .with_value_to_string(Arc::new(|v: f32| match v.round() as i32 {
                0 => String::from("Clean"),
                2 => String::from("Hard"),
                _ => String::from("Warm"),
            })),
            mix: FloatParam::new(
                "Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(Arc::new(|v: f32| format!("{:.0} %", v * 100.0))),
            monitor: FloatParam::new(
                "Monitor",
                0.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_step_size(1.0)
            .with_value_to_string(Arc::new(|v: f32| match v.round() as i32 {
                1 => String::from("Mid"),
                2 => String::from("Side"),
                _ => String::from("Stereo"),
            })),
            tube: FloatParam::new(
                "Tube",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            haas: FloatParam::new(
                "Haas",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            dirt: FloatParam::new(
                "Dirt",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            schp: FloatParam::new(
                "SC HP",
                20.0,
                FloatRange::Linear {
                    min: 20.0,
                    max: 500.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(0)),
            balance: FloatParam::new(
                "Balance",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            // GUI-only theme switch: hidden from the host UI, persisted
            // with the project state, settable from our own editor.
            theme: FloatParam::new(
                "Theme",
                0.0,
                FloatRange::Linear { min: 0.0, max: 4.0 },
            )
            .with_step_size(1.0)
            .hide()
            .with_value_to_string(Arc::new(|v: f32| match v.round() as i32 {
                1 => String::from("White"),
                2 => String::from("Black"),
                3 => String::from("Bootstrap"),
                4 => String::from("Flat"),
                _ => String::from("Hardware"),
            })),
        }
    }
}

impl Plugin for MexPlug {
    const NAME: &'static str = "MexPlug";
    const VENDOR: &'static str = "MexPlug";
    const URL: &'static str = "https://example.com/mexplug";
    const EMAIL: &'static str = "nobody@example.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type Editor = EguiEditor<MexEditor>;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Self::Editor> {
        // The host may open the editor any number of times: reuse the stored
        // app on first open, build a fresh one (from current values) after.
        let app = self.initial_editor.take().unwrap_or_else(|| {
            MexEditor::new(
                self.params.clone(),
                self.peak_l.clone(),
                self.peak_r.clone(),
                self.gr_db.clone(),
            )
        });
        MexEditor::make_editor(self.editor_state.clone(), self.repaint.clone(), app)
    }

    fn activate(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl ActivateContext<Self>,
    ) -> bool {
        // Allocation happens here (not on the audio thread).
        self.core.set_sample_rate(buffer_config.sample_rate);
        // Peak meter: -12 dB decay over PEAK_METER_DECAY_MS of silence.
        self.peak_decay = 0.25f64
            .powf((buffer_config.sample_rate as f64 * 150.0 / 1000.0).recip())
            as f32;
        true
    }

    fn reset(&mut self) {
        self.core.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut block_peak = [0.0f32; 2];
        for mut frame in buffer.iter_samples() {
            let p = LiveParams {
                drive: self.params.drive.smoothed.next(),
                width: self.params.width.smoothed.next(),
                room: self.params.room.smoothed.next(),
                human: self.params.human.smoothed.next(),
                out_gain: util::db_to_gain(self.params.output.smoothed.next()),
                punch: self.params.punch.smoothed.next(),
                smooth: self.params.smooth.smoothed.next(),
                monobass: self.params.monobass.value(),
                ceil_db: self.params.ceil.smoothed.next(),
                input_gain: util::db_to_gain(self.params.input.smoothed.next()),
                bass_db: self.params.bass.smoothed.next(),
                air_db: self.params.air.smoothed.next(),
                glue: self.params.glue.smoothed.next(),
                style: self.params.style.smoothed.next(),
                mix: self.params.mix.smoothed.next(),
                monitor: self.params.monitor.smoothed.next(),
                tube: self.params.tube.smoothed.next(),
                haas: self.params.haas.smoothed.next(),
                dirt: self.params.dirt.smoothed.next(),
                schp_fc: self.params.schp.smoothed.next(),
                balance: self.params.balance.smoothed.next(),
            };

            // Copy through a local array: gives simultaneous L/R access for
            // the M/S stage. `iter_mut()` is a resetting iterator, so two
            // passes over the same frame are supported.
            let mut s = [0.0f32; 2];
            let mut n = 0usize;
            for sample in frame.iter_mut() {
                if n < 2 {
                    s[n] = *sample;
                    n += 1;
                }
            }
            if n == 0 {
                continue;
            }

            self.core.process_frame(&mut s, n, &p);

            let mut i = 0usize;
            for sample in frame.iter_mut() {
                if i < n {
                    *sample = s[i];
                    i += 1;
                }
            }
            if n > 0 {
                block_peak[0] = block_peak[0].max(s[0].abs());
            }
            if n > 1 {
                block_peak[1] = block_peak[1].max(s[1].abs());
            }
        }

        // Meters for the editor (lock-free, only when open).
        if self.editor_state.is_open() {
            let decay = self.peak_decay.powf(buffer.samples() as f32);
            let mut repaint = false;
            for (atom, bp) in [(&self.peak_l, block_peak[0]), (&self.peak_r, block_peak[1])] {
                let old = atom.load(Ordering::Relaxed);
                let mut new = bp.max(old * decay);
                if new <= util::MINUS_INFINITY_GAIN {
                    new = 0.0;
                }
                if new != old {
                    atom.store(new, Ordering::Relaxed);
                    repaint = true;
                }
            }
            // Glue reduction in dB (<= 0).
            let gr_db = 20.0 * self.core.last_gr().max(1e-6).log10();
            let old_gr = self.gr_db.load(Ordering::Relaxed);
            if (gr_db - old_gr).abs() > 0.05 {
                self.gr_db.store(gr_db, Ordering::Relaxed);
                repaint = true;
            }
            if repaint {
                self.repaint.request_repaint();
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for MexPlug {
    const CLAP_ID: &'static str = "dev.mexplug.mexplug";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Punchy auto-mix + analog liveliness (de-sterilize AI tracks)");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
        ClapFeature::Distortion,
    ];
}

impl Vst3Plugin for MexPlug {
    const VST3_CLASS_ID: [u8; 16] = *b"MexPlug000000001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nice_export_clap!(MexPlug);
nice_export_vst3!(MexPlug);
