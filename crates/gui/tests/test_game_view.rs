#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use egui_kittest::{
    kittest::{by, Queryable},
    Harness,
};
use gui::{EmulatorApp, ViewMode};

#[test]
fn game_execution_does_not_collect_debugger_data() {
    let mut app = EmulatorApp::default();
    app.view_mode = ViewMode::ScreenOnly;
    app.session
        .load_binary(0x1000, &[0x60, 0xfe], true)
        .unwrap();
    app.session.temporal.set_recording(true);
    app.session.toggle_run();
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| app.update_ui(ctx));
    assert_eq!(app.session.machine.denise.frame_builder.frame_number, 1);
    assert!(app.session.debugger.trace.entries().is_empty());
    assert!(app.session.temporal.is_empty());
    assert!(app.session.prev_cpu_state.is_none());
    assert_eq!(app.session.instructions_executed, 0);
    assert!(app.session.quick_slots.iter().all(Option::is_none));
}

#[test]
fn game_toolbar_supports_files_manual_states_speed_fps_and_return_to_debug() {
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.session
        .load_binary(0x1000, &[0x60, 0xfe], true)
        .unwrap();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    harness.run();
    for label in [
        "File",
        "State",
        "▶ Run (F5)",
        "Slower",
        "Faster",
        "1x",
        "FPS",
        "Debug View (F2 / F12)",
    ] {
        let bounds = harness.get_by_label(label).raw_bounds().unwrap();
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
    harness.get_by_label("File").click();
    harness.run();
    harness.get_by_label("Load ROM from File...");
    harness.get_by_label("📁 Load Binary... (Ctrl+O)");
    harness.get_by_label("File").click();
    harness.run();
    harness.get_by_label("FPS").click();
    harness.run();
    assert!(!harness.state().show_fps);
    assert!(harness.query_by_label("FPS: 0.0").is_none());
    harness.get_by_label("Faster").click();
    harness.run();
    assert_eq!(harness.state().playback.speed(), 2.0);
    harness.get_by_label("Slower").click();
    harness.run();
    assert_eq!(harness.state().playback.speed(), 1.0);
    harness.get_by_label("State").click();
    harness.run();
    harness.get_by_label("💾 Save State to File... (Ctrl+S)");
    harness.get_by_label("📂 Load State from File... (Ctrl+L)");
    harness.get_by_label("⚡ Quick Save Slot 1 (F6)").click();
    harness.run();
    assert!(harness.state().session.has_quick_slot(1));
    harness
        .state_mut()
        .session
        .machine
        .cpu
        .state
        .set_d_long(0, 123);
    harness.get_by_label("State").click();
    harness.run();
    harness.get_by_label("🔄 Quick Load Slot 1 (F9)").click();
    harness.run();
    assert_eq!(harness.state().session.machine.cpu.state.d_long(0), 0);
    assert_eq!(
        harness.state().loaded_rom_name.as_deref(),
        Some("from saved state")
    );
    harness.get_by_label("▶ Run (F5)").click();
    harness.run_steps(2);
    assert!(harness.state().session.is_running);
    harness.get_by_label("⏹ Stop (F5)").click();
    harness.run_steps(2);
    assert!(!harness.state().session.is_running);
    harness
        .query_all(by().role(egui::accesskit::Role::ComboBox))
        .next()
        .unwrap()
        .click();
    harness.run();
    harness.get_by_label("Light").click();
    harness.run();
    assert_eq!(harness.state().theme, gui::AppTheme::Light);
    harness
        .query_all(by().role(egui::accesskit::Role::ComboBox))
        .nth(1)
        .unwrap()
        .click();
    harness.run();
    harness.get_by_label("125%").click();
    harness.run();
    assert_eq!(harness.ctx.zoom_factor(), 1.25);
    harness.get_by_label("Debug View (F2 / F12)").click();
    harness.run();
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
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    harness.run();
    harness.get_by_label("File: loop.bin");
    harness.get_by_label("Load into Memory").click();
    harness.run();
    assert!(!harness.state().load_binary_modal_open);
    assert_eq!(
        harness.state().session.bus().read_word_debug(0x1000),
        0x60fe
    );
    assert_eq!(
        harness.state().session.machine.cpu.state.instruction_pc(),
        0x1000
    );
}

#[test]
fn game_shortcuts_cannot_start_debugger_work() {
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.session
        .load_binary(0x1000, &[0x60, 0xfe], true)
        .unwrap();
    let before = app.session.save_state();
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
            |ctx| app.update_ui(ctx),
        );
    }
    assert_eq!(app.session.save_state(), before);
    assert!(!app.session.temporal.is_recording());
    assert!(app.session.debugger.trace.entries().is_empty());
    assert!(app.show_microcode);
}
