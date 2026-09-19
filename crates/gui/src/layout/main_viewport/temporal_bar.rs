//! Main Viewport: Temporal Rewind Scrubber Bar
//!
//! Multi-granularity time-travel navigation, timeline scrubber slider with time/CCK deltas,
//! recording on/off toggle, capacity presets, and direct CCK cycle jumping.

use crate::app::EmulatorApp;
use egui::RichText;

pub(crate) fn render_temporal_bar(app: &mut EmulatorApp, ui: &mut egui::Ui) {
    let tokens = app.theme.tokens();

    ui.vertical(|ui| {
        // Row 1: Transport Controls & Capacity Preset
        ui.horizontal_wrapped(|ui| {
            // 1. Recording Toggle Button
            let is_rec = app.session.temporal.is_recording();
            let rec_btn_text = if is_rec {
                RichText::new("⏺ Rec: ON")
                    .color(tokens.accent_success)
                    .strong()
            } else {
                RichText::new("⏸ Rec: OFF").color(tokens.accent_warning)
            };
            if ui
                .button(rec_btn_text)
                .on_hover_text("Toggle recording of execution snapshots (Alt+T)")
                .clicked()
            {
                app.session.temporal.toggle_recording();
            }

            ui.separator();

            let history_len = app.session.temporal.len();
            let can_step_back = history_len > 0 && app.session.temporal.scrub_cursor != Some(0);
            let can_step_fwd = app.session.temporal.scrub_cursor.is_some();

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
            let current_cap = app.session.temporal.capacity();
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
        let history_len = app.session.temporal.len();
        ui.horizontal_wrapped(|ui| {
            if history_len > 0 {
                let current_pos = app
                    .session
                    .temporal
                    .scrub_cursor
                    .unwrap_or(history_len.saturating_sub(1));
                let mut slider_val = current_pos;

                let cur_frame = app.session.temporal.get_chronological(current_pos);
                let head_frame = app
                    .session
                    .temporal
                    .get_chronological(history_len.saturating_sub(1));

                let (time_label, cur_cck) = match (cur_frame, head_frame) {
                    (Some(cur), Some(head)) => {
                        let delta_cck = head.cck.saturating_sub(cur.cck);
                        let delta_ms = (delta_cck as f64 * 1000.0) / 3_546_895.0; // PAL ~3.546895 MHz
                        let text = if app.session.temporal.scrub_cursor.is_some() {
                            format!("- {:.1} ms (Δ {} CCKs)", delta_ms, delta_cck)
                        } else {
                            "LIVE HEAD (0.0 ms)".to_string()
                        };
                        (text, cur.cck)
                    }
                    _ => ("0.0 ms".to_string(), 0),
                };

                let max_idx = history_len.saturating_sub(1);
                let slider_text = format!(
                    "[{}/{}]  CCK: #{}  {}",
                    current_pos + 1,
                    history_len,
                    cur_cck,
                    time_label
                );

                let slider = egui::Slider::new(&mut slider_val, 0..=max_idx)
                    .text(slider_text)
                    .show_value(false);

                if ui.add(slider).changed() && slider_val != current_pos {
                    app.session.scrub_to_frame(slider_val);
                }
            } else {
                ui.label(
                    RichText::new(
                        "No execution history recorded yet. Press F10 to step or F5 to run.",
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
            if (ui.button("Go").clicked()
                || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                && !app.target_cck_input.is_empty()
            {
                if let Ok(target) = app.target_cck_input.trim().parse::<u64>() {
                    app.session.jump_to_cck(target);
                }
            }
        });
    });
}
