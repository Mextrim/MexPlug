//! MexPlug editor (egui), Flat UI theme:
//! wet asphalt slate, clouds text, turquoise accent, flat fills, no gradients.
//! Slim knobs, preset bar with A/B compare, thin stereo meter, Mono Bass switch.

use atomic_float::AtomicF32;
use egui::{Color32, Pos2, RichText, Sense, Stroke, Vec2};
use nice_plug::context::gui::GuiContext;
use nice_plug::editor::dpi::LogicalSize;
use nice_plug::prelude::*;
use nice_plug_egui::{
    EguiEditorState, EguiNiceSettings, NiceEguiApp, RepaintNotifier, create_egui_editor,
};
use std::f32::consts::PI;
use std::sync::{Arc, atomic::Ordering};

use super::MexPlugParams;

pub const EDITOR_SIZE: LogicalSize<f32> = LogicalSize::new(920.0, 510.0);

// Palette: Flat UI — wet asphalt slate, clouds text, turquoise accent.
// (Const names kept stable; only values define the theme.)
const BG: Color32 = Color32::from_rgb(44, 62, 80);
const INK: Color32 = Color32::from_rgb(236, 240, 241);
const MUTED: Color32 = Color32::from_rgb(149, 165, 166);
const LINE: Color32 = Color32::from_rgb(52, 73, 94);
const ACCENT: Color32 = Color32::from_rgb(26, 188, 156);
const ACCENT_HOT: Color32 = Color32::from_rgb(72, 201, 176);
const BAD: Color32 = Color32::from_rgb(231, 76, 60);

/// Plain-value snapshot of all automatable params (A/B slots + presets).
/// Order: drive,width,room,human,punch,smooth,output,ceil,input,bass,air,glue,style,mix,monitor.
#[derive(Clone, Copy)]
struct AbSlot {
    v: [f32; 15],
    mono: bool,
}

impl AbSlot {
    fn capture(p: &MexPlugParams) -> Self {
        Self {
            v: [
                p.drive.value(),
                p.width.value(),
                p.room.value(),
                p.human.value(),
                p.punch.value(),
                p.smooth.value(),
                p.output.value(),
                p.ceil.value(),
                p.input.value(),
                p.bass.value(),
                p.air.value(),
                p.glue.value(),
                p.style.value(),
                p.mix.value(),
                p.monitor.value(),
            ],
            mono: p.monobass.value(),
        }
    }

    fn apply(&self, setter: &ParamSetter, p: &MexPlugParams) {
        let ps = [
            &p.drive,
            &p.width,
            &p.room,
            &p.human,
            &p.punch,
            &p.smooth,
            &p.output,
            &p.ceil,
            &p.input,
            &p.bass,
            &p.air,
            &p.glue,
            &p.style,
            &p.mix,
            &p.monitor,
        ];
        for (param, &val) in ps.iter().zip(self.v.iter()) {
            setter.begin_set_parameter(*param);
            setter.set_parameter(*param, val);
            setter.end_set_parameter(*param);
        }
        setter.begin_set_parameter(&p.monobass);
        setter.set_parameter(&p.monobass, self.mono);
        setter.end_set_parameter(&p.monobass);
    }
}

struct Preset {
    name: &'static str,
    v: [f32; 15],
    mono: bool,
}

// drive,width,room,human,punch,smooth,output,ceil,input,bass,air,glue,style,mix,monitor
// style: 0 = Clean, 1 = Warm, 2 = Hard. monitor: 0 = Stereo, 1 = Mid, 2 = Side.
const PRESETS: [Preset; 7] = [
    Preset {
        name: "Gentle Polish",
        v: [1.5, 1.12, 0.05, 0.4, 0.2, 0.3, 0.0, -1.0, 0.0, 0.0, 1.2, 0.6, 1.0, 1.0, 0.0],
        mono: true,
    },
    Preset {
        name: "AI Rescue",
        v: [2.4, 1.22, 0.08, 0.85, 0.35, 0.6, 0.0, -1.0, -1.0, -0.5, 1.0, 0.8, 1.0, 1.0, 0.0],
        mono: true,
    },
    Preset {
        name: "Club Punch",
        v: [2.8, 1.15, 0.04, 0.3, 0.7, 0.2, 1.0, -0.5, 0.0, 2.0, 1.8, 1.0, 2.0, 1.0, 0.0],
        mono: true,
    },
    Preset {
        name: "Lo-Fi Warmth",
        v: [3.2, 1.05, 0.12, 1.0, 0.15, 0.4, 0.0, -1.5, -2.0, 1.0, 0.5, 0.7, 1.0, 0.85, 0.0],
        mono: true,
    },
    Preset {
        name: "Airy Clean",
        v: [1.3, 1.28, 0.06, 0.35, 0.25, 0.35, 0.0, -1.0, 0.0, -1.0, 2.5, 0.5, 0.0, 1.0, 0.0],
        mono: true,
    },
    Preset {
        name: "Streaming Loud",
        v: [2.2, 1.15, 0.05, 0.4, 0.5, 0.3, 2.0, -1.0, 0.0, 1.0, 1.5, 1.0, 1.0, 1.0, 0.0],
        mono: true,
    },
    Preset {
        name: "Vinyl Dust",
        v: [3.0, 1.08, 0.14, 1.0, 0.2, 0.5, 0.0, -1.5, -1.0, 0.5, 0.8, 0.7, 1.0, 0.9, 0.0],
        mono: true,
    },
];

/// Editor state shared with the audio thread (meters only).
pub struct MexEditor {
    params: Arc<MexPlugParams>,
    peak_l: Arc<AtomicF32>,
    peak_r: Arc<AtomicF32>,
    gr_db: Arc<AtomicF32>,
    clip: bool,
    ab: [AbSlot; 2],
    ab_active: usize,
    gui: Option<OpenGui>,
}

struct OpenGui {
    _egui_ctx: egui::Context,
    ctx: GuiContext,
}

impl MexEditor {
    pub(crate) fn new(
        params: Arc<MexPlugParams>,
        peak_l: Arc<AtomicF32>,
        peak_r: Arc<AtomicF32>,
        gr_db: Arc<AtomicF32>,
    ) -> Self {
        let slot = AbSlot::capture(&params);
        Self {
            params,
            peak_l,
            peak_r,
            gr_db,
            clip: false,
            ab: [slot, slot],
            ab_active: 0,
            gui: None,
        }
    }

    pub fn make_editor(
        editor_state: Arc<EguiEditorState>,
        repaint: RepaintNotifier,
        app: MexEditor,
    ) -> Option<nice_plug_egui::EguiEditor<MexEditor>> {
        create_egui_editor(editor_state, repaint, EguiNiceSettings::new(), app)
    }

    fn ab_switch(
        ab: &mut [AbSlot; 2],
        ab_active: &mut usize,
        params: &Arc<MexPlugParams>,
        setter: &ParamSetter,
        target: usize,
    ) {
        if target == *ab_active {
            return;
        }
        ab[*ab_active] = AbSlot::capture(params);
        *ab_active = target;
        ab[target].apply(setter, params);
    }
}

impl NiceEguiApp for MexEditor {
    fn build(
        &mut self,
        egui_ctx: egui::Context,
        nice_gui_ctx: GuiContext,
        _frame: &mut nice_plug_egui::Frame,
    ) -> Result<(), nice_plug_egui::baseview::HandlerError> {
        self.gui = Some(OpenGui {
            _egui_ctx: egui_ctx,
            ctx: nice_gui_ctx,
        });
        Ok(())
    }

    fn editor_closed(&mut self) {
        self.gui = None;
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut nice_plug_egui::Frame) {
        let Some(gui) = self.gui.as_mut() else {
            return;
        };

        // Flat UI theme: dark slate, explicit fills everywhere.
        // The background rect is painted manually so the look never depends
        // on the host window clear color.
        let mut vis = egui::Visuals::dark();
        vis.dark_mode = true;
        vis.panel_fill = BG;
        vis.window_fill = BG;
        vis.override_text_color = Some(INK);
        vis.widgets.noninteractive.bg_fill = BG;
        vis.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
        vis.widgets.inactive.bg_fill = LINE;
        vis.widgets.hovered.bg_fill = Color32::from_rgb(62, 87, 113);
        vis.widgets.active.bg_fill = ACCENT;
        vis.widgets.active.fg_stroke = Stroke::new(1.5, BG);
        vis.selection.bg_fill = ACCENT;
        vis.selection.stroke = Stroke::new(1.0, ACCENT);
        ui.ctx().set_visuals(vis);

        let bg_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(bg_rect, 0.0, BG);

        let setter = gui.ctx.param_setter();

        // Header: wordmark + stereo meter.
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("MexPlug").size(26.0).strong().color(INK));
                ui.label(RichText::new("auto-mix · analog liveliness").size(11.0).color(MUTED));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                meter_block(
                    ui,
                    &self.peak_l,
                    &self.peak_r,
                    &self.gr_db,
                    &mut self.clip,
                );
            });
        });
        ui.add_space(2.0);
        ui.separator();

        // Preset bar + A/B.
        ui.horizontal(|ui| {
            ui.label(RichText::new("PRESET").size(10.0).color(MUTED));
            for pr in PRESETS.iter() {
                if ui
                    .button(RichText::new(pr.name).size(11.0).color(INK))
                    .clicked()
                {
                    let applied = AbSlot {
                        v: pr.v,
                        mono: pr.mono,
                    };
                    applied.apply(&setter, &self.params);
                    // Keep A/B consistent: the active slot becomes the preset.
                    self.ab[self.ab_active] = applied;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Listed B-first so that A ends up on the left.
                for name in ["B", "A"] {
                    let idx = if name == "A" { 0 } else { 1 };
                    let active = self.ab_active == idx;
                    if ui
                        .button(
                            RichText::new(name)
                                .size(12.0)
                                .strong()
                                .color(if active { ACCENT } else { MUTED }),
                        )
                        .clicked()
                    {
                        Self::ab_switch(
                            &mut self.ab,
                            &mut self.ab_active,
                            &self.params,
                            &setter,
                            idx,
                        );
                    }
                }
                ui.label(RichText::new("A/B").size(10.0).color(MUTED));
            });
        });
        ui.separator();

        // Row 1: main character knobs.
        ui.label(RichText::new("CHARACTER").size(10.0).color(MUTED));
        ui.add_space(1.0);
        ui.horizontal(|ui| {
            ui.add_space(2.0);
            for (i, (param, label, hint)) in [
                (&self.params.drive, "DRIVE", "Tape-style saturation drive"),
                (&self.params.width, "WIDTH", "Stereo width (Mid/Side)"),
                (&self.params.room, "ROOM", "Small room ambience"),
                (&self.params.human, "HUMAN", "Wow, flutter and tape noise"),
                (&self.params.punch, "PUNCH", "Transient attack emphasis"),
                (&self.params.smooth, "SMOOTH", "Tames harsh highs dynamically"),
                (&self.params.output, "OUTPUT", "Output trim before the limiter"),
                (&self.params.ceil, "CEILING", "Limiter ceiling"),
            ]
            .iter()
            .enumerate()
            {
                if i == 4 {
                    ui.separator();
                }
                knob_cell(ui, param, &setter, label, hint);
            }
        });
        ui.separator();

        // Row 2: input + tone + glue + style, mono switch on the right.
        ui.label(RichText::new("TONE · DYNAMICS").size(10.0).color(MUTED));
        ui.add_space(1.0);
        ui.horizontal(|ui| {
            ui.add_space(2.0);
            for (param, label, hint) in [
                (&self.params.input, "INPUT", "Input trim into the chain"),
                (&self.params.bass, "BASS", "Low shelf at 100 Hz"),
                (&self.params.air, "AIR", "High shelf at 8.2 kHz"),
                (&self.params.glue, "GLUE", "Glue compression amount"),
                (
                    &self.params.style,
                    "STYLE",
                    "Saturation character: Clean / Warm / Hard",
                ),
                (&self.params.mix, "MIX", "Dry/wet parallel mix"),
            ] {
                knob_cell(ui, param, &setter, label, hint);
            }
            ui.separator();
            ui.add_space(4.0);
            mono_switch(ui, &self.params.monobass, &setter);
            ui.add_space(8.0);
            monitor_seg(ui, &self.params.monitor, &setter);
        });
        ui.separator();

        // Footer hints.
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                    .size(10.0)
                    .color(MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("drag · double-click resets · shift = fine")
                        .size(11.0)
                        .color(MUTED),
                );
            });
        });

        // Keep the meters alive while open.
        ui.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

/// Stereo output meter + glue reduction meter, clip latch LED (click to clear).
fn meter_block(
    ui: &mut egui::Ui,
    peak_l: &Arc<AtomicF32>,
    peak_r: &Arc<AtomicF32>,
    gr_db: &Arc<AtomicF32>,
    clip: &mut bool,
) {
    let l = peak_l.load(Ordering::Relaxed);
    let r = peak_r.load(Ordering::Relaxed);
    let ldb = util::gain_to_db(l);
    let rdb = util::gain_to_db(r);
    if ldb > -0.3 || rdb > -0.3 {
        *clip = true;
    }
    let peak_db = ldb.max(rdb);
    let peak_txt = if peak_db <= util::MINUS_INFINITY_DB + 1.0 {
        String::from("−inf dB")
    } else {
        format!("{peak_db:.1} dB")
    };

    ui.vertical(|ui| {
        meter_bar(ui, "L", ldb, 140.0);
        ui.add_space(3.0);
        meter_bar(ui, "R", rdb, 140.0);
        ui.add_space(3.0);
        gr_bar(ui, gr_db.load(Ordering::Relaxed));
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(11.0, 11.0), Sense::click());
            ui.painter().circle_filled(
                rect.center(),
                4.5,
                if *clip { BAD } else { LINE },
            );
            if resp.clicked() {
                *clip = false;
            }
            ui.label(RichText::new("CLIP").size(10.0).color(MUTED));
            ui.add_space(6.0);
            ui.label(
                RichText::new(peak_txt)
                    .size(12.0)
                    .color(if *clip { BAD } else { INK }),
            );
        });
    });
}

/// Glue reduction bar, 0..-12 dB.
fn gr_bar(ui: &mut egui::Ui, db: f32) {
    let norm = ((-db) / 12.0).clamp(0.0, 1.0);
    let txt = format!("{db:.1} dB");
    ui.horizontal(|ui| {
        ui.label(RichText::new("GR").size(10.0).color(MUTED));
        let (rect, _) = ui.allocate_exact_size(Vec2::new(140.0, 6.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 2.0, LINE);
        if norm > 0.002 {
            let mut fill = rect;
            fill.set_right(rect.left() + rect.width() * norm);
            p.rect_filled(fill, 1.5, ACCENT);
        }
        ui.label(RichText::new(txt).size(10.0).color(MUTED));
    })
    .response
    .on_hover_text("Glue compression depth");
}

/// One knob cell: name + slim knob + live value.
fn knob_cell(
    ui: &mut egui::Ui,
    param: &FloatParam,
    setter: &ParamSetter,
    label: &str,
    hint: &'static str,
) {
    ui.allocate_ui_with_layout(
        Vec2::new(80.0, 158.0),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.label(RichText::new(label).size(10.0).color(MUTED));
            ui.add_space(1.0);
            param_knob(ui, param, setter, 62.0, hint);
            ui.add_space(1.0);
            let shown = param.modulated_normalized_value();
            ui.label(
                RichText::new(param.normalized_value_to_string(shown, true))
                    .size(12.0)
                    .color(INK),
            );
        },
    );
}

/// Slim rotary knob bound to a FloatParam (normalized mapping).
fn param_knob(
    ui: &mut egui::Ui,
    param: &FloatParam,
    setter: &ParamSetter,
    diameter: f32,
    hint: &'static str,
) {
    let mut norm = param.modulated_normalized_value();
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::click_and_drag());

    if response.drag_started() {
        setter.begin_set_parameter(param);
    }
    if response.dragged() {
        let fine = ui.input(|i| i.modifiers.shift);
        let speed = if fine { 600.0 } else { 160.0 };
        let delta = -response.drag_delta().y / speed;
        if delta != 0.0 {
            norm = (norm + delta).clamp(0.0, 1.0);
            setter.set_parameter(param, param.preview_plain(norm));
        }
    }
    if response.drag_stopped() {
        setter.end_set_parameter(param);
    }
    if response.double_clicked() {
        let def = param.default_normalized_value();
        setter.begin_set_parameter(param);
        setter.set_parameter(param, param.preview_plain(def));
        setter.end_set_parameter(param);
        norm = def;
    }

    // Paint: thin 270° sweep, needle, dot. Nothing else.
    let p = ui.painter();
    let center = rect.center();
    let radius = diameter / 2.0 - 4.0;
    let a0 = 0.75 * PI;
    let a1 = 2.25 * PI;
    let ang = a0 + norm * 1.5 * PI;
    let hot = response.hovered() || response.dragged();

    arc_line(p, center, radius, a0, a1, Stroke::new(2.5, LINE));
    if norm > 0.002 {
        arc_line(
            p,
            center,
            radius,
            a0,
            ang,
            Stroke::new(2.5, if hot { ACCENT_HOT } else { ACCENT }),
        );
    }
    let dir = Vec2::angled(ang);
    p.line_segment(
        [center, center + dir * (radius - 7.0)],
        Stroke::new(2.0, INK),
    );
    p.circle_filled(center, 2.5, if hot { ACCENT_HOT } else { ACCENT });
    response.on_hover_text(hint);
}

/// Arc polyline (egui 0.36 has no Painter::arc).
fn arc_line(p: &egui::Painter, center: Pos2, radius: f32, a0: f32, a1: f32, stroke: Stroke) {
    const N: usize = 48;
    let pts: Vec<Pos2> = (0..=N)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / N as f32;
            center + Vec2::angled(a) * radius
        })
        .collect();
    p.line(pts, stroke);
}

/// Thin level bar.
fn meter_bar(ui: &mut egui::Ui, label: &str, peak_db: f32, width: f32) {
    let norm = ((peak_db + 60.0) / 60.0).clamp(0.0, 1.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(10.0).color(MUTED));
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 6.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 2.0, LINE);
        if norm > 0.002 {
            let mut fill = rect;
            fill.set_right(rect.left() + rect.width() * norm);
            p.rect_filled(
                fill,
                1.5,
                if peak_db > -1.0 { BAD } else { INK },
            );
        }
    });
}

/// Minimal toggle for Mono Bass.
fn mono_switch(ui: &mut egui::Ui, param: &BoolParam, setter: &ParamSetter) {
    let on = param.value();
    ui.horizontal(|ui| {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(38.0, 20.0), Sense::click());
        if response.clicked() {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, !on);
            setter.end_set_parameter(param);
        }
        response.on_hover_text("Fold bass below 120 Hz to mono");
        let p = ui.painter();
        p.rect_filled(rect, 10.0, LINE);
        let cx = if on { rect.right() - 11.0 } else { rect.left() + 11.0 };
        p.circle_filled(
            egui::Pos2::new(cx, rect.center().y),
            7.0,
            if on { ACCENT } else { MUTED },
        );
        ui.vertical(|ui| {
            ui.label(RichText::new("MONO BASS").size(11.0).color(INK));
            ui.label(RichText::new("lows below 120 Hz").size(10.0).color(MUTED));
        });
    });
}

/// Segmented Stereo / Mid / Side monitor switch.
fn monitor_seg(ui: &mut egui::Ui, param: &FloatParam, setter: &ParamSetter) {
    ui.vertical(|ui| {
        ui.label(RichText::new("MONITOR").size(11.0).color(INK));
        ui.horizontal(|ui| {
            for (label, val) in [("ST", 0.0f32), ("M", 1.0), ("S", 2.0)] {
                let active = (param.value() - val).abs() < 0.5;
                let resp = ui.button(
                    RichText::new(label)
                        .size(11.0)
                        .strong()
                        .color(if active { ACCENT } else { MUTED }),
                );
                let clicked = resp.clicked();
                resp.on_hover_text("Solo Mid / Side to check mono compatibility");
                if clicked {
                    setter.begin_set_parameter(param);
                    setter.set_parameter(param, val);
                    setter.end_set_parameter(param);
                }
            }
        });
    });
}
