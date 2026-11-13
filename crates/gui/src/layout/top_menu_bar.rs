//! Top Menu Bar Component
//!
//! File management, execution controls, status badges, and visual preferences.

use crate::app::EmulatorApp;
use crate::theme::AppTheme;

const ZOOM_SELECTION_TOLERANCE: f32 = 0.01;

pub(crate) const TOAST_DURATION_FRAMES: usize = 180;

pub(crate) fn render_top_menu_bar(app: &mut EmulatorApp, ctx: &egui::Context) {
    let developer = app.view_mode == crate::ViewMode::Developer;
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

                    if ui.button("Load ROM from File...")
                        .on_hover_text("Install a 256 KiB Kickstart ROM and reset the machine to its boot vectors. Press Run after loading.")
                        .clicked() {
                        app.open_load_rom_dialog();
                        ui.close_menu();
                    }

                    crate::layout::file_dialogs::render_recent_files(app, ui);

                    ui.separator();

                    if ui.button("🔄 Reset (Ctrl+R)").clicked() {
                        app.session.reset();
                        app.inspection.ready = false;
                        ui.close_menu();
                    }

                    if ui.button("⚡ Reset Warm").clicked() {
                        app.session.reset_warm();
                        app.inspection.ready = false;
                        ui.close_menu();
                    }
                });

                // --- 2. State Menu ---
                ui.menu_button("State", |ui| {
                    if ui.button("💾 Save State to File... (Ctrl+S)").clicked() {
                        app.open_save_state_dialog();
                        ui.close_menu();
                    }

                    if ui.button("📂 Load State from File... (Ctrl+L)").clicked() {
                        app.open_load_state_dialog();
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
                        for slot in 2..=debugger::session::QUICK_SLOT_COUNT {
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
                let run_label = if app.session.pause_pending() {
                    "Pausing..."
                } else if app.session.is_running {
                    if developer { "⏸ Pause (F5)" } else { "⏹ Stop (F5)" }
                } else {
                    "▶ Run (F5)"
                };
                let run_button = egui::Button::new(run_label).fill(if app.session.is_running {
                    egui::Color32::from_rgb(180, 140, 20)
                } else {
                    egui::Color32::from_rgb(34, 139, 34)
                });
                if ui.add_enabled(!app.session.pause_pending(), run_button)
                    .on_hover_text("Run at the selected PAL/NTSC speed, or pause after the current video field completes (F5).")
                    .clicked() {
                    app.session.toggle_run();
                }

                if developer {
                    if ui.add_enabled(!app.session.is_running, egui::Button::new("⏭ Step Inst (F10)"))
                        .on_hover_text("Execute the current CPU instruction, even at a breakpoint. The image may be partway through a field.")
                        .clicked() {
                        app.disassembly_view_addr = None;
                        app.session.step_instruction();
                    }

                    if ui.add_enabled(!app.session.is_running, egui::Button::new("⏯ Step CCK (F11)"))
                        .on_hover_text("Advance the entire machine by one Color Clock. The image may be partway through a field.")
                        .clicked() {
                        app.disassembly_view_addr = None;
                        app.session.step_cck();
                    }

                    if ui.add_enabled(!app.session.is_running, egui::Button::new("Step Frame (Shift+F11)"))
                        .on_hover_text("Advance to the next completed PAL/NTSC video field and pause, including while the CPU is stopped. Breakpoints and CPU halts take precedence.")
                        .clicked() {
                        app.disassembly_view_addr = None;
                        app.session.step_frame();
                    }

                    if ui.add_enabled(!app.session.is_running, egui::Button::new("⏮ Rewind (Shift+F10)")).clicked() {
                        app.disassembly_view_addr = None;
                        app.session.step_backward();
                    }
                }

                ui.separator();

                if ui.add_enabled(app.playback.can_slow_down(), egui::Button::new("Slower"))
                    .on_hover_text("Halve the requested emulation speed, down to 0.25x.")
                    .clicked() {
                    app.playback.slower();
                }
                ui.label(format!("{:.2}x", app.playback.speed()))
                    .on_hover_text("Requested speed relative to the configured PAL/NTSC color clock.");
                if ui.add_enabled(app.playback.can_speed_up(), egui::Button::new("Faster"))
                    .on_hover_text("Double the requested emulation speed, up to 4x. Actual speed depends on host capacity; presentation targets 50 Hz PAL or 60 Hz NTSC using the latest completed field.")
                    .clicked() {
                    app.playback.faster();
                }
                if ui.button("1x").on_hover_text("Restore normal PAL/NTSC emulation speed.").clicked() {
                    app.playback.normal_speed();
                }
                ui.checkbox(&mut app.show_fps, "FPS")
                    .on_hover_text("Show or hide the measured emulated video fields per second.");
                if app.show_fps {
                    ui.label(format!("FPS: {:.1}", app.playback.fps()))
                    .on_hover_text("Completed emulated video fields per host second, sampled over one second. This is not the GUI repaint rate. Paused execution shows zero.");
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = true;

                if developer && (!app.session.continuous_run() || (!app.inspection.ready && !app.session.pause_pending())) {
                    app.inspection.refresh_status(&app.session);
                }
                // --- 3. Machine Status Badge ---
                let (status_text, bg_color) = if if developer { app.inspection.halted } else { app.session.machine.cpu.state.is_halted() } {
                    ("HALTED", egui::Color32::from_rgb(180, 40, 40))
                } else if developer && app.inspection.stopped {
                    ("STOPPED", egui::Color32::from_rgb(40, 90, 180))
                } else if app.session.is_running {
                    ("RUNNING", egui::Color32::from_rgb(40, 160, 60))
                } else {
                    ("PAUSED", egui::Color32::from_rgb(160, 130, 30))
                };

                ui.colored_label(bg_color, format!(" ● [{}] ", status_text))
                    .on_hover_text("STOPPED means the CPU is waiting for an interrupt. During Run, chip clocks and video continue; Pause suspends the machine. HALTED ends Run.");

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
                if developer {
                    ui.label(&app.inspection.cck_label);
                    ui.label(&app.inspection.instruction_label);
                }

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
                            .selectable_label((zoom - 1.0).abs() < ZOOM_SELECTION_TOLERANCE, "100%")
                            .clicked()
                        {
                            ctx.set_zoom_factor(1.0);
                        }
                        if ui
                            .selectable_label(
                                (zoom - 1.25).abs() < ZOOM_SELECTION_TOLERANCE,
                                "125%",
                            )
                            .clicked()
                        {
                            ctx.set_zoom_factor(1.25);
                        }
                        if ui
                            .selectable_label((zoom - 1.5).abs() < ZOOM_SELECTION_TOLERANCE, "150%")
                            .clicked()
                        {
                            ctx.set_zoom_factor(1.5);
                        }
                        if ui
                            .selectable_label((zoom - 2.0).abs() < ZOOM_SELECTION_TOLERANCE, "200%")
                            .clicked()
                        {
                            ctx.set_zoom_factor(2.0);
                        }
                    });

                ui.separator();

                egui::ComboBox::from_label("Image scale")
                    .selected_text(app.screen_scale.label())
                    .show_ui(ui, |ui| {
                        for scale in crate::ScreenScale::ALL {
                            if ui.selectable_value(&mut app.screen_scale, scale, scale.label()).clicked() {
                                ui.memory_mut(|memory| memory.close_popup());
                            }
                        }
                    })
                    .response
                    .on_hover_text("Fit smoothly fills the available area while preserving proportions. Fixed scales (0.5x, 1x, 2x, 4x) use sharp pixels, independently of interface zoom and monitor DPI. Oversized images stay centered and are clipped.");

                ui.separator();

                // --- 6. View Mode Toggle ---
                if ui
                    .button(if developer { "🎮 Game View (F2 / F12)" } else { "Debug View (F2 / F12)" })
                    .on_hover_text("Switch between Game and Debug views. Returning to Debug requests a pause at the next completed field. F2/F12 toggle; Escape returns to Debug.")
                    .clicked()
                {
                    app.set_view_mode(if developer { crate::ViewMode::ScreenOnly } else { crate::ViewMode::Developer });
                }

                ui.separator();
                ui.add(egui::Label::new(format!("ROM: {}", app.loaded_rom_name.as_deref().unwrap_or("not loaded"))).truncate())
                    .on_hover_text(app.loaded_rom_name.as_deref().unwrap_or("Use File > Load ROM from File to install Kickstart firmware."));
            });
        });
}

/// Saves snapshot to an in-memory quick-save slot (1..=5)
pub(crate) fn quick_save_slot(app: &mut EmulatorApp, slot: usize) {
    if let Ok(()) = app.session.save_quick_slot(slot) {
        app.toast_message = Some((
            format!("Quick-saved to Slot #{slot}"),
            TOAST_DURATION_FRAMES,
        ));
    }
}

/// Restores snapshot from an in-memory quick-save slot (1..=5)
pub(crate) fn quick_load_slot(app: &mut EmulatorApp, slot: usize) {
    if let Ok(()) = app.session.load_quick_slot(slot) {
        app.state_loaded(format!("Restored from Slot #{slot}"));
    }
}
