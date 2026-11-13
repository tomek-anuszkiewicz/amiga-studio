//! Right Dock: Trace History Log Component
//!
//! 1024-entry execution trace table with virtual scrolling and time-travel synchronization.

use crate::app::EmulatorApp;
use egui::{Color32, RichText, Ui};

#[derive(Debug, Default)]
pub(crate) struct TracePresentation {
    count: usize,
    label: String,
    rows: Vec<(usize, String, bool)>,
}

pub(crate) fn render_trace_log(app: &mut EmulatorApp, ui: &mut Ui) {
    ui.scope(|ui| {
        let refresh = !app.inspection.frozen;
        if app.session.is_running {
            ui.disable();
        }
        if refresh {
            app.inspection.trace.count = app.session.debugger.trace.len();
            app.inspection.trace.label =
                format!("Recorded: {} / 1024 entries", app.inspection.trace.count);
        }
        egui::CollapsingHeader::new(RichText::new("📜 Execution Trace Log").strong())
            .default_open(true)
            .show(ui, |ui| {
                let count = app.inspection.trace.count;
                if count == 0 {
                    ui.label("No trace entries recorded yet (press F10 to step).");
                    return;
                }

                ui.horizontal(|ui| {
                    ui.label(&app.inspection.trace.label);
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
                let tokens = app.theme.tokens();

                egui::ScrollArea::both()
                    .id_salt("trace_log_scroll")
                    .enable_scrolling(!app.session.is_running)
                    .max_height(260.0)
                    .auto_shrink([false, false])
                    .show_rows(ui, row_height, count, |ui, row_range| {
                        if refresh {
                            app.inspection.trace.rows.clear();
                            for idx in row_range.clone() {
                                if let Some(entry) = app.session.debugger.trace.get(idx) {
                                    app.inspection.trace.rows.push((
                                        idx,
                                        format!(
                                            "{:<13} ${:06X}  ${:04X}  {}",
                                            entry.cck, entry.pc, entry.opcode, entry.disassembly
                                        ),
                                        app.session.temporal.scrub_cursor == Some(idx),
                                    ));
                                }
                            }
                        }
                        for (idx, line_text, is_active) in &app.inspection.trace.rows {
                            if row_range.contains(idx) {
                                let is_active = *is_active;
                                let row_bg = if is_active {
                                    tokens.accent_pc_bg
                                } else {
                                    Color32::TRANSPARENT
                                };

                                ui.painter().rect_filled(
                                    ui.available_rect_before_wrap(),
                                    0.0,
                                    row_bg,
                                );

                                ui.horizontal(|ui| {
                                    let label = ui.add(
                                        egui::Label::new(
                                            RichText::new(line_text).monospace().color(
                                                if is_active {
                                                    tokens.accent_pc
                                                } else {
                                                    tokens.text_primary
                                                },
                                            ),
                                        )
                                        .sense(egui::Sense::click()),
                                    );

                                    if label.clicked() {
                                        clicked_idx = Some(*idx);
                                    }
                                });
                            }
                        }
                    });

                if let Some(idx) = clicked_idx {
                    app.session.scrub_to_frame(idx);
                }
            });
    });
}
