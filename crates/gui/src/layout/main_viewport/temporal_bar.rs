//! Main Viewport: Temporal Rewind Scrubber Bar
//!
//! Multi-granularity time-travel navigation, timeline scrubber slider with time/CCK deltas,
//! recording on/off toggle, capacity presets, and direct CCK cycle jumping.

use crate::app::EmulatorApp;
use egui::RichText;

const PAL_CCK_PER_SECOND: f64 = 3_546_895.0;
const MILLISECONDS_PER_SECOND: f64 = 1000.0;

#[derive(Debug, Default)]
pub(crate) struct TemporalPresentation {
    recording: bool,
    count: usize,
    cursor: Option<usize>,
    capacity: usize,
    position: usize,
    slider_label: String,
}

impl TemporalPresentation {
    fn refresh(&mut self, history: &debugger::temporal::TemporalHistory) {
        self.recording = history.is_recording();
        self.count = history.len();
        self.cursor = history.scrub_cursor;
        self.capacity = history.capacity();
        self.position = self.cursor.unwrap_or_else(|| self.count.saturating_sub(1));
        let (label, cck) = match (
            history.get_chronological(self.position),
            history.get_chronological(self.count.saturating_sub(1)),
        ) {
            (Some(cur), Some(head)) => {
                let delta = head.cck.saturating_sub(cur.cck);
                let label = if self.cursor.is_some() {
                    format!(
                        "- {:.1} ms (Δ {} CCKs)",
                        delta as f64 * MILLISECONDS_PER_SECOND / PAL_CCK_PER_SECOND,
                        delta
                    )
                } else {
                    "LATEST RECORDED".to_string()
                };
                (label, cur.cck)
            }
            _ => ("0.0 ms".to_string(), 0),
        };
        self.slider_label = format!(
            "[{}/{}]  CCK: #{}  {}",
            self.position + 1,
            self.count,
            cck,
            label
        );
    }
}

pub(crate) fn render_temporal_bar(app: &mut EmulatorApp, ui: &mut egui::Ui) {
    let tokens = app.theme.tokens();
    if !app.inspection.frozen {
        app.inspection.temporal.refresh(&app.session.temporal);
    }
    ui.scope(|ui| {
    if app.session.is_running { ui.disable(); }

    ui.vertical(|ui| {
        // Row 1: Transport Controls & Capacity Preset
        ui.horizontal_wrapped(|ui| {
            // 1. Recording Toggle Button
            let is_rec = app.inspection.temporal.recording;
            let rec_btn_text = if is_rec {
                RichText::new("⏺ Rec: ON")
                    .color(tokens.accent_success)
                    .strong()
            } else {
                RichText::new("⏸ Rec: OFF").color(tokens.accent_warning)
            };
            if ui
                .button(rec_btn_text)
                .on_hover_text("Toggle recording of explicit instruction, CCK and frame steps (Alt+T). Continuous Run is not recorded.")
                .clicked()
            {
                app.session.temporal.toggle_recording();
            }

            ui.separator();

            let history_len = app.inspection.temporal.count;
            let can_step_back = history_len > 0 && app.inspection.temporal.cursor != Some(0);
            let can_step_fwd = app.inspection.temporal.cursor.is_some();

            // 2. Navigation Buttons
            if ui
                .add_enabled(can_step_back, egui::Button::new("⏮ First"))
                .clicked()
            {
                app.session.scrub_to_frame(0);
            }

            if ui
                .add_enabled(can_step_back, egui::Button::new("◀◀ Frame"))
                .on_hover_text("Rewind 1 PAL video frame (~70,824 CCK / 20ms)")
                .clicked()
            {
                app.session.step_backward_frame();
            }

            if ui
                .add_enabled(can_step_back, egui::Button::new("◀ -1"))
                .on_hover_text("Step back 1 instruction (Shift+F10)")
                .clicked()
            {
                app.session.step_backward();
            }

            if ui
                .button("+1 ▶")
                .on_hover_text("Step forward 1 instruction toward live head")
                .clicked()
            {
                app.session.step_forward();
            }

            if ui
                .add_enabled(can_step_fwd, egui::Button::new("Frame ▶▶"))
                .on_hover_text("Advance forward 1 PAL video frame (~70,824 CCK)")
                .clicked()
            {
                app.session.step_forward_frame();
            }

            if ui
                .add_enabled(can_step_fwd, egui::Button::new("Live Head ⏭"))
                .on_hover_text("Return to current live execution head")
                .clicked()
            {
                app.session.jump_to_live_head();
            }

            ui.separator();

            // 3. Capacity Preset Dropdown
            let presets = [
                (25_000, "25k (~0.1s)"),
                (50_000, "50k (~0.2s)"),
                (100_000, "100k (~0.4s)"),
                (250_000, "250k (~1.0s)"),
                (500_000, "500k (~2.0s)"),
            ];
            let current_cap = app.inspection.temporal.capacity;
            let label = presets
                .iter()
                .find(|(cap, _)| *cap == current_cap)
                .map(|(_, l)| *l)
                .unwrap_or("Custom Cap");

            egui::ComboBox::from_id_salt("cap_selector")
                .selected_text(label)
                .show_ui(ui, |ui| {
                    for &(cap, text) in &presets {
                        if ui
                            .selectable_value(&mut app.temporal_capacity_selection, cap, text)
                            .clicked()
                        {
                            app.session.temporal.set_capacity(cap);
                        }
                    }
                });
        });

        // Row 2: Timeline Slider & Direct CCK Target Jump
        let history_len = app.inspection.temporal.count;
        ui.horizontal_wrapped(|ui| {
            if history_len > 0 {
                let current_pos = app.inspection.temporal.position;
                let mut slider_val = current_pos;
                let max_idx = history_len.saturating_sub(1);
                let slider = egui::Slider::new(&mut slider_val, 0..=max_idx)
                    .text(&app.inspection.temporal.slider_label)
                    .show_value(false);

                if ui.add(slider).changed() && slider_val != current_pos {
                    app.session.scrub_to_frame(slider_val);
                }
            } else {
                ui.label(
                    RichText::new(
                        "No execution history recorded yet. Enable Rec and step; Run is not recorded.",
                    )
                    .color(tokens.text_muted)
                    .italics(),
                );
            }

            ui.separator();

            // Direct CCK Target Jump Input
            ui.label(RichText::new("Jump CCK:").color(tokens.text_secondary));
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.target_cck_input)
                    .desired_width(65.0)
                    .font(egui::TextStyle::Monospace),
            );
            let is_jump_requested = ui.button("Go").clicked()
                || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if ui.is_enabled() && is_jump_requested && !app.target_cck_input.is_empty() {
                if let Ok(target) = app.target_cck_input.trim().parse::<u64>() {
                    app.session.jump_to_cck(target);
                }
            }
        });
    });
    });
}
