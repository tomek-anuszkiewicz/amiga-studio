#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use common::TestContext as _;
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
    assert!(!owner.status().has_quicksave);
}

#[test]
fn game_toolbar_and_player_controls_follow_the_machine_owner() {
    let mut session = Box::new(gui::DebuggerSession::new());
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    let mut app = EmulatorApp::with_session(session);
    app.set_view_mode(ViewMode::ScreenOnly);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 720.0))
        .build_ui_state(common::show_full_viewport, app);
    common::settle_harness(&mut harness);

    for label in [
        "File",
        "▶ Run",
        "⏹ Stop",
        "Debug View",
        "Debug",
        "Setup",
        "Slower",
        "Faster",
        "1x",
        "FPS",
        "Step Frame",
        "⏭ Step Inst",
        "▶ Step CCK",
        "⏮ Rewind",
    ] {
        assert!(harness.query_by_label(label).is_none(), "{label}");
    }
    assert!(harness.query_by_label_contains("ROM:").is_none());
    assert!(harness.query_by_label_contains("F12").is_none());
    assert_eq!(harness.get_all_by_label_contains("FPS: ").count(), 1);
    harness.get_by_label("FPS: 0.0");
    assert!(harness.query_by_label("State").is_none());
    harness.get_by_label("Export state…");
    harness.get_by_label("Import state…");
    harness.get_by_label("Quick save").click_accesskit();
    common::settle_harness(&mut harness);
    assert!(harness.state().host.status.has_quicksave);
    common::command(
        harness.state_mut(),
        HostCommand::SetRegister {
            register: gui::EditRegister::D(0),
            value: 123,
        },
    );
    harness.get_by_label("Quick load").click_accesskit();
    common::wait_harness(&mut harness, |app| app.host.status.running);
    harness.get_by_label("Pause").click_accesskit();
    common::wait_harness(&mut harness, |app| !app.host.status.running);
    assert_eq!(common::inspect(harness.state_mut()).cpu.d_long(0), 0);
    assert!(harness
        .state()
        .toast_message
        .as_ref()
        .unwrap()
        .0
        .contains("Quicksave restored"));

    let generation = harness.state().host.status.generation;
    harness.get_by_label("Resume").click_accesskit();
    common::wait_harness(&mut harness, |app| {
        app.host.status.running && app.host.status.fps > 0.0
    });
    assert!(harness.state().host.status.field_number > 0);
    let measured = harness.state().host.status.fps;
    assert_eq!(harness.get_all_by_label_contains("FPS: ").count(), 1);
    harness.get_by_label(&format!("FPS: {measured:.1}"));
    assert!(harness.query_by_label("Resume").is_none());

    harness.get_by_label("Pause").click_accesskit();
    common::wait_harness(&mut harness, |app| {
        !app.host.status.running && !app.host.status.pause_pending
    });
    assert_eq!(harness.state().host.status.generation, generation);
    assert_eq!(harness.state().host.status.fps, 0.0);
    harness.get_by_label("FPS: 0.0");
    harness.get_by_label("Resume");
    assert!(harness.query_by_label("Pause").is_none());

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
    tap_physical(&mut harness, "F12");
    assert_eq!(harness.state().view_mode, ViewMode::Developer);
    for label in ["File", "State", "Snapshot files"] {
        assert!(harness.query_by_label(label).is_none(), "{label}");
    }
    assert!(harness.query_by_label_contains("ROM:").is_none());
    harness.get_by_label("Step Frame");
    for label in ["Export state file…", "Import state file…"] {
        harness.get_by_role_and_label(egui::accesskit::Role::Button, label);
    }
}

#[test]
fn setup_navigation_returns_to_the_same_paused_session() {
    use gui::player::PlayerScreen;

    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 780.0))
        .build_ui_state(common::show_full_viewport, app);
    common::settle_harness(&mut h);
    let generation = h.state().host.status.generation;
    let before = common::snapshot(h.state_mut());

    for width in [1280.0, 340.0] {
        h.set_size(egui::vec2(width, 780.0));
        common::settle_harness(&mut h);
        h.get_by_label("Manual setup").click_accesskit();
        common::settle_harness(&mut h);
        assert_eq!(h.state().player.screen, PlayerScreen::Setup);
        for label in [
            "Library",
            "Manual setup",
            "Configurations",
            "Return to session",
        ] {
            assert!(
                h.ctx
                    .content_rect()
                    .contains_rect(h.get_by_label(label).rect()),
                "{label}"
            );
        }
        assert!(h.query_by_label("Debug").is_none());
        h.get_by_label("Return to session").click_accesskit();
        common::settle_harness(&mut h);
        assert_eq!(h.state().player.screen, PlayerScreen::Session);
        assert_eq!(h.state().host.status.generation, generation);
        assert_eq!(common::snapshot(h.state_mut()), before);
    }
}

#[test]
fn bundled_instructions_open_and_close_without_replacing_or_pausing_the_session() {
    let mut session = Box::new(gui::DebuggerSession::new());
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    let mut app = EmulatorApp::with_session(session);
    app.set_view_mode(ViewMode::ScreenOnly);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(common::show_full_viewport, app);
    common::settle_harness(&mut h);
    let before = common::snapshot(h.state_mut());
    let generation = h.state().host.status.generation;
    h.get_by_label("Instructions").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Amiga 500 player guide");
    h.get_by_label("Open Configurations");
    h.get_by_label_contains("press F12 to switch between Game view and Developer Studio");
    assert_eq!(h.state().host.status.generation, generation);
    assert_eq!(common::snapshot(h.state_mut()), before);
    h.get_by_label("Controls").click_accesskit();
    common::settle_harness(&mut h);

    h.set_size(egui::vec2(600.0, 720.0));
    common::settle_harness(&mut h);
    h.get_by_label("Game controls").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Resume").click_accesskit();
    common::wait_harness(&mut h, |app| app.host.status.running);
    let field = h.state().host.status.field_number;
    h.get_by_label("Instructions").click_accesskit();
    common::wait_harness(&mut h, |app| app.host.status.field_number > field);
    let title = h.get_by_label("Amiga 500 player guide");
    assert!(h.ctx.content_rect().contains_rect(title.rect()));
    h.get_by_label("Controls").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Pause");
    assert!(h.state().host.status.running);
    assert_eq!(h.state().host.status.generation, generation);
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
        let _ = ctx.run_test_ui(
            egui::RawInput {
                events: vec![
                    egui::Event::ModifiersChanged(modifiers),
                    egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    },
                ],
                ..Default::default()
            },
            |ui| {
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui)
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
            .build_ui_state(
                |ui, app: &mut EmulatorApp| {
                    common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                    app.update_ui(ui)
                },
                app,
            );
        common::settle_harness(&mut harness);
        let bounds = harness
            .get_by_label("Mouse free. Click the Amiga display to capture it.")
            .accesskit_node()
            .bounding_box()
            .unwrap();
        assert!(bounds.y0 >= 0.0 && bounds.y1 < 150.0);
        assert!(bounds.x0 >= 0.0 && bounds.x1 <= 720.0);
    }
}

#[test]
fn game_toolbar_uses_one_row_when_wide_and_wraps_at_larger_zoom() {
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 740.0))
        .build_ui_state(common::show_full_viewport, app);
    common::settle_harness(&mut h);
    let positions = |h: &Harness<'_, EmulatorApp>| {
        [
            "Manual setup",
            "PAUSED",
            "−",
            "Speed 1.00x",
            "+",
            "FPS: 0.0",
            "Image scale",
            "Mouse free. Click the Amiga display to capture it.",
        ]
        .map(|label| {
            let rect = h.get_by_label(label).rect();
            assert!(
                h.ctx.content_rect().contains_rect(rect),
                "Clipped {label}: {rect:?}"
            );
            rect.center().y
        })
    };
    let wide_rows = positions(&h);
    assert!(wide_rows.iter().all(|y| (*y - wide_rows[0]).abs() < 5.0));

    h.set_size(egui::vec2(640.0, 740.0));
    h.step();
    h.ctx.set_zoom_factor(1.5);
    common::settle_harness(&mut h);
    assert_eq!(h.ctx.zoom_factor(), 1.5);
    assert!((h.ctx.content_rect().width() - 640.0 / 1.5).abs() < 1.0);
    let narrow_rows = positions(&h);
    assert!(narrow_rows[7] > narrow_rows[0] + 8.0);
    assert!((narrow_rows[2] - narrow_rows[4]).abs() < 5.0);
}

#[test]
fn shortcut_hover_shows_active_bindings_and_only_applicable_equivalents() {
    use gui::player::PlayerScreen;

    let mut session = Box::new(gui::DebuggerSession::new());
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    let mut app = EmulatorApp::with_session(session);
    app.set_view_mode(ViewMode::ScreenOnly);
    #[cfg(not(target_arch = "wasm32"))]
    let library = tempfile::tempdir().unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.player.library_root = Some(library.path().to_path_buf());
    }
    app.player.activate_selected_configuration();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(common::show_full_viewport, app);
    h.ctx
        .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
    common::settle_harness(&mut h);

    let hover_control = |h: &mut Harness<'_, EmulatorApp>, label| {
        h.remove_cursor();
        common::settle_harness(h);
        h.get_by_label(label).hover();
        common::settle_harness(h);
    };
    hover_control(&mut h, "Resume");
    h.get_by_label("Keyboard: F5");
    h.get_by_label_contains("Resume the same machine");

    h.state_mut().navigate_player(PlayerScreen::Configuration);
    common::settle_harness(&mut h);
    h.get_by_label("F5").click_accesskit();
    common::settle_harness(&mut h);
    tap_physical(&mut h, "F7");
    h.get_by_label("Save").click_accesskit();
    common::settle_harness(&mut h);
    hover_control(&mut h, "Resume");
    h.get_by_label("Keyboard: F7");
    assert!(h.query_by_label("Keyboard: F5").is_none());
    tap_physical(&mut h, "F7");
    common::wait_harness(&mut h, |app| app.host.status.running);
    hover_control(&mut h, "Pause");
    h.get_by_label("Keyboard: F7");
    h.get_by_label("Pause after the current video field completes.");

    tap_physical(&mut h, "F7");
    common::wait_harness(&mut h, |app| !app.host.status.running);
    hover_control(&mut h, "Reset");
    h.get_by_label_contains("Reset and run the current machine");
    assert!(h.query_by_label_contains("Keyboard:").is_none());

    tap_physical(&mut h, "F12");
    assert_eq!(h.state().view_mode, ViewMode::Developer);
    hover_control(&mut h, "🎮 Game View");
    h.get_by_label("Keyboard: F12");
    hover_control(&mut h, "Step Frame");
    h.get_by_label("Keyboard: Shift+F11");
    let frame = h.state().host.status.field_number;
    let generation = h.state().host.status.generation;
    gui::keyboard_input::receive_key(
        &h.ctx,
        common::keyboard::key("ShiftLeft", true),
        Some(generation),
    );
    tap_physical(&mut h, "F11");
    gui::keyboard_input::receive_key(
        &h.ctx,
        common::keyboard::key("ShiftLeft", false),
        Some(generation),
    );
    common::wait_harness(&mut h, |app| app.host.status.field_number > frame);
    assert!(!h.state().host.status.running);
}

fn tap_physical(h: &mut Harness<'_, EmulatorApp>, key: &str) {
    let generation = h.state().host.status.generation;
    for pressed in [true, false] {
        assert!(gui::keyboard_input::receive_key(
            &h.ctx,
            common::keyboard::key(key, pressed),
            Some(generation)
        ));
    }
    common::settle_harness(h);
}
