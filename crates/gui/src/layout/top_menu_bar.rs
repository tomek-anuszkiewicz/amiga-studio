//! Top Menu Bar Component
//!
//! File management, execution controls, status badges, and visual preferences.

use crate::app::EmulatorApp;
use crate::theme::AppTheme;

pub fn render_top_menu_bar(app: &mut EmulatorApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_menu_bar")
        .frame(egui::Frame::menu(&ctx.style()))
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = true;

                // --- 1. File Menu ---
                ui.menu_button("File", |ui| {
                    if ui.button("📁 Load Binary... (Ctrl+O)").clicked() {
                        app.open_load_binary_dialog();
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("🔄 Reset (Ctrl+R)").clicked() {
                        app.session.reset();
                        ui.close_menu();
                    }

                    if ui.button("⚡ Reset Warm").clicked() {
                        app.session.reset_warm();
                        ui.close_menu();
                    }
                });

                // --- 2. State Menu ---
                ui.menu_button("State", |ui| {
                    if ui.button("💾 Save State to File... (Ctrl+S)").clicked() {
                        open_save_state_dialog(app);
                        ui.close_menu();
                    }

                    if ui.button("📂 Load State from File... (Ctrl+L)").clicked() {
                        open_load_state_dialog(app);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("⚡ Quick Save Slot 1 (F6)").clicked() {
                        quick_save_slot(app, 1);
                        ui.close_menu();
                    }

                    let slot1_label = if app.session.has_quick_slot(1) {
                        "🔄 Quick Load Slot 1 (F9)"
                    } else {
                        "🔄 Quick Load Slot 1 (Empty)"
                    };
                    if ui
                        .add_enabled(
                            app.session.has_quick_slot(1),
                            egui::Button::new(slot1_label),
                        )
                        .clicked()
                    {
                        quick_load_slot(app, 1);
                        ui.close_menu();
                    }

                    ui.separator();

                    ui.menu_button("Slots 2–5", |ui| {
                        for slot in 2..=5 {
                            ui.horizontal(|ui| {
                                if ui.button(format!("Save #{slot}")).clicked() {
                                    quick_save_slot(app, slot);
                                }
                                let loaded = app.session.has_quick_slot(slot);
                                let btn = egui::Button::new(if loaded {
                                    format!("Load #{slot}")
                                } else {
                                    format!("#{slot} (Empty)")
                                });
                                if ui.add_enabled(loaded, btn).clicked() {
                                    quick_load_slot(app, slot);
                                }
                            });
                        }
                    });
                });

                ui.separator();

                // --- 2. Execution Controls ---
                let run_label = if app.session.is_running {
                    "⏸ Pause (F5)"
                } else {
                    "▶ Run (F5)"
                };
                let run_button = egui::Button::new(run_label).fill(if app.session.is_running {
                    egui::Color32::from_rgb(180, 140, 20)
                } else {
                    egui::Color32::from_rgb(34, 139, 34)
                });
                if ui.add(run_button).clicked() {
                    app.disassembly_view_addr = None;
                    app.session.toggle_run();
                }

                if ui.button("⏭ Step Inst (F10)").clicked() {
                    app.disassembly_view_addr = None;
                    app.session.step_instruction();
                }

                if ui.button("⏯ Step CCK (F11)").clicked() {
                    app.disassembly_view_addr = None;
                    app.session.step_cck();
                }

                if ui.button("⏮ Rewind (Shift+F10)").clicked() {
                    app.disassembly_view_addr = None;
                    app.session.step_backward();
                }

                ui.separator();

                // --- 3. Machine Status Badge ---
                let (status_text, bg_color) = if app.session.machine.cpu.state.halted {
                    ("HALTED", egui::Color32::from_rgb(180, 40, 40))
                } else if app.session.machine.cpu.state.stopped {
                    ("STOPPED", egui::Color32::from_rgb(40, 90, 180))
                } else if app.session.is_running {
                    ("RUNNING", egui::Color32::from_rgb(40, 160, 60))
                } else {
                    ("PAUSED", egui::Color32::from_rgb(160, 130, 30))
                };

                ui.colored_label(bg_color, format!(" ● [{}] ", status_text));

                if let Some((ref msg, ref mut frames)) = app.toast_message {
                    ui.colored_label(egui::Color32::from_rgb(0, 220, 255), format!(" 💾 {msg} "));
                    if *frames > 0 {
                        *frames -= 1;
                    }
                }
                if let Some((_, frames)) = app.toast_message {
                    if frames == 0 {
                        app.toast_message = None;
                    }
                }

                ui.separator();

                // --- 4. Telemetry Metrics ---
                ui.label(format!("CCK: {}", app.session.debugger.current_cck));
                ui.label(format!("Inst: {}", app.session.instructions_executed));

                ui.separator();

                // --- 5. Theme & Zoom Preferences ---
                egui::ComboBox::from_id_salt("theme_selector")
                    .selected_text(match app.theme {
                        AppTheme::Dark => "Theme: Dark",
                        AppTheme::Light => "Theme: Light",
                        AppTheme::ClassicWorkbench => "Theme: Workbench",
                    })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut app.theme, AppTheme::Dark, "Dark")
                            .clicked()
                        {
                            app.theme.apply(ctx);
                        }
                        if ui
                            .selectable_value(&mut app.theme, AppTheme::Light, "Light")
                            .clicked()
                        {
                            app.theme.apply(ctx);
                        }
                        if ui
                            .selectable_value(
                                &mut app.theme,
                                AppTheme::ClassicWorkbench,
                                "Workbench",
                            )
                            .clicked()
                        {
                            app.theme.apply(ctx);
                        }
                    });

                let zoom = ctx.zoom_factor();
                egui::ComboBox::from_id_salt("zoom_selector")
                    .selected_text(format!("{:.0}%", zoom * 100.0))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label((zoom - 1.0).abs() < 0.01, "100%")
                            .clicked()
                        {
                            ctx.set_zoom_factor(1.0);
                        }
                        if ui
                            .selectable_label((zoom - 1.25).abs() < 0.01, "125%")
                            .clicked()
                        {
                            ctx.set_zoom_factor(1.25);
                        }
                        if ui
                            .selectable_label((zoom - 1.5).abs() < 0.01, "150%")
                            .clicked()
                        {
                            ctx.set_zoom_factor(1.5);
                        }
                        if ui
                            .selectable_label((zoom - 2.0).abs() < 0.01, "200%")
                            .clicked()
                        {
                            ctx.set_zoom_factor(2.0);
                        }
                    });

                ui.separator();

                // --- 6. View Mode Toggle ---
                if ui
                    .button("🎮 Game View (F2 / F12)")
                    .on_hover_text("Toggle Screen-Only Game View (F2, F12, or `)")
                    .clicked()
                {
                    app.view_mode = crate::app::ViewMode::ScreenOnly;
                }
            });
        });
}

/// Saves snapshot to an in-memory quick-save slot (1..=5)
pub fn quick_save_slot(app: &mut EmulatorApp, slot: usize) {
    if let Ok(()) = app.session.save_quick_slot(slot) {
        app.toast_message = Some((format!("Quick-saved to Slot #{slot}"), 180));
    }
}

/// Restores snapshot from an in-memory quick-save slot (1..=5)
pub fn quick_load_slot(app: &mut EmulatorApp, slot: usize) {
    if let Ok(()) = app.session.load_quick_slot(slot) {
        app.disassembly_view_addr = None;
        app.toast_message = Some((format!("Restored from Slot #{slot}"), 180));
    }
}

/// Opens native file chooser dialog to save an A500 state file
pub fn open_save_state_dialog(app: &mut EmulatorApp) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let file = rfd::FileDialog::new()
            .set_title("Save Amiga 500 State")
            .add_filter("Amiga Save State (*.a500z, *.json)", &["a500z", "json"])
            .save_file();

        if let Some(path) = file {
            if let Ok(()) = app.session.save_state_to_file(&path, false) {
                app.toast_message = Some((format!("State saved to {}", path.display()), 180));
            }
        }
    }
}

/// Opens native file chooser dialog to load an A500 state file
pub fn open_load_state_dialog(app: &mut EmulatorApp) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let file = rfd::FileDialog::new()
            .set_title("Load Amiga 500 State")
            .add_filter("Amiga Save State (*.a500z, *.json)", &["a500z", "json"])
            .pick_file();

        if let Some(path) = file {
            if let Ok(()) = app.session.load_state_from_file(&path) {
                app.disassembly_view_addr = None;
                app.toast_message = Some((format!("State loaded from {}", path.display()), 180));
            }
        }
    }
}
