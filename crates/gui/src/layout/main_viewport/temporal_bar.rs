//! Main Viewport: Temporal Rewind Scrubber Bar
//!
//! Multi-granularity time-travel navigation, timeline scrubber slider with time/CCK deltas,
//! recording on/off toggle, capacity presets, and direct CCK cycle jumping.

use crate::app::EmulatorApp;
use crate::host_commands::{HistoryCommand, HostCommand};
use crate::keyboard_input::KeyCommand;
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
    fn refresh(&mut self, history: &crate::host_inspection::HistoryInspection) {
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

pub(crate) fn render_temporal_bar(
    app: &mut EmulatorApp,
    ui: &mut egui::Ui,
    history: &crate::host_inspection::HistoryInspection,
) {
    let tokens = app.theme.tokens();
    if !app.inspection.frozen {
        app.inspection.temporal.refresh(history);
    }
    ui.scope(|ui| {
    if app.host.status.running { ui.disable(); }

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
            if app.shortcut_tooltip(
                ui.button(rec_btn_text), KeyCommand::Recording,
                "Toggle recording of individual instruction, Color Clock and video field steps.",
            )
                .clicked()
            {
                app.host.queue(HostCommand::History(HistoryCommand::ToggleRecording));
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
                app.host.queue(HostCommand::History(HistoryCommand::Index(0)));
            }

            if ui
                .add_enabled(can_step_back, egui::Button::new("◀◀ Frame"))
                .on_hover_text("Rewind 1 PAL video frame (~70,824 CCK / 20ms)")
                .clicked()
            {
                app.host.queue(HostCommand::History(HistoryCommand::BackwardField));
            }

            if app.shortcut_tooltip(
                ui.add_enabled(can_step_back, egui::Button::new("◀ -1")),
                KeyCommand::HistoryBack, "Go back one instruction in the recorded history.",
            ).on_disabled_hover_text("There is no earlier instruction in the recorded history.")
                .clicked()
            {
                app.host.queue(HostCommand::History(HistoryCommand::Backward));
            }

            if ui
                .button("+1 ▶")
                .on_hover_text("Step forward 1 instruction toward live head")
                .clicked()
            {
                app.host.queue(HostCommand::History(HistoryCommand::Forward));
            }

            if ui
                .add_enabled(can_step_fwd, egui::Button::new("Frame ▶▶"))
                .on_hover_text("Advance forward 1 PAL video frame (~70,824 CCK)")
                .clicked()
            {
                app.host.queue(HostCommand::History(HistoryCommand::ForwardField));
            }

            if ui
                .add_enabled(can_step_fwd, egui::Button::new("Live Head ⏭"))
                .on_hover_text("Return to current live execution head")
                .clicked()
            {
                app.host.queue(HostCommand::History(HistoryCommand::Live));
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
                            app.host.queue(HostCommand::History(HistoryCommand::Capacity(cap)));
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
                    app.host.queue(HostCommand::History(HistoryCommand::Index(slider_val)));
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
            let resp = crate::keyboard_input::text_edit(ui,
                egui::TextEdit::singleline(&mut app.target_cck_input)
                    .desired_width(65.0)
                    .font(egui::TextStyle::Monospace),
            );
            let is_jump_requested = ui.button("Go").clicked()
                || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if ui.is_enabled() && is_jump_requested && !app.target_cck_input.is_empty() {
                if let Ok(target) = app.target_cck_input.trim().parse::<u64>() {
                    app.host.queue(HostCommand::History(HistoryCommand::Cck(target)));
                }
            }
        });
    });
    });
}
