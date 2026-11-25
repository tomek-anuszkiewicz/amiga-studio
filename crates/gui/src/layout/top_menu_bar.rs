//! Machine controls and presentation preferences shared by the session views.

use crate::app::EmulatorApp;
use crate::app_host::HostEffect;
use crate::file_actions::FileCompletion;
use crate::host_commands::HostCommand;
use crate::keyboard_input::KeyCommand;
use crate::player::PlayerScreen;
use crate::theme::AppTheme;
use crate::ViewMode;

const ZOOM_SELECTION_TOLERANCE: f32 = 0.01;

pub(crate) const TOAST_DURATION_FRAMES: usize = 180;

pub(crate) fn render_top_menu_bar(app: &mut EmulatorApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let theme = app.theme;
    let developer = app.view_mode == ViewMode::Developer;
    if developer
        && (!app.host.continuous_requested()
            || (!app.inspection.ready && !app.host.status.pause_pending))
    {
        if let Some(data) = &app.host.data {
            app.inspection.refresh_status(data);
        }
    }

    egui::Panel::top("top_menu_bar")
        .frame(egui::Frame::menu(ui.style()))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = true;
                render_state_menu(app, ui);
                if developer {
                    render_debugger_execution(app, ui);
                    if app.shortcut_tooltip(
                        ui.button("🎮 Game View"),
                        KeyCommand::ToggleView,
                        "Show the Game view of this machine and release held gameplay controls.",
                    ).clicked() {
                        app.set_view_mode(ViewMode::ScreenOnly);
                    }
                } else if ui.button("Manual setup")
                    .on_hover_text("Pause this session and open setup. Return to session opens the same machine.")
                    .clicked()
                {
                    app.navigate_player(PlayerScreen::Setup);
                }

                render_machine_status(app, ui, developer);
                if developer {
                    ui.label(&app.inspection.cck_label);
                    ui.label(&app.inspection.instruction_label);
                }
                render_speed(app, ui);
                let fps = if app.host.status.running { app.host.status.fps } else { 0.0 };
                ui.label(format!("FPS: {fps:.1}"))
                    .on_hover_text("Completed emulated video fields per host second, sampled over one second. Paused execution shows zero.");
                render_presentation_preferences(app, ui, &ctx);
                ui.small(if app.player.captured {
                    "Mouse captured. Middle mouse frees it."
                } else {
                    "Mouse free. Click the Amiga display to capture it."
                });
                if let Some(configuration) = &app.player.active_configuration {
                    let release = &configuration.controls.host.frontend.release;
                    if cfg!(not(target_arch = "wasm32"))
                        && app.player.captured
                        && crate::host_keys::frontend_key_position(release) == Some("Escape")
                    {
                        ui.small("Saved Escape release shortcut is inactive while captured.");
                    } else if !release.is_empty() {
                        ui.small(format!("Mouse release key: {}", crate::host_keys::frontend_binding_name(release)));
                    }
                }
            });
        });
    if app.theme != theme {
        ui.set_style(ctx.global_style());
    }
}

fn render_state_menu(app: &mut EmulatorApp, ui: &mut egui::Ui) {
    ui.menu_button("State", |ui| {
        if app
            .shortcut_tooltip(
                ui.button("💾 Save State to File..."),
                KeyCommand::SaveState,
                "Export a snapshot of the current machine.",
            )
            .clicked()
        {
            app.open_save_state_dialog();
            ui.close_kind(egui::UiKind::Menu);
        }

        if app
            .shortcut_tooltip(
                ui.button("📂 Load State from File..."),
                KeyCommand::LoadState,
                "Choose saved progress or import a machine snapshot.",
            )
            .clicked()
        {
            app.open_load_state_dialog();
            ui.close_kind(egui::UiKind::Menu);
        }

        ui.separator();

        if app
            .shortcut_tooltip(
                ui.button("⚡ Quick Save Slot 1"),
                KeyCommand::QuickSave,
                "Save current progress to quick slot 1.",
            )
            .clicked()
        {
            quick_save_slot(app, 1);
            ui.close_kind(egui::UiKind::Menu);
        }

        let slot1_available = quick_slot_available(app, 1);
        let slot1_label = if slot1_available {
            "Quick Load Slot 1"
        } else {
            "Quick Load Slot 1 (Empty)"
        };
        if app
            .shortcut_tooltip(
                ui.add_enabled(slot1_available, egui::Button::new(slot1_label)),
                KeyCommand::QuickLoad,
                "Restore the state stored in quick slot 1.",
            )
            .on_disabled_hover_text("Quick slot 1 is empty.")
            .clicked()
        {
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
}

fn render_debugger_execution(app: &mut EmulatorApp, ui: &mut egui::Ui) {
    let run_label = if app.host.status.pause_pending {
        "Pausing..."
    } else if app.host.status.running {
        "⏸ Pause"
    } else {
        "▶ Run"
    };
    let run_button = egui::Button::new(run_label).fill(if app.host.status.running {
        egui::Color32::from_rgb(180, 140, 20)
    } else {
        egui::Color32::from_rgb(34, 139, 34)
    });
    if app
        .shortcut_tooltip(
            ui.add_enabled(!app.host.status.pause_pending, run_button),
            KeyCommand::Pause,
            app.execution_help(),
        )
        .on_disabled_hover_text("Waiting for the machine owner to finish pausing.")
        .clicked()
    {
        app.apply_key_command(KeyCommand::Pause, ui.ctx());
    }

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
            app.apply_key_command(action, ui.ctx());
        }
    }
}

fn render_machine_status(app: &mut EmulatorApp, ui: &mut egui::Ui, developer: bool) {
    let halted = if developer {
        app.inspection.halted
    } else {
        app.host.status.halted
    };
    let tokens = app.theme.tokens();
    let (status_text, color) = if halted {
        ("HALTED", tokens.accent_error)
    } else if developer && app.inspection.stopped {
        ("STOPPED", tokens.accent_pc)
    } else if app.host.status.running {
        ("RUNNING", tokens.accent_success)
    } else {
        ("PAUSED", tokens.accent_warning)
    };
    let label = if developer {
        format!(" • [{status_text}] ")
    } else {
        status_text.to_owned()
    };
    ui.colored_label(color, label)
        .on_hover_text("STOPPED means the CPU is waiting for an interrupt. During Run, chip clocks and video continue. Pause suspends the machine. HALTED ends Run.");

    if let Some((message, frames)) = &mut app.toast_message {
        // A long path should wrap without changing how the controls are sized.
        ui.colored_label(tokens.accent_diff, message.as_str());
        *frames = frames.saturating_sub(1);
        if *frames == 0 {
            app.toast_message = None;
        }
    }
}

fn render_speed(app: &mut EmulatorApp, ui: &mut egui::Ui) {
    let speed = app.host.status.speed;
    let label = format!("Speed {speed:.2}x");
    // Reserve one widget's worth of space so the speed controls wrap together.
    let font = egui::TextStyle::Button.resolve(ui.style());
    let measure = |text: String| {
        ui.painter()
            .layout_no_wrap(text, font.clone(), egui::Color32::WHITE)
            .size()
            .x
    };
    let padding = ui.spacing().button_padding.x;
    let button_width = |text| (measure(text) + 2.0 * padding).max(ui.spacing().interact_size.x);
    let width = measure(label.clone())
        + button_width("−".into())
        + button_width("+".into())
        + 2.0 * ui.spacing().item_spacing.x;
    ui.allocate_ui_with_layout(
        egui::vec2(width, ui.spacing().interact_size.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            if ui.add_enabled(speed > crate::playback::MIN_SPEED, egui::Button::new("−"))
                .on_hover_text("Halve the requested emulation speed, down to 0.25x.")
                .on_disabled_hover_text("The minimum speed is 0.25x.")
                .clicked()
            {
                app.host.queue(HostCommand::Slower);
            }
            ui.label(label).on_hover_text("Requested speed relative to the configured PAL or NTSC color clock.");
            if ui.add_enabled(speed < crate::playback::MAX_SPEED, egui::Button::new("+"))
                .on_hover_text("Double the requested emulation speed, up to 4x. Actual speed depends on host capacity.")
                .on_disabled_hover_text("The maximum speed is 4x.")
                .clicked()
            {
                app.host.queue(HostCommand::Faster);
            }
        },
    );
}

fn render_presentation_preferences(app: &mut EmulatorApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    egui::ComboBox::from_id_salt("theme_selector")
        .selected_text(match app.theme {
            AppTheme::Dark => "Theme: Dark",
            AppTheme::Light => "Theme: Light",
            AppTheme::ClassicWorkbench => "Theme: Workbench",
        })
        .show_ui(ui, |ui| {
            for (theme, name) in [
                (AppTheme::Dark, "Dark"),
                (AppTheme::Light, "Light"),
                (AppTheme::ClassicWorkbench, "Workbench"),
            ] {
                if ui.selectable_value(&mut app.theme, theme, name).clicked() {
                    app.theme.apply(ctx);
                }
            }
        })
        .response
        .on_hover_text("Change the appearance of the emulator interface.");

    let zoom = ctx.zoom_factor();
    egui::ComboBox::from_id_salt("zoom_selector")
        .width(56.0)
        .selected_text(format!("{:.0}%", zoom * 100.0))
        .show_ui(ui, |ui| {
            for (factor, label) in [(1.0, "100%"), (1.25, "125%"), (1.5, "150%"), (2.0, "200%")] {
                let option =
                    ui.selectable_label((zoom - factor).abs() < ZOOM_SELECTION_TOLERANCE, label);
                let option = if factor == 1.0 {
                    app.shortcut_tooltip(
                        option,
                        KeyCommand::NormalZoom,
                        "Restore normal interface size.",
                    )
                } else {
                    option
                };
                if option.clicked() {
                    ctx.set_zoom_factor(factor);
                }
            }
        })
        .response
        .on_hover_text("Change the size of the emulator interface.");

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
        HostEffect::File(FileCompletion::State(format!("Restored from Slot #{slot}"))),
    );
}

fn quick_slot_available(app: &EmulatorApp, slot: usize) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    if app.player.active_game.is_some() {
        return app.player.game_slots[slot - 1];
    }
    app.host.status.quick_slots[slot - 1]
}
