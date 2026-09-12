//! Right Dock: Trace History Log Component
//!
//! 1024-entry execution trace table with virtual scrolling and time-travel synchronization.

use crate::app::EmulatorApp;
use egui::{Color32, RichText, Ui};

pub fn render_trace_log(app: &mut EmulatorApp, ui: &mut Ui) {
    egui::CollapsingHeader::new(RichText::new("📜 Execution Trace Log").strong())
        .default_open(true)
        .show(ui, |ui| {
            let count = app.session.debugger.trace.len();
            if count == 0 {
                ui.label("No trace entries recorded yet (press F10 to step).");
                return;
            }

            ui.horizontal(|ui| {
                ui.label(format!("Recorded: {} / 1024 entries", count));
                if ui.button("Clear Log").clicked() {
                    app.session.debugger.trace.clear();
                }
            });

            ui.separator();

            // Table Header
            ui.horizontal(|ui| {
                ui.monospace(RichText::new("CCK Timestamp").strong());
                ui.monospace(RichText::new("   PC   ").strong());
                ui.monospace(RichText::new("Opcode").strong());
                ui.monospace(RichText::new("Disassembly").strong());
            });

            ui.separator();

            let row_height = 18.0;
            let mut clicked_idx = None;

            egui::ScrollArea::both()
                .id_salt("trace_log_scroll")
                .max_height(260.0)
                .auto_shrink([false, false])
                .show_rows(ui, row_height, count, |ui, row_range| {
                    for idx in row_range {
                        if let Some(entry) = app.session.debugger.trace.get(idx) {
                            let is_active = app.session.temporal.scrub_cursor == Some(idx);
                            let row_bg = if is_active {
                                Color32::from_rgb(40, 70, 100)
                            } else {
                                Color32::TRANSPARENT
                            };

                            ui.painter()
                                .rect_filled(ui.available_rect_before_wrap(), 0.0, row_bg);

                            let line_text = format!(
                                "{:<13} ${:06X}  ${:04X}  {}",
                                entry.cck, entry.pc, entry.opcode, entry.disassembly
                            );

                            ui.horizontal(|ui| {
                                let label = ui.add(
                                    egui::Label::new(RichText::new(line_text).monospace().color(
                                        if is_active {
                                            Color32::from_rgb(0, 255, 200)
                                        } else {
                                            Color32::from_rgb(210, 215, 225)
                                        },
                                    ))
                                    .sense(egui::Sense::click()),
                                );

                                if label.clicked() {
                                    clicked_idx = Some(idx);
                                }
                            });
                        }
                    }
                });

            if let Some(idx) = clicked_idx {
                app.session.scrub_to_frame(idx);
            }
        });
}
