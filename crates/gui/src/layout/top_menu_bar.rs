//! Top Menu Bar Component
//!
//! File management, execution controls, status badges, and visual preferences.

use crate::app::EmulatorApp;
use crate::app_host::HostEffect;
use crate::file_actions::FileCompletion;
use crate::host_commands::HostCommand;
use crate::keyboard_input::KeyCommand;
use crate::theme::AppTheme;

const ZOOM_SELECTION_TOLERANCE: f32 = 0.01;

pub(crate) const TOAST_DURATION_FRAMES: usize = 180;

pub(crate) fn render_top_menu_bar(app: &mut EmulatorApp, ui: &mut egui::Ui) {
    let context = ui.ctx().clone();
    let ctx = &context;
    let theme = app.theme;
    let developer = app.view_mode == crate::ViewMode::Developer;
    egui::Panel::top("top_menu_bar")
        .frame(egui::Frame::menu(ui.style()))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = true;

                // --- 1. File Menu ---
                ui.menu_button("File", |ui| {
                    if ui.button("Load ROM from File...")
                        .on_hover_text("Install a 256 KiB Kickstart ROM and reset the machine to its boot vectors. Press Run after loading.")
                        .clicked() {
                        app.open_load_rom_dialog();
                        ui.close_kind(egui::UiKind::Menu);
                    }

                    crate::layout::file_dialogs::render_recent_files(app, ui);

                    ui.separator();

                    if ui.button("🔄 Reset").clicked() {
                        app.host.queue(HostCommand::Reset);
                        app.inspection.ready = false;
                        ui.close_kind(egui::UiKind::Menu);
                    }

                    if ui.button("⚡ Reset Warm").clicked() {
                        app.host.queue(HostCommand::ResetWarm);
                        app.inspection.ready = false;
                        ui.close_kind(egui::UiKind::Menu);
                    }
                });

                // --- 2. State Menu ---
                ui.menu_button("State", |ui| {
                    if app.shortcut_tooltip(
                        ui.button("💾 Save State to File..."), KeyCommand::SaveState,
                        "Export a snapshot of the current machine.",
                    ).clicked() {
                        app.open_save_state_dialog();
                        ui.close_kind(egui::UiKind::Menu);
                    }

                    if app.shortcut_tooltip(
                        ui.button("📂 Load State from File..."), KeyCommand::LoadState,
                        "Choose saved progress or import a machine snapshot.",
                    ).clicked() {
                        app.open_load_state_dialog();
                        ui.close_kind(egui::UiKind::Menu);
                    }

                    ui.separator();

                    if app.shortcut_tooltip(
                        ui.button("⚡ Quick Save Slot 1"), KeyCommand::QuickSave,
                        "Save current progress to quick slot 1.",
                    ).clicked() {
                        quick_save_slot(app, 1);
                        ui.close_kind(egui::UiKind::Menu);
                    }

                    let slot1_available = quick_slot_available(app, 1);
                    let slot1_label = if slot1_available {
                        "Quick Load Slot 1"
                    } else {
                        "Quick Load Slot 1 (Empty)"
                    };
                    if app.shortcut_tooltip(
                        ui.add_enabled(slot1_available, egui::Button::new(slot1_label)),
                        KeyCommand::QuickLoad, "Restore the state stored in quick slot 1.",
                    ).on_disabled_hover_text("Quick slot 1 is empty.").clicked() {
                        quick_load_slot(app, 1);
                        ui.close_kind(egui::UiKind::Menu);
                    }

                    ui.separator();

                    ui.menu_button("Slots 2–5", |ui| {
                        for slot in 2..=debugger::session::QUICK_SLOT_COUNT {
                            ui.horizontal(|ui| {
                                if ui.button(format!("Save #{slot}")).clicked() {
                                    quick_save_slot(app, slot);
                                }
                                let loaded = quick_slot_available(app, slot);
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
                let run_label = if app.host.status.pause_pending {
                    "Pausing..."
                } else if app.host.status.running {
                    if developer { "⏸ Pause" } else { "⏹ Stop" }
                } else {
                    "▶ Run"
                };
                let run_button = egui::Button::new(run_label).fill(if app.host.status.running {
                    egui::Color32::from_rgb(180, 140, 20)
                } else {
                    egui::Color32::from_rgb(34, 139, 34)
                });
                if app.shortcut_tooltip(
                    ui.add_enabled(!app.host.status.pause_pending, run_button), KeyCommand::Pause,
                    app.execution_help(),
                ).on_disabled_hover_text("Waiting for the machine owner to finish pausing.")
                    .clicked() {
                    app.apply_key_command(KeyCommand::Pause, ctx);
                }

                if developer {
                    let can_step = !app.host.status.running && app.host.can_edit();
                    for (label, action, description) in [
                        ("⏭ Step Inst", KeyCommand::StepInstruction, "Execute the current CPU instruction, even at a breakpoint. The image may be partway through a field."),
                        ("▶ Step CCK", KeyCommand::StepCck, "Advance the entire machine by one Color Clock. The image may be partway through a field."),
                        ("Step Frame", KeyCommand::StepField, "Advance to the next completed PAL/NTSC video field and pause, including while the CPU is stopped. Breakpoints and CPU halts take precedence."),
                        ("⏮ Rewind", KeyCommand::HistoryBack, "Go back one instruction in the recorded history."),
                    ] {
                        if app.shortcut_tooltip(
                            ui.add_enabled(can_step, egui::Button::new(label)), action, description,
                        ).on_disabled_hover_text("Pause the machine before using this control.").clicked() {
                            app.apply_key_command(action, ctx);
                        }
                    }
                }

                ui.separator();

                if ui.add_enabled(app.host.status.speed > crate::playback::MIN_SPEED, egui::Button::new("Slower"))
                    .on_hover_text("Halve the requested emulation speed, down to 0.25x.")
                    .clicked() {
                    app.host.queue(HostCommand::Slower);
                }
                ui.label(format!("{:.2}x", app.host.status.speed))
                    .on_hover_text("Requested speed relative to the configured PAL/NTSC color clock.");
                if ui.add_enabled(app.host.status.speed < crate::playback::MAX_SPEED, egui::Button::new("Faster"))
                    .on_hover_text("Double the requested emulation speed, up to 4x. Actual speed depends on host capacity; presentation targets 50 Hz PAL or 60 Hz NTSC using the latest completed field.")
                    .clicked() {
                    app.host.queue(HostCommand::Faster);
                }
                if ui.button("1x").on_hover_text("Restore normal PAL/NTSC emulation speed.").clicked() {
                    app.host.queue(HostCommand::NormalSpeed);
                }
                ui.checkbox(&mut app.show_fps, "FPS")
                    .on_hover_text("Show or hide the measured emulated video fields per second.");
                if app.show_fps {
                    ui.label(format!("FPS: {:.1}", app.host.status.fps))
                    .on_hover_text("Completed emulated video fields per host second, sampled over one second. This is not the GUI repaint rate. Paused execution shows zero.");
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = true;

                if developer && (!app.host.continuous_requested() || (!app.inspection.ready && !app.host.status.pause_pending)) {
                    if let Some(data) = &app.host.data { app.inspection.refresh_status(data); }
                }
                // --- 3. Machine Status Badge ---
                let (status_text, bg_color) = if if developer { app.inspection.halted } else { app.host.status.halted } {
                    ("HALTED", egui::Color32::from_rgb(180, 40, 40))
                } else if developer && app.inspection.stopped {
                    ("STOPPED", egui::Color32::from_rgb(40, 90, 180))
                } else if app.host.status.running {
                    ("RUNNING", egui::Color32::from_rgb(40, 160, 60))
                } else {
                    ("PAUSED", egui::Color32::from_rgb(160, 130, 30))
                };

                ui.colored_label(bg_color, format!(" • [{}] ", status_text))
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
                        if app.shortcut_tooltip(
                            ui.selectable_label((zoom - 1.0).abs() < ZOOM_SELECTION_TOLERANCE, "100%"),
                            KeyCommand::NormalZoom, "Restore normal interface size.",
                        ).clicked()
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
                                egui::Popup::close_all(ui.ctx());
                            }
                        }
                    })
                    .response
                    .on_hover_text("Fit smoothly fills the available area while preserving proportions. Fixed scales (0.5x, 1x, 2x, 4x) use sharp pixels, independently of interface zoom and monitor DPI. Oversized images stay centered and are clipped.");

                ui.separator();

                // --- 6. View Mode Toggle ---
                if app.shortcut_tooltip(
                    ui.button(if developer { "🎮 Game View" } else { "Debug View" }),
                    KeyCommand::ToggleView,
                    "Switch views and release held gameplay controls. Returning to Debug pauses at the next completed field. The middle mouse button frees only the mouse.",
                )
                    .clicked()
                {
                    app.set_view_mode(if developer { crate::ViewMode::ScreenOnly } else { crate::ViewMode::Developer });
                }

                ui.separator();
                ui.add(egui::Label::new(format!("ROM: {}", app.loaded_rom_name.as_deref().unwrap_or("not loaded"))).truncate())
                    .on_hover_text(app.loaded_rom_name.as_deref().unwrap_or("Use File > Load ROM from File to install Kickstart firmware."));
            });
            ui.horizontal_wrapped(|ui| {
        ui.small(if app.player.captured {
            "Mouse captured. Middle mouse frees it. F12 switches Game/Debug."
        } else {
            "Mouse free. Click the Amiga display to capture it. F12 switches Game/Debug."
        });
        if let Some(configuration) = &app.player.active_configuration {
            if !configuration.controls.host.frontend.release.is_empty() {
                ui.small(format!("Mouse release key: {}", crate::host_keys::frontend_binding_name(&configuration.controls.host.frontend.release)));
            }
        }
            });
        });
    if app.theme != theme {
        ui.set_style(ctx.global_style());
    }
}

/// Saves snapshot to an in-memory quick-save slot (1..=5)
pub(crate) fn quick_save_slot(app: &mut EmulatorApp, slot: usize) {
    #[cfg(not(target_arch = "wasm32"))]
    if app.player.active_game.is_some() {
        app.save_player(Some(slot));
        return;
    }
    app.host.queue_effect(
        HostCommand::SaveQuickSlot(slot),
        HostEffect::File(FileCompletion::QuickSave(slot)),
    );
}

/// Restores snapshot from an in-memory quick-save slot (1..=5)
pub(crate) fn quick_load_slot(app: &mut EmulatorApp, slot: usize) {
    if let Err(error) = app.validate_current_input() {
        app.file_error = Some(error);
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    if app.player.active_game.is_some() {
        app.load_player_quick(slot);
        return;
    }
    app.host.queue_effect(
        HostCommand::LoadQuickSlot {
            slot,
            ports: app.current_control_ports(),
        },
        HostEffect::File(FileCompletion::State {
            message: format!("Restored from Slot #{slot}"),
            verified_rom: false,
        }),
    );
}

fn quick_slot_available(app: &EmulatorApp, slot: usize) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    if app.player.active_game.is_some() {
        return app.player.game_slots[slot - 1];
    }
    app.host.status.quick_slots[slot - 1]
}
