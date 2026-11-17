#![allow(clippy::unwrap_used, clippy::panic)]

mod common;
use egui::{Color32, Context, RawInput};
use gui::{host_commands::HostCommand, AppTheme, EmulatorApp, ViewMode};

fn pending_step_session() -> gui::DebuggerSession {
    let mut session = gui::DebuggerSession::new();
    use physical_memory::{AddressBus, BusResult};
    let m = &mut session.machine;
    for (address, value) in [(0xbfd100, 0xff), (0xbfd300, 0xff), (0xbfd100, 0xf5)] {
        loop {
            if matches!(
                m.memory_bus().write_byte(address, value),
                BusResult::Ready(())
            ) {
                break;
            }
            m.step_cck();
        }
        m.step_cck();
    }
    m.step_cycles(2);
    m.paula.disk_activity.sequences[0] = u64::MAX;
    loop {
        if matches!(
            m.memory_bus().write_byte(0xbfd100, 0xf4),
            BusResult::Ready(())
        ) {
            break;
        }
        m.step_cck();
    }
    session
}

fn frame(app: &mut EmulatorApp, ctx: &Context, time: f64) -> egui::FullOutput {
    ctx.run(
        RawInput {
            time: Some(time),
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            ..Default::default()
        },
        |ctx| app.update_ui(ctx),
    )
}

fn indicator(output: &egui::FullOutput, color: Color32) -> Option<egui::Rect> {
    output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::epaint::Shape::Circle(circle) if circle.fill == color => {
                Some(circle.visual_bounding_rect())
            }
            _ => None,
        })
}

#[test]
fn head_step_lights_fixed_indicator_for_120_ms_and_restore_clears_it() {
    let ctx = Context::default();
    let mut app = EmulatorApp::default();
    app.view_mode = ViewMode::ScreenOnly;
    let session = pending_step_session();
    let json = session.save_state_to_json().unwrap();
    common::command(
        &mut app,
        HostCommand::RestorePlayer {
            json: json.clone(),
            game_id: None,
            ports: [
                debugger::launch::DeviceKind::Mouse,
                debugger::launch::DeviceKind::Joystick,
            ],
            resume: false,
        },
    );
    let tokens = AppTheme::Dark.tokens();
    let idle = frame(&mut app, &ctx, 1.0);
    let bounds = indicator(&idle, tokens.border_subtle).unwrap();
    assert!(indicator(&idle, tokens.accent_success).is_none());

    common::command(&mut app, HostCommand::StepCck);
    assert_eq!(app.host.status.drives[0].activity_sequence, 0);
    let active = frame(&mut app, &ctx, 1.01);
    assert_eq!(indicator(&active, tokens.accent_success), Some(bounds));
    assert!(indicator(&frame(&mut app, &ctx, 1.129), tokens.accent_success).is_some());
    assert!(indicator(&frame(&mut app, &ctx, 1.131), tokens.accent_success).is_none());

    common::command(
        &mut app,
        HostCommand::RestorePlayer {
            json: json.clone(),
            game_id: None,
            ports: [
                debugger::launch::DeviceKind::Mouse,
                debugger::launch::DeviceKind::Joystick,
            ],
            resume: false,
        },
    );
    let restored = frame(&mut app, &ctx, 1.14);
    assert!(indicator(&restored, tokens.accent_success).is_none());
    assert_eq!(indicator(&restored, tokens.border_subtle), Some(bounds));

    common::command(&mut app, HostCommand::StepCck);
    assert!(indicator(&frame(&mut app, &ctx, 1.15), tokens.accent_success).is_some());
    common::command(
        &mut app,
        HostCommand::RestorePlayer {
            json: json,
            game_id: None,
            ports: [
                debugger::launch::DeviceKind::Mouse,
                debugger::launch::DeviceKind::Joystick,
            ],
            resume: false,
        },
    );
    // Restore while the light is still on must cancel its remaining hold time.
    assert!(indicator(&frame(&mut app, &ctx, 1.16), tokens.accent_success).is_none());
}

#[test]
fn guest_and_stepped_keyboard_resets_cancel_the_light_without_a_host_generation_change() {
    for keyboard_reset in [false, true] {
        let mut session = pending_step_session();
        session
            .load_binary(0x1000, &[0x4e, 0x71, 0x4e, 0x70, 0x60, 0xfe], true)
            .unwrap();
        // A delivered keyboard reset is captured after the initial head step.
        session.machine.keyboard.pins.reset_line_asserted = keyboard_reset;
        session.machine.keyboard_reset_asserted = false;
        let mut app = EmulatorApp::with_session(Box::new(session));
        app.view_mode = ViewMode::ScreenOnly;
        let ctx = Context::default();
        common::settle(&mut app, &ctx);
        frame(&mut app, &ctx, 1.0);
        let generation = app.host.status.generation;
        let reset_sequence = app.host.status.drive_reset_sequence;
        common::command(&mut app, HostCommand::StepCck);
        assert!(indicator(
            &frame(&mut app, &ctx, 1.01),
            AppTheme::Dark.tokens().accent_success
        )
        .is_some());
        for _ in 0..3 {
            common::command(
                &mut app,
                if keyboard_reset {
                    HostCommand::StepCck
                } else {
                    HostCommand::StepInstruction
                },
            );
            if app.host.status.drive_reset_sequence != reset_sequence {
                break;
            }
        }
        assert_ne!(
            app.host.status.drive_reset_sequence, reset_sequence,
            "reset source: keyboard={keyboard_reset}"
        );
        assert_eq!(app.host.status.generation, generation);
        assert!(indicator(
            &frame(&mut app, &ctx, 1.02),
            AppTheme::Dark.tokens().accent_success
        )
        .is_none());
    }
}

#[test]
fn restored_running_selection_is_sustained_and_deselection_starts_the_hold() {
    use physical_memory::{AddressBus, BusResult};
    let mut session = gui::DebuggerSession::new();
    let m = &mut session.machine;
    for (address, value) in [
        (0xbfd100, 0xff),
        (0xbfd300, 0xff),
        (0xbfd100, 0x7f),
        (0xbfd100, 0x77),
    ] {
        loop {
            if matches!(
                m.memory_bus().write_byte(address, value),
                BusResult::Ready(())
            ) {
                break;
            }
            m.step_cck();
        }
        m.step_cck();
    }
    m.step_cycles(2);
    assert!(m.floppy.drives[0].selected && m.floppy.drives[0].motor_on);
    let json = session.save_state_to_json().unwrap();
    let m = &mut session.machine;
    // Leave a real CIA capture pending for the first explicit owner step.
    loop {
        if matches!(
            m.memory_bus().write_byte(0xbfd100, 0x7f),
            BusResult::Ready(())
        ) {
            break;
        }
        m.step_cck();
    }
    let mut app = EmulatorApp::with_session(Box::new(session));
    app.view_mode = ViewMode::ScreenOnly;
    let ctx = Context::default();
    common::settle(&mut app, &ctx);
    let tokens = AppTheme::Dark.tokens();
    let active = frame(&mut app, &ctx, 1.0);
    let bounds = indicator(&active, tokens.accent_success).unwrap();
    assert_eq!(
        indicator(&frame(&mut app, &ctx, 10.0), tokens.accent_success),
        Some(bounds)
    );
    common::command(&mut app, HostCommand::StepCck);
    common::command(&mut app, HostCommand::StepCck);
    assert!(!app.host.status.drives[0].sustained_activity);
    assert!(indicator(&frame(&mut app, &ctx, 10.01), tokens.accent_success).is_some());
    assert!(indicator(&frame(&mut app, &ctx, 10.129), tokens.accent_success).is_some());
    assert_eq!(
        indicator(&frame(&mut app, &ctx, 10.131), tokens.border_subtle),
        Some(bounds)
    );
    common::command(
        &mut app,
        HostCommand::RestorePlayer {
            json,
            game_id: None,
            ports: [
                debugger::launch::DeviceKind::Mouse,
                debugger::launch::DeviceKind::Joystick,
            ],
            resume: false,
        },
    );
    assert!(app.host.status.drives[0].sustained_activity);
    assert_eq!(
        indicator(&frame(&mut app, &ctx, 10.14), tokens.accent_success),
        Some(bounds)
    );
    common::command(&mut app, HostCommand::Reset);
    assert!(!app.host.status.drives[0].sustained_activity);
    assert!(indicator(&frame(&mut app, &ctx, 10.15), tokens.accent_success).is_none());
}
