//! MexPlug custom editor (egui): dark mastering-console UI with custom knobs,
//! a stereo output meter with clip latch, and a Mono Bass switch.

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

pub const EDITOR_SIZE: LogicalSize<f32> = LogicalSize::new(800.0, 400.0);

// Palette: near-black console, warm amber accent.
const BG: Color32 = Color32::from_rgb(13, 15, 19);
const TRACK: Color32 = Color32::from_rgb(38, 42, 50);
const TEXT: Color32 = Color32::from_rgb(232, 230, 225);
const DIM: Color32 = Color32::from_rgb(138, 143, 152);
const ACCENT: Color32 = Color32::from_rgb(245, 165, 36);
const ACCENT_HOT: Color32 = Color32::from_rgb(255, 196, 90);
const BAD: Color32 = Color32::from_rgb(242, 85, 85);

/// Editor state shared with the audio thread (meters only).
pub struct MexEditor {
    params: Arc<MexPlugParams>,
    peak_l: Arc<AtomicF32>,
    peak_r: Arc<AtomicF32>,
    clip: bool,
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
    ) -> Self {
        Self {
            params,
            peak_l,
            peak_r,
            clip: false,
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

        // Dark console theme.
        let mut vis = egui::Visuals::dark();
        vis.panel_fill = BG;
        vis.window_fill = BG;
        vis.widgets.noninteractive.fg_stroke = Stroke::new(1.0, DIM);
        ui.ctx().set_visuals(vis);

        let setter = gui.ctx.param_setter();

        // Top amber accent line.
        let avail = ui.available_width();
        let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(avail, 3.0), Sense::hover());
        ui.painter().rect_filled(bar_rect, 0.0, ACCENT);
        ui.add_space(6.0);

        // Header: title + stereo meter.
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("MEXPLUG").size(30.0).strong().color(TEXT));
                ui.label(
                    RichText::new("punchy auto-mix  ·  analog liveliness")
                        .size(12.0)
                        .color(DIM),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                meter_block(ui, &self.peak_l, &self.peak_r, &mut self.clip);
            });
        });
        ui.add_space(4.0);
        ui.separator();

        // Knob groups.
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            for (i, (param, label)) in [
                (&self.params.drive, "DRIVE"),
                (&self.params.width, "WIDTH"),
                (&self.params.room, "ROOM"),
                (&self.params.human, "HUMAN"),
                (&self.params.punch, "PUNCH"),
                (&self.params.smooth, "SMOOTH"),
                (&self.params.output, "OUTPUT"),
                (&self.params.ceil, "CEILING"),
            ]
            .iter()
            .enumerate()
            {
                if i == 4 {
                    ui.separator();
                }
                knob_cell(ui, param, &setter, label);
            }
        });
        ui.separator();

        // Bottom row: mono bass switch + hints.
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            mono_switch(ui, &self.params.monobass, &setter);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("drag knobs · double-click resets · shift = fine")
                        .size(11.0)
                        .color(DIM),
                );
            });
        });

        // Keep the meters alive while open.
        ui.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

/// Stereo output meter with clip latch LED (click to clear).
fn meter_block(
    ui: &mut egui::Ui,
    peak_l: &Arc<AtomicF32>,
    peak_r: &Arc<AtomicF32>,
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
            String::from("-inf dB")
        } else {
            format!("{peak_db:.1} dB")
        };

        ui.vertical(|ui| {
            meter_bar(ui, "L", ldb, 150.0);
            ui.add_space(2.0);
            meter_bar(ui, "R", rdb, 150.0);
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                // Clip latch LED, click to clear.
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(12.0, 12.0), Sense::click());
                ui.painter().circle_filled(
                    rect.center(),
                    5.0,
                    if *clip { BAD } else { TRACK },
                );
                if resp.clicked() {
                    *clip = false;
                }
                ui.label(RichText::new("CLIP").size(11.0).color(DIM));
                ui.add_space(6.0);
                ui.label(
                    RichText::new(peak_txt)
                        .size(12.0)
                        .color(if *clip { BAD } else { TEXT }),
                );
            });
        });
}

/// One knob cell: knob + name + live value.
fn knob_cell(ui: &mut egui::Ui, param: &FloatParam, setter: &ParamSetter, label: &str) {
    ui.allocate_ui_with_layout(
        Vec2::new(86.0, 168.0),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            param_knob(ui, param, setter, 76.0);
            ui.add_space(2.0);
            ui.label(RichText::new(label).size(11.0).color(DIM).strong());
            let shown = param.modulated_normalized_value();
            ui.label(
                RichText::new(param.normalized_value_to_string(shown, true))
                    .size(12.0)
                    .color(TEXT),
            );
        },
    );
}

/// Custom rotary knob bound to a FloatParam (normalized mapping).
fn param_knob(ui: &mut egui::Ui, param: &FloatParam, setter: &ParamSetter, diameter: f32) {
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

    // Paint: 270° sweep, bottom-left -> top -> bottom-right.
    let p = ui.painter();
    let center = rect.center();
    let radius = diameter / 2.0 - 5.0;
    let a0 = 0.75 * PI;
    let a1 = 2.25 * PI;
    let ang = a0 + norm * 1.5 * PI;
    let hot = response.hovered() || response.dragged();

    p.circle_filled(center, radius + 3.0, Color32::from_rgb(22, 25, 31));
    arc_line(&p, center, radius, a0, a1, Stroke::new(5.0, TRACK));
    if norm > 0.002 {
        arc_line(
            &p,
            center,
            radius,
            a0,
            ang,
            Stroke::new(5.0, if hot { ACCENT_HOT } else { ACCENT }),
        );
    }
    let dir = Vec2::angled(ang);
    p.line_segment(
        [center, center + dir * (radius - 10.0)],
        Stroke::new(2.5, TEXT),
    );
    p.circle_filled(center, 3.0, if hot { ACCENT_HOT } else { ACCENT });
    if response.hovered() {
        p.circle_stroke(center, radius + 3.0, Stroke::new(1.0, DIM));
    }
}

/// Arc polyline (egui 0.36 has no Painter::arc): 270° knob sweep.
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

/// Horizontal level bar with dB readout.
fn meter_bar(ui: &mut egui::Ui, label: &str, peak_db: f32, width: f32) {
    let norm = ((peak_db + 60.0) / 60.0).clamp(0.0, 1.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0).color(DIM));
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 10.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 3.0, TRACK);
        if norm > 0.002 {
            let mut fill = rect;
            fill.set_right(rect.left() + rect.width() * norm);
            p.rect_filled(
                fill.shrink(1.5),
                2.0,
                if peak_db > -1.0 { BAD } else { ACCENT },
            );
        }
    });
}

/// Jennings-style toggle for Mono Bass.
fn mono_switch(ui: &mut egui::Ui, param: &BoolParam, setter: &ParamSetter) {
    let on = param.value();
    ui.horizontal(|ui| {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(46.0, 24.0), Sense::click());
        if response.clicked() {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, !on);
            setter.end_set_parameter(param);
        }
        let p = ui.painter();
        p.rect_filled(
            rect,
            12.0,
            if on {
                Color32::from_rgb(94, 64, 18)
            } else {
                TRACK
            },
        );
        let cx = if on { rect.right() - 13.0 } else { rect.left() + 13.0 };
        p.circle_filled(
            egui::Pos2::new(cx, rect.center().y),
            9.0,
            if on { ACCENT } else { DIM },
        );
        ui.vertical(|ui| {
            ui.label(RichText::new("MONO BASS").size(12.0).color(TEXT).strong());
            ui.label(RichText::new("lows below 120 Hz").size(11.0).color(DIM));
        });
    });
}
