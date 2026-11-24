#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use egui_kittest::kittest::NodeT;
use egui_kittest::{
    kittest::{by, Queryable},
    Harness,
};
use gui::host_commands::{HostCommand, HostRequest, HostValue};
use gui::{EmulatorApp, ViewMode};

#[test]
fn game_execution_does_not_collect_debugger_data() {
    let mut session = Box::new(gui::DebuggerSession::new());
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    session.temporal.set_recording(true);
    session.set_debugger_enabled(false);
    session.toggle_run();
    let mut owner = gui::host_owner::MachineOwner::new(session);
    let exchange = gui::host_video::VideoExchange::new(vec![0; owner.video_storage_len()]);
    assert!(owner.publish(&exchange).unwrap());
    for tick in 0..=20 {
        owner.advance(f64::from(tick) / 1000.0, || 0.0);
        if owner.status().field_number != 0 {
            break;
        }
    }
    assert_eq!(owner.status().field_number, 1);
    owner.execute(HostRequest {
        id: 1,
        generation: 1,
        command: HostCommand::Pause,
    });
    let reply = owner.execute(HostRequest {
        id: 2,
        generation: 1,
        command: HostCommand::Inspect,
    });
    let Ok(HostValue::Inspection(data)) = reply.value else {
        panic!("Missing paused owner inspection");
    };
    assert!(data.debugger.trace.entries().is_empty());
    assert!(data.temporal.is_empty());
    assert!(data.previous_cpu.is_none());
    assert_eq!(data.instructions_executed, 0);
    assert!(owner.status().quick_slots.iter().all(|occupied| !occupied));
}

#[test]
fn game_toolbar_supports_files_manual_states_speed_fps_and_return_to_debug() {
    let mut session = Box::new(gui::DebuggerSession::new());
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    let mut app = EmulatorApp::with_session(session);
    app.set_view_mode(ViewMode::ScreenOnly);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(
            |ctx, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ctx, app.host.status.generation);
                app.update_ui(ctx)
            },
            app,
        );
    common::settle_harness(&mut harness);
    for label in [
        "File",
        "State",
        "▶ Run (F5)",
        "Slower",
        "Faster",
        "1x",
        "FPS",
        "Debug View (F12)",
    ] {
        let bounds = harness
            .get_by_label(label)
            .accesskit_node()
            .bounding_box()
            .unwrap();
        assert!(bounds.x0 >= 0.0 && bounds.x1 <= 1024.0);
    }
    for label in [
        "Step Frame (Shift+F11)",
        "⏭ Step Inst (F10)",
        "⏯ Step CCK (F11)",
        "⏮ Rewind (Shift+F10)",
    ] {
        assert!(harness.query_by_label(label).is_none());
    }
    harness.get_by_label("File").click_accesskit();
    common::settle_harness(&mut harness);
    harness.get_by_label("Load ROM from File...");
    harness.get_by_label("📁 Load Binary... (Ctrl+O)");
    harness.get_by_label("File").click_accesskit();
    common::settle_harness(&mut harness);
    harness.get_by_label("FPS").click_accesskit();
    common::settle_harness(&mut harness);
    assert!(!harness.state().show_fps);
    assert!(harness.query_by_label("FPS: 0.0").is_none());
    harness.get_by_label("Faster").click_accesskit();
    common::settle_harness(&mut harness);
    assert_eq!(harness.state().host.status.speed, 2.0);
    harness.get_by_label("Slower").click_accesskit();
    common::settle_harness(&mut harness);
    assert_eq!(harness.state().host.status.speed, 1.0);
    harness.get_by_label("State").click_accesskit();
    common::settle_harness(&mut harness);
    harness.get_by_label("💾 Save State to File... (Ctrl+S)");
    harness.get_by_label("📂 Load State from File... (Ctrl+L)");
    harness
        .get_by_label("⚡ Quick Save Slot 1 (F6)")
        .click_accesskit();
    common::settle_harness(&mut harness);
    assert!(harness.state().host.status.quick_slots[0]);
    common::command(
        harness.state_mut(),
        HostCommand::SetRegister {
            register: gui::EditRegister::D(0),
            value: 123,
        },
    );
    harness.get_by_label("State").click_accesskit();
    common::settle_harness(&mut harness);
    harness
        .get_by_label("🔄 Quick Load Slot 1 (F9)")
        .click_accesskit();
    common::settle_harness(&mut harness);
    assert_eq!(common::inspect(harness.state_mut()).cpu.d_long(0), 0);
    assert_eq!(
        harness.state().loaded_rom_name.as_deref(),
        Some("from saved state")
    );
    harness.get_by_label("▶ Run (F5)").click_accesskit();
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    assert!(harness.state().host.status.running);
    harness.get_by_label("⏹ Stop (F5)").click_accesskit();
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    assert!(!harness.state().host.status.running);
    harness
        .query_all(by().role(egui::accesskit::Role::ComboBox))
        .next()
        .unwrap()
        .click_accesskit();
    common::settle_harness(&mut harness);
    harness.get_by_label("Light").click_accesskit();
    common::settle_harness(&mut harness);
    assert_eq!(harness.state().theme, gui::AppTheme::Light);
    harness
        .query_all(by().role(egui::accesskit::Role::ComboBox))
        .nth(1)
        .unwrap()
        .click_accesskit();
    common::settle_harness(&mut harness);
    harness.get_by_label("125%").click_accesskit();
    common::settle_harness(&mut harness);
    assert_eq!(harness.ctx.zoom_factor(), 1.25);
    harness.get_by_label("Debug View (F12)").click_accesskit();
    common::settle_harness(&mut harness);
    assert_eq!(harness.state().view_mode, ViewMode::Developer);
    harness.get_by_label("Step Frame (Shift+F11)");
}

#[test]
fn browser_style_binary_bytes_can_be_loaded_without_a_host_path() {
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.pending_binary = Some(gui::BinarySource::Bytes {
        name: "loop.bin".into(),
        data: vec![0x60, 0xfe],
    });
    app.load_binary_modal_open = true;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(
            |ctx, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ctx, app.host.status.generation);
                app.update_ui(ctx)
            },
            app,
        );
    common::settle_harness(&mut harness);
    harness.get_by_label("File: loop.bin");
    harness.get_by_label("Load into Memory").click_accesskit();
    common::settle_harness(&mut harness);
    assert!(!harness.state().load_binary_modal_open);
    assert_eq!(
        common::inspect(harness.state_mut())
            .memory
            .read_word_debug(0x1000),
        0x60fe
    );
    assert_eq!(
        common::inspect(harness.state_mut()).cpu.instruction_pc(),
        0x1000
    );
}

#[test]
fn game_shortcuts_cannot_start_debugger_work() {
    let mut session = Box::new(gui::DebuggerSession::new());
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    let mut app = EmulatorApp::with_session(session);
    app.set_view_mode(ViewMode::ScreenOnly);
    let before = common::snapshot(&mut app);
    let ctx = egui::Context::default();
    for (key, modifiers) in [
        (egui::Key::F10, egui::Modifiers::NONE),
        (egui::Key::F11, egui::Modifiers::NONE),
        (egui::Key::F11, egui::Modifiers::SHIFT),
        (egui::Key::F8, egui::Modifiers::NONE),
        (egui::Key::T, egui::Modifiers::ALT),
    ] {
        let _ = ctx.run(
            egui::RawInput {
                modifiers,
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
                ..Default::default()
            },
            |ctx| {
                common::keyboard::inject_taps(ctx, app.host.status.generation);
                app.update_ui(ctx)
            },
        );
    }
    assert_eq!(common::snapshot(&mut app), before);
    assert!(!common::inspect(&mut app).temporal.is_recording());
    assert!(common::inspect(&mut app)
        .debugger
        .trace
        .entries()
        .is_empty());
    assert!(app.show_microcode);
}

#[test]
fn capture_guidance_remains_visible_in_both_views_and_compact_windows() {
    for view in [ViewMode::ScreenOnly, ViewMode::Developer] {
        let mut app = EmulatorApp::default();
        app.set_view_mode(view);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(720.0, 600.0))
            .build_state(
                |ctx, app: &mut EmulatorApp| {
                    common::keyboard::inject_taps(ctx, app.host.status.generation);
                    app.update_ui(ctx)
                },
                app,
            );
        common::settle_harness(&mut harness);
        let bounds = harness
            .get_by_label(
                "Mouse free. Click the Amiga display to capture it. F12 switches Game/Debug.",
            )
            .accesskit_node()
            .bounding_box()
            .unwrap();
        assert!(bounds.y0 >= 0.0 && bounds.y1 < 150.0);
        assert!(bounds.x0 >= 0.0 && bounds.x1 <= 720.0);
    }
}
