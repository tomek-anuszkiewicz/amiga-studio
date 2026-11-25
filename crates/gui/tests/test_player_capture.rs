#![allow(clippy::unwrap_used, clippy::panic)]
mod common;
use common::TestContext as _;
use egui_kittest::kittest::NodeT;
use gui::host_commands::{HostCommand, HostValue};
use gui::{EmulatorApp, ViewMode};

fn running() -> (EmulatorApp, egui::Context) {
    running_with(ViewMode::ScreenOnly, true, Default::default())
}

fn running_with(
    view: ViewMode,
    captured: bool,
    configuration: gui::player_config::NamedConfiguration,
) -> (EmulatorApp, egui::Context) {
    let mut session = Box::new(gui::DebuggerSession::new());
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    debugger::launch::reconcile_ports(
        &mut session,
        configuration
            .controls
            .ports
            .each_ref()
            .map(|port| port.device),
    );
    let mut app = EmulatorApp::with_session(session);
    app.set_view_mode(view);
    let ctx = egui::Context::default();
    common::settle(&mut app, &ctx);
    common::command(&mut app, HostCommand::Run);
    app.player.active_configuration = Some(configuration);
    app.player.active_mappings = app.player.catalog.mappings.clone();
    app.player.captured = captured;
    common::settle(&mut app, &ctx);
    (app, ctx)
}
fn draw(app: &mut EmulatorApp, ctx: &egui::Context, key: Option<egui::Key>) {
    let events = key
        .map(|key| egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
        .into_iter()
        .collect();
    let _ = ctx.run_test_ui(
        egui::RawInput {
            focused: true,
            events,
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
}
#[test]
fn free_mouse_guest_navigation_keys_leave_a_focused_button_alone() {
    let (mut app, ctx) = running();
    app.player.captured = false;
    // The general navigation regression observes guest keys. The configured
    // keyboard joystick has independent coverage in the next test.
    app.player.active_mappings[0].keys[0] = 0x17;
    let mut clicked = false;
    let render = |ui: &mut egui::Ui, app: &mut EmulatorApp, clicked: &mut bool| {
        egui::Window::new("Session controls").show(ui.ctx(), |ui| {
            let response = ui.button("Ordinary session button");
            if ui.ctx().memory(|memory| memory.focused().is_none()) {
                response.request_focus();
            }
            *clicked |= response.clicked();
            let _ = ui.button("Another button");
        });
        app.update_ui(ui);
    };
    let _ = ctx.run_test_ui(egui::RawInput::default(), |ui| {
        render(ui, &mut app, &mut clicked)
    });
    let focus = ctx.memory(|memory| memory.focused()).unwrap();
    let generation = app.host.status.generation;
    for (physical, key, code) in [
        ("Tab", egui::Key::Tab, 0x42),
        ("ArrowUp", egui::Key::ArrowUp, 0x4c),
        ("Space", egui::Key::Space, 0x40),
    ] {
        for pressed in [true, false] {
            let routed = gui::keyboard_input::receive_key(
                &ctx,
                common::keyboard::key(physical, pressed),
                Some(generation),
            );
            // The native adapter and browser listener forward only GUI-owned keys.
            let raw = egui::RawInput {
                events: (!routed)
                    .then_some(egui::Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    })
                    .into_iter()
                    .collect(),
                ..Default::default()
            };
            let _ = ctx.run_test_ui(raw, |ui| render(ui, &mut app, &mut clicked));
            assert!(routed);
            assert_eq!(ctx.memory(|memory| memory.focused()), Some(focus));
            assert!(!clicked);
            assert!(!app.player.captured);
            wait_guest_key(&mut app, &ctx, code, pressed);
            let _ = ctx.run_test_ui(egui::RawInput::default(), |ui| {
                render(ui, &mut app, &mut clicked)
            });
        }
    }
}

fn queue_window_keys(ctx: &egui::Context, edges: &[(&str, bool, u64)]) {
    for &(physical, pressed, generation) in edges {
        if physical == "__focus_lost__" {
            gui::keyboard_input::receive_focus(ctx, false, Some(generation));
        } else {
            let mut key = common::keyboard::key(physical, pressed);
            key.logical = "different logical key".into();
            gui::keyboard_input::receive_key(ctx, key, Some(generation));
        }
    }
}

fn window_keys(app: &mut EmulatorApp, ctx: &egui::Context, edges: &[(&str, bool, u64)]) {
    queue_window_keys(ctx, edges);
    draw(app, ctx, None);
}

fn guest_key_held(app: &mut EmulatorApp, code: u8) -> bool {
    common::observe_input(app, Some(code)).key_held.unwrap()
}

fn saved_key_held(state: &debugger::A500State, code: usize) -> bool {
    let keyboard = serde_json::to_value(&state.keyboard).unwrap();
    keyboard["held"][code / 64].as_u64().unwrap() & (1 << (code % 64)) != 0
}

fn wait_guest_key(app: &mut EmulatorApp, ctx: &egui::Context, code: u8, held: bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        common::settle(app, ctx);
        if guest_key_held(app, code) == held {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Guest key {code:#x} did not become held={held}"
        );
        std::thread::yield_now();
    }
}

#[test]
fn active_running_sessions_deliver_keys_and_keyboard_joystick_with_the_mouse_free() {
    for mode in [ViewMode::ScreenOnly, ViewMode::Developer] {
        let (mut app, ctx) = running_with(mode, false, Default::default());
        let generation = app.host.status.generation;
        window_keys(&mut app, &ctx, &[("KeyA", true, generation)]);
        wait_guest_key(&mut app, &ctx, 0x20, true);
        assert!(!app.player.captured);
        window_keys(&mut app, &ctx, &[("KeyA", false, generation)]);
        wait_guest_key(&mut app, &ctx, 0x20, false);
        window_keys(
            &mut app,
            &ctx,
            &[("ArrowUp", true, generation), ("KeyZ", true, generation)],
        );
        delivered(&mut app, &ctx);
        assert!(fire(&mut app));
        assert_ne!(
            owner_snapshot(&mut app).game_ports.pins.direction_pins[1] & 1,
            1
        );
        assert!(!guest_key_held(&mut app, 0x4c));
        assert!(!guest_key_held(&mut app, 0x31));
        window_keys(
            &mut app,
            &ctx,
            &[("ArrowUp", false, generation), ("KeyZ", false, generation)],
        );
        delivered(&mut app, &ctx);
        assert!(!fire(&mut app));
    }
}

#[test]
fn window_ingress_rejects_stale_generations_and_delivers_physical_hold_and_release() {
    let (mut app, ctx) = running();
    let generation = app.host.status.generation;
    // Keep the physical key waiting for owner activation while the mouse is released.
    let sender = app.host.runtime.as_ref().unwrap().input_sender();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !sender.set_enabled(false, false, true) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    let runtime = app.host.runtime.take().unwrap();
    window_keys(
        &mut app,
        &ctx,
        &[
            ("KeyA", true, generation.wrapping_sub(1)),
            ("KeyB", true, generation),
        ],
    );
    assert!(app.player.captured);
    app.player.captured = false;
    draw(&mut app, &ctx, None);
    app.host.runtime = Some(runtime);
    draw(&mut app, &ctx, None);
    wait_guest_key(&mut app, &ctx, 0x35, true);
    assert!(!guest_key_held(&mut app, 0x20), "Stale A reached the guest");
    assert!(!app.player.captured);
    window_keys(&mut app, &ctx, &[("KeyB", false, generation)]);
    wait_guest_key(&mut app, &ctx, 0x35, false);
    for (physical, code) in [("SuperLeft", 0x66), ("SuperRight", 0x67)] {
        window_keys(&mut app, &ctx, &[(physical, true, generation)]);
        wait_guest_key(&mut app, &ctx, code, true);
        window_keys(&mut app, &ctx, &[(physical, false, generation)]);
        wait_guest_key(&mut app, &ctx, code, false);
    }
}

#[test]
fn window_ingress_focus_loss_and_inbox_overflow_release_held_guest_input() {
    let (mut app, ctx) = running();
    let generation = app.host.status.generation;
    window_keys(&mut app, &ctx, &[("KeyA", true, generation)]);
    wait_guest_key(&mut app, &ctx, 0x20, true);
    window_keys(
        &mut app,
        &ctx,
        &[
            ("__focus_lost__", false, generation),
            ("KeyB", true, generation),
            ("F6", true, generation),
            ("F6", false, generation),
        ],
    );
    wait_guest_key(&mut app, &ctx, 0x20, false);
    assert!(!app.player.captured);
    assert!(app.host.status.running);
    assert!(!guest_key_held(&mut app, 0x20));
    assert!(!app.host.status.quick_slots[0]);
    assert!(
        !guest_key_held(&mut app, 0x35),
        "B escaped focus-loss ordering"
    );

    app.player.captured = true;
    window_keys(
        &mut app,
        &ctx,
        &[("KeyB", false, generation), ("KeyA", true, generation)],
    );
    wait_guest_key(&mut app, &ctx, 0x20, true);
    let overflow: Vec<_> = (0..=gui::host_input::INPUT_QUEUE_CAPACITY)
        .map(|edge| ("KeyB", edge % 2 == 0, generation))
        .collect();
    window_keys(&mut app, &ctx, &overflow);
    wait_guest_key(&mut app, &ctx, 0x20, false);
    assert!(!app.player.captured);
    assert!(app.host.status.running);
    assert!(!guest_key_held(&mut app, 0x20), "Overflow left A held");
    assert!(!guest_key_held(&mut app, 0x35), "Overflow replayed B");
}

#[test]
fn quick_save_shortcut_keeps_capture_and_generation_while_running() {
    let (mut app, ctx) = running();
    let generation = app.host.status.generation;
    draw(&mut app, &ctx, Some(egui::Key::F6));
    common::settle(&mut app, &ctx);
    assert!(app.player.captured);
    assert!(app.host.status.quick_slots[0]);
    assert!(app.host.status.running);
    assert_eq!(app.host.status.generation, generation);
}
#[test]
fn configured_letter_shortcuts_take_priority_over_guest_keys_and_joystick() {
    let (mut app, ctx) = running_with(ViewMode::ScreenOnly, false, Default::default());
    let bindings = &mut app
        .player
        .active_configuration
        .as_mut()
        .unwrap()
        .controls
        .host
        .frontend;
    bindings.save = "A".into();
    bindings.pause = "B".into();
    bindings.load = "C".into();
    common::settle(&mut app, &ctx);
    let generation = app.host.status.generation;
    let shortcut = |app: &mut EmulatorApp, physical, generation| {
        let key = common::keyboard::key(physical, true);
        assert!(gui::keyboard_input::receive_key(
            &ctx,
            key.clone(),
            Some(generation)
        ));
        let mut repeated = key;
        repeated.repeat = true;
        assert!(gui::keyboard_input::receive_key(
            &ctx,
            repeated,
            Some(generation)
        ));
        assert!(gui::keyboard_input::receive_key(
            &ctx,
            common::keyboard::key(physical, false),
            Some(generation)
        ));
        draw(app, &ctx, None);
        common::settle(app, &ctx);
    };
    shortcut(&mut app, "KeyA", generation);
    assert!(app.host.status.quick_slots[0]);
    assert!(app.host.status.running);
    assert_eq!(app.host.status.generation, generation);
    delivered(&mut app, &ctx);
    assert_eq!(
        owner_snapshot(&mut app).game_ports.pins.direction_pins[1] & 1,
        1
    );
    assert!(!guest_key_held(&mut app, 0x20));
    shortcut(&mut app, "KeyB", generation);
    common::wait(&mut app, &ctx, |app| !app.host.status.running);
    assert!(!guest_key_held(&mut app, 0x35));
    shortcut(&mut app, "KeyC", generation);
    assert_ne!(app.host.status.generation, generation);
    assert!(!guest_key_held(&mut app, 0x33));
}

#[test]
fn enter_bound_to_release_does_not_recapture_through_a_keyboard_canvas_click() {
    let (mut app, ctx) = running();
    app.player
        .active_configuration
        .as_mut()
        .unwrap()
        .controls
        .host
        .frontend
        .release = "Enter".into();
    draw(&mut app, &ctx, None);
    let generation = app.host.status.generation;
    window_keys(
        &mut app,
        &ctx,
        &[("Enter", true, generation), ("Enter", false, generation)],
    );
    assert!(!app.player.captured);
}

#[test]
fn long_disk_names_keep_the_wide_player_panel_compact() {
    use egui_kittest::{kittest::Queryable, Harness};
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.player.disks.push(gui::player::SessionDisk {
        id: "long".into(),
        label:
            "Workbench v1.3 rev 34.20 (1988)(Commodore)(A500-A2000)(Disk 1 of 2)(Workbench)[m].adf"
                .into(),
        bytes: None,
    });
    app.player.mounted[0] = Some("long".into());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui)
            },
            app,
        );
    common::settle_harness(&mut harness);
    let save = harness
        .get_by_label("Save")
        .accesskit_node()
        .bounding_box()
        .unwrap();
    assert!(
        save.x0 >= 1000.0 && save.x1 <= 1280.0,
        "Long media label stretched the player panel: {save:?}"
    );
}

#[test]
fn escape_and_f2_remain_guest_keys_and_optional_release_frees_only_the_mouse() {
    let (mut app, ctx) = running();
    for key in [egui::Key::Escape, egui::Key::F2] {
        draw(&mut app, &ctx, Some(key));
        assert!(app.player.captured);
        assert_eq!(app.view_mode, ViewMode::ScreenOnly);
    }
    app.player
        .active_configuration
        .as_mut()
        .unwrap()
        .controls
        .host
        .frontend
        .release = "F4".into();
    draw(&mut app, &ctx, None);
    let generation = app.host.status.generation;
    window_keys(
        &mut app,
        &ctx,
        &[
            ("KeyA", true, generation),
            ("F4", true, generation),
            ("F4", false, generation),
        ],
    );
    assert!(!app.player.captured);
    wait_guest_key(&mut app, &ctx, 0x20, true);
    window_keys(&mut app, &ctx, &[("KeyA", false, generation)]);
    wait_guest_key(&mut app, &ctx, 0x20, false);
    assert!(app.host.status.running);
}

#[test]
fn middle_mouse_cancels_motion_and_buttons_but_preserves_keyboard_and_joystick() {
    let (mut app, ctx) = running();
    use gui::host_input::HostInputEvent;
    // The Run reply can arrive before the worker publishes input-ready status.
    delivered(&mut app, &ctx);
    let generation = app.host.status.generation;
    let sender = app.host.runtime.as_ref().unwrap().input_sender();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    for input in [
        HostInputEvent::MouseMotion {
            port: 0,
            dx: 100_000_000,
            dy: 0,
        },
        HostInputEvent::MouseButtons {
            port: 0,
            left: true,
            right: true,
            middle: false,
        },
    ] {
        while !sender.try_host_event(input, generation) {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    window_keys(
        &mut app,
        &ctx,
        &[("KeyA", true, generation), ("ArrowUp", true, generation)],
    );
    wait_guest_key(&mut app, &ctx, 0x20, true);
    delivered(&mut app, &ctx);
    let before = owner_snapshot(&mut app);
    assert_eq!(before.game_ports.sample_ciaa_port_a_inputs() & 0x40, 0);
    assert_eq!(before.game_ports.pins.direction_pins[1] & 1, 0);
    let mouse = serde_json::to_value(before.game_ports.state.port1).unwrap();
    assert!(mouse["Mouse"]["pending_x"].as_i64().unwrap() > 0);
    ctx.data_mut(|data| data.insert_temp(egui::Id::unique("capture_requested"), true));
    let _ = ctx.run_test_ui(
        egui::RawInput {
            events: vec![egui::Event::PointerButton {
                pos: egui::pos2(400.0, 250.0),
                button: egui::PointerButton::Middle,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ui| {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        },
    );
    assert!(!app.player.captured);
    assert!(app.host.status.running);
    delivered(&mut app, &ctx);
    let after = owner_snapshot(&mut app);
    let mouse = serde_json::to_value(after.game_ports.state.port1).unwrap();
    assert_eq!(mouse["Mouse"]["pending_x"], 0);
    assert_eq!(mouse["Mouse"]["left_button"], false);
    assert_eq!(mouse["Mouse"]["right_button"], false);
    assert_eq!(mouse["Mouse"]["middle_button"], false);
    assert_eq!(after.game_ports.pins.direction_pins[1] & 1, 0);
    assert!(guest_key_held(&mut app, 0x20));
    window_keys(
        &mut app,
        &ctx,
        &[("KeyA", false, generation), ("ArrowUp", false, generation)],
    );
    wait_guest_key(&mut app, &ctx, 0x20, false);
    delivered(&mut app, &ctx);
    assert_eq!(
        owner_snapshot(&mut app).game_ports.pins.direction_pins[1] & 1,
        1
    );
    draw(&mut app, &ctx, None);
    assert!(
        !app.player.captured,
        "Window activation must not recapture input"
    );
}

fn pad(sender: &gui::host_runtime::HostInputSender, generation: u64, pressed: bool) {
    // Standard controller south and up buttons.
    pad_buttons(sender, generation, if pressed { &[0, 12] } else { &[] });
}

fn pad_buttons(sender: &gui::host_runtime::HostInputSender, generation: u64, pressed: &[usize]) {
    let mut buttons = vec![false; 16];
    for &index in pressed {
        buttons[index] = true;
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let sample = gui::input_mapping::GamepadSample {
            identity: "pad".into(),
            name: "Test controller".into(),
            axes: vec![0.0, 0.0],
            buttons: buttons.clone(),
        };
        if sender.try_gamepads(vec![sample], generation, true) {
            return;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
}
fn delivered(app: &mut EmulatorApp, ctx: &egui::Context) {
    // The snapshot preserves the timing of fixtures that pause at a field boundary.
    delivered_after(app, ctx, |app| {
        let _ = owner_snapshot(app);
    });
}

fn delivered_after_input_read(app: &mut EmulatorApp, ctx: &egui::Context) {
    delivered_after(app, ctx, |app| {
        let _ = common::observe_input(app, None);
    });
}

fn delivered_after(
    app: &mut EmulatorApp,
    ctx: &egui::Context,
    read_owner: impl FnOnce(&mut EmulatorApp),
) {
    common::settle(app, ctx);
    // Establish an owner barrier after submission; cached GUI telemetry may predate it.
    read_owner(app);
    let before = app.host.status.input_deliveries;
    common::wait(app, ctx, |app| {
        app.host.status.input_deliveries >= before + 2
            // Admission may occur at a zero-budget boundary; await an actual CCK sample.
            && app.host.status.cck > app.host.status.last_input_cck
    });
}
fn fire(app: &mut EmulatorApp) -> bool {
    owner_snapshot(app).game_ports.sample_ciaa_port_a_inputs() & 0x80 == 0
}

fn observed_fire(app: &mut EmulatorApp) -> bool {
    common::observe_input(app, None).ciaa_port_a_inputs & 0x80 == 0
}
fn owner_snapshot(app: &mut EmulatorApp) -> Box<debugger::A500State> {
    let reply = common::idle_command(app, HostCommand::SaveSnapshot);
    let Ok(HostValue::Snapshot(json)) = reply.value else {
        panic!("Missing machine snapshot");
    };
    let saved = debugger::SaveState::from_json(&json).unwrap();
    app.host.status = reply.status;
    saved.state
}

fn click_view_toolbar(app: &mut EmulatorApp, ctx: &egui::Context) {
    let label = match app.view_mode {
        ViewMode::Developer => "🎮 Game View",
        ViewMode::ScreenOnly => "Debug View",
    };
    ctx.enable_accesskit();
    let tree = ui_frame(app, ctx, vec![])
        .into_platform_output()
        .accesskit_update
        .unwrap();
    let target = tree
        .nodes
        .iter()
        .find_map(|(id, node)| {
            (node.role() == egui::accesskit::Role::Button && node.label() == Some(label))
                .then_some(*id)
        })
        .unwrap_or_else(|| panic!("Missing {label} toolbar button"));
    ui_frame(
        app,
        ctx,
        vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Click,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: target,
                data: None,
            },
        )],
    );
}

#[track_caller]
fn assert_held_view_controls(snapshot: &debugger::A500State, held: bool) {
    assert_eq!(saved_key_held(snapshot, 0x20), held, "Amiga A key");
    assert_eq!(
        snapshot.game_ports.pins.direction_pins[0] & 1 == 0,
        held,
        "Keyboard joystick up"
    );
    assert_eq!(
        snapshot.game_ports.pins.direction_pins[1] & 1 == 0,
        held,
        "Gamepad up"
    );
    assert_eq!(
        snapshot.game_ports.sample_ciaa_port_a_inputs() & 0x80 == 0,
        held,
        "Gamepad fire"
    );
}

fn view_change_releases_gameplay(start: ViewMode, via_toolbar: bool) {
    use gui::player_config::{InputSource, NamedConfiguration};
    let mut configuration = NamedConfiguration::default();
    configuration.controls.ports[0] = configuration.controls.ports[1].clone();
    configuration.controls.ports[1].source = InputSource::Gamepad {
        identity: "pad".into(),
        mapping: "arrows".into(),
    };
    let (mut app, ctx) = running_with(start, false, configuration);
    let sender = app.host.runtime.as_ref().unwrap().input_sender();
    let generation = app.host.status.generation;
    delivered(&mut app, &ctx);
    pad(&sender, generation, false);
    pad(&sender, generation, true);
    window_keys(
        &mut app,
        &ctx,
        &[("KeyA", true, generation), ("ArrowUp", true, generation)],
    );
    delivered(&mut app, &ctx);
    assert_held_view_controls(&owner_snapshot(&mut app), true);
    app.set_view_mode(start);
    delivered(&mut app, &ctx);
    assert!(app.host.status.running);
    assert_held_view_controls(&owner_snapshot(&mut app), true);

    if via_toolbar {
        click_view_toolbar(&mut app, &ctx);
    } else {
        window_keys(&mut app, &ctx, &[("F12", true, generation)]);
    }
    let expected_view = if start == ViewMode::Developer {
        ViewMode::ScreenOnly
    } else {
        ViewMode::Developer
    };
    assert_eq!(app.view_mode, expected_view);
    if expected_view == ViewMode::Developer {
        common::wait(&mut app, &ctx, |app| !app.host.status.running);
        let paused_cck = app.host.status.cck;
        let paused = owner_snapshot(&mut app);
        assert!(!app.host.status.running);
        assert_eq!(paused.cck, paused_cck);
        assert!(!saved_key_held(&paused, 0x20), "Amiga A key at pause");
        for (port, label) in [
            (paused.game_ports.port1(), "Keyboard joystick"),
            (paused.game_ports.port2(), "Gamepad"),
        ] {
            let device = serde_json::to_value(port).unwrap();
            let contacts = device["Joystick"]["direction_contacts"].as_u64().unwrap();
            assert_eq!(contacts & 1, 0, "{label} Up contact at pause");
        }
        assert!(
            !paused.game_ports.fire1_port2(),
            "Gamepad fire contact at pause"
        );

        // The machine samples the released contacts on the next CCK.
        let step = common::idle_command(&mut app, HostCommand::StepCck);
        assert!(
            matches!(step.value, Ok(HostValue::Done)),
            "Single CCK step failed: {:?}",
            step.value
        );
        assert!(!step.status.running);
        assert_eq!(step.status.cck, paused_cck + 1);
        let sampled = owner_snapshot(&mut app);
        assert!(!app.host.status.running);
        assert_eq!(sampled.cck, step.status.cck);
        assert_held_view_controls(&sampled, false);
        assert_ne!(sampled.cia_a.pins.port_a_input & 0x80, 0, "CIA-A Fire pin");
        common::command(&mut app, HostCommand::Run);
    }
    delivered(&mut app, &ctx);
    assert!(app.host.status.running);
    assert_eq!(app.host.status.generation, generation);
    assert_held_view_controls(&owner_snapshot(&mut app), false);
    for physical in ["KeyA", "ArrowUp"] {
        let mut key = common::keyboard::key(physical, true);
        key.repeat = true;
        assert!(gui::keyboard_input::receive_key(
            &ctx,
            key,
            Some(generation)
        ));
    }
    if !via_toolbar {
        let mut repeat = common::keyboard::key("F12", true);
        repeat.repeat = true;
        assert!(gui::keyboard_input::receive_key(
            &ctx,
            repeat,
            Some(generation)
        ));
        assert!(gui::keyboard_input::receive_key(
            &ctx,
            common::keyboard::key("F12", false),
            Some(generation),
        ));
    }
    pad(&sender, generation, true);
    delivered(&mut app, &ctx);
    assert_eq!(app.view_mode, expected_view);
    assert_held_view_controls(&owner_snapshot(&mut app), false);

    window_keys(
        &mut app,
        &ctx,
        &[("KeyA", false, generation), ("ArrowUp", false, generation)],
    );
    pad(&sender, generation, false);
    pad(&sender, generation, true);
    window_keys(
        &mut app,
        &ctx,
        &[("KeyA", true, generation), ("ArrowUp", true, generation)],
    );
    delivered(&mut app, &ctx);
    assert_held_view_controls(&owner_snapshot(&mut app), true);
}

#[test]
fn toolbar_view_switches_release_gameplay_until_fresh_input() {
    view_change_releases_gameplay(ViewMode::Developer, true);
    view_change_releases_gameplay(ViewMode::ScreenOnly, true);
}

#[test]
fn f12_view_switches_release_gameplay_until_fresh_input() {
    view_change_releases_gameplay(ViewMode::Developer, false);
    view_change_releases_gameplay(ViewMode::ScreenOnly, false);
}

#[test]
fn switching_to_game_accepts_the_first_press_from_a_neutral_gamepad() {
    use gui::player_config::{InputSource, NamedConfiguration};
    let mut configuration = NamedConfiguration::default();
    configuration.controls.ports[1].source = InputSource::Gamepad {
        identity: "pad".into(),
        mapping: "arrows".into(),
    };
    let (mut app, ctx) = running_with(ViewMode::Developer, false, configuration);
    let sender = app.host.runtime.as_ref().unwrap().input_sender();
    let generation = app.host.status.generation;
    delivered(&mut app, &ctx);
    pad(&sender, generation, false);
    delivered(&mut app, &ctx);
    assert!(!fire(&mut app));

    window_keys(&mut app, &ctx, &[("F12", true, generation)]);
    delivered(&mut app, &ctx);
    assert_eq!(app.view_mode, ViewMode::ScreenOnly);
    assert!(app.host.status.running);
    assert_eq!(app.host.status.generation, generation);
    assert!(!fire(&mut app));

    pad(&sender, generation, true);
    delivered(&mut app, &ctx);
    let snapshot = owner_snapshot(&mut app);
    assert_eq!(
        snapshot.game_ports.pins.direction_pins[1] & 1,
        0,
        "The first gamepad direction press reaches the guest"
    );
    assert_eq!(
        snapshot.game_ports.sample_ciaa_port_a_inputs() & 0x80,
        0,
        "The first gamepad fire press reaches the guest"
    );
}

#[test]
fn changing_views_releases_the_captured_mouse_button() {
    use gui::host_input::HostInputEvent;
    let (mut app, ctx) = running_with(ViewMode::Developer, true, Default::default());
    delivered(&mut app, &ctx);
    let sender = app.host.runtime.as_ref().unwrap().input_sender();
    let generation = app.host.status.generation;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !sender.try_host_event(
        HostInputEvent::MouseButtons {
            port: 0,
            left: true,
            right: false,
            middle: false,
        },
        generation,
    ) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    delivered(&mut app, &ctx);
    app.set_view_mode(ViewMode::Developer);
    delivered(&mut app, &ctx);
    assert!(app.player.captured);
    assert_eq!(
        owner_snapshot(&mut app)
            .game_ports
            .sample_ciaa_port_a_inputs()
            & 0x40,
        0
    );

    click_view_toolbar(&mut app, &ctx);
    delivered(&mut app, &ctx);
    assert_eq!(app.view_mode, ViewMode::ScreenOnly);
    assert!(app.host.status.running);
    assert!(!app.player.captured);
    assert_eq!(
        owner_snapshot(&mut app)
            .game_ports
            .sample_ciaa_port_a_inputs()
            & 0x40,
        0x40
    );
}

#[test]
fn gamepad_plays_without_capture_survives_mouse_release_and_stops_outside_session() {
    use gui::player_config::{InputSource, NamedConfiguration};
    let mut configuration = NamedConfiguration::default();
    configuration.controls.ports[0] = configuration.controls.ports[1].clone();
    configuration.controls.ports[1].source = InputSource::Gamepad {
        identity: "pad".into(),
        mapping: "arrows".into(),
    };
    let (mut app, ctx) = running_with(ViewMode::ScreenOnly, false, configuration);
    let sender = app.host.runtime.as_ref().unwrap().input_sender();
    let generation = app.host.status.generation;
    delivered_after_input_read(&mut app, &ctx);
    pad(&sender, generation, false);
    pad(&sender, generation, true);
    delivered_after_input_read(&mut app, &ctx);
    assert!(observed_fire(&mut app));
    assert!(!app.player.captured);
    let _ = ctx.run_test_ui(egui::RawInput::default(), |ui| {
        egui::Window::new("Session control").show(ui.ctx(), |ui| {
            ui.button("Ordinary button").request_focus();
        });
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    delivered_after_input_read(&mut app, &ctx);
    assert!(
        observed_fire(&mut app),
        "A focused session button must not disable the gamepad"
    );
    window_keys(&mut app, &ctx, &[("ArrowUp", true, generation)]);
    delivered_after_input_read(&mut app, &ctx);
    assert_eq!(
        common::observe_input(&mut app, None).direction_pins[0] & 1,
        0
    );
    for captured in [true, false] {
        app.player.captured = captured;
        draw(&mut app, &ctx, None);
        delivered_after_input_read(&mut app, &ctx);
        assert!(
            observed_fire(&mut app),
            "Pointer capture changes must preserve controller contacts"
        );
    }
    assert_eq!(
        common::observe_input(&mut app, None).direction_pins[0] & 1,
        0
    );
    window_keys(&mut app, &ctx, &[("ArrowUp", false, generation)]);
    delivered_after_input_read(&mut app, &ctx);
    assert_eq!(
        common::observe_input(&mut app, None).direction_pins[0] & 1,
        1
    );
    assert!(observed_fire(&mut app));
    pad(&sender, generation.wrapping_add(1), false);
    delivered_after_input_read(&mut app, &ctx);
    assert!(
        observed_fire(&mut app),
        "Obsolete-generation controller samples must be discarded"
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !sender.try_host_event(
        gui::host_input::HostInputEvent::ReleaseMouse { port: 0 },
        generation,
    ) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    delivered_after_input_read(&mut app, &ctx);
    assert!(
        observed_fire(&mut app),
        "Browser capture-loss ingress must retain the held controller after capture is disabled"
    );
    let mut text = String::new();
    let text_id = egui::Id::unique("session-text");
    let _ = ctx.run_test_ui(egui::RawInput::default(), |ui| {
        egui::Window::new("Session editor").show(ui.ctx(), |ui| {
            ui.add(egui::TextEdit::singleline(&mut text).id(text_id))
                .request_focus();
        });
        {
            common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
            app.update_ui(ui)
        };
    });
    pad(&sender, generation, true);
    delivered_after_input_read(&mut app, &ctx);
    assert!(
        !observed_fire(&mut app),
        "Text editing must release and suppress gamepad contacts"
    );
    ctx.memory_mut(|memory| memory.surrender_focus(text_id));
    draw(&mut app, &ctx, None);
    common::settle(&mut app, &ctx);
    pad(&sender, generation, false);
    pad(&sender, generation, true);
    delivered_after_input_read(&mut app, &ctx);
    assert!(observed_fire(&mut app));
    app.player.screen = gui::player::PlayerScreen::Configuration;
    draw(&mut app, &ctx, None);
    pad(&sender, generation, true);
    delivered_after_input_read(&mut app, &ctx);
    assert!(!observed_fire(&mut app));
    app.player.screen = gui::player::PlayerScreen::Session;
    draw(&mut app, &ctx, None);
    pad(&sender, generation, true);
    delivered_after_input_read(&mut app, &ctx);
    assert!(
        !observed_fire(&mut app),
        "Held controls must not replay on return"
    );
    pad(&sender, generation, false);
    pad(&sender, generation, true);
    delivered_after_input_read(&mut app, &ctx);
    assert!(observed_fire(&mut app));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let _ = ctx.run_test_ui(
            egui::RawInput {
                focused: false,
                ..Default::default()
            },
            |ui| {
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui)
            },
        );
        pad(&sender, generation, true);
        if app.host.is_idle() && !observed_fire(&mut app) {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(!observed_fire(&mut app));
}

#[test]
fn physical_zoom_commands_update_gui_without_pausing_or_releasing_capture() {
    let (mut app, ctx) = running();
    let generation = app.host.status.generation;
    for (physical, zoom) in [
        ("Equal", 1.25),
        ("Minus", 1.0),
        ("NumpadAdd", 1.25),
        ("Numpad0", 1.0),
    ] {
        window_keys(
            &mut app,
            &ctx,
            &[
                ("ControlLeft", true, generation),
                (physical, true, generation),
                (physical, false, generation),
                ("ControlLeft", false, generation),
            ],
        );
        common::settle(&mut app, &ctx);
        assert_eq!(ctx.zoom_factor(), zoom);
        assert!(app.player.captured);
        assert!(app.host.status.running);
        assert_eq!(app.host.status.generation, generation);
    }
}

#[test]
fn stopped_developer_alt_t_toggles_recording_without_advancing_the_machine() {
    let (mut app, ctx) = running();
    common::command(&mut app, HostCommand::Pause);
    common::wait(&mut app, &ctx, |app| !app.host.status.running);
    app.set_view_mode(ViewMode::Developer);
    common::settle(&mut app, &ctx);
    let generation = app.host.status.generation;
    let cck = app.host.status.cck;
    let mut recording = common::inspect(&mut app).temporal.is_recording();
    for modifier in ["AltLeft", "AltRight"] {
        window_keys(
            &mut app,
            &ctx,
            &[
                (modifier, true, generation),
                ("KeyT", true, generation),
                ("KeyT", false, generation),
                (modifier, false, generation),
            ],
        );
        common::settle(&mut app, &ctx);
        recording = !recording;
        assert_eq!(common::inspect(&mut app).temporal.is_recording(), recording);
        assert!(!app.host.status.running);
        assert_eq!(app.host.status.cck, cck);
    }
}

fn ui_frame(
    app: &mut EmulatorApp,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> common::FrameOutput {
    ctx.run_test_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440.0, 1000.0),
            )),
            events,
            ..Default::default()
        },
        |ui| app.update_ui(ui),
    )
}

fn memory_search_position(app: &mut EmulatorApp, ctx: &egui::Context) -> egui::Pos2 {
    ctx.enable_accesskit();
    ui_frame(app, ctx, vec![]);
    let tree = ui_frame(app, ctx, vec![])
        .into_platform_output()
        .accesskit_update
        .unwrap();
    let field = tree
        .nodes
        .iter()
        .find_map(|(_, node)| {
            (node.placeholder() == Some("e.g. 4E 71 32 00")).then(|| node.bounds().unwrap())
        })
        .unwrap();
    egui::pos2(
        ((field.x0 + field.x1) / 2.0) as f32,
        ((field.y0 + field.y1) / 2.0) as f32,
    )
}

fn editor_click(position: egui::Pos2) -> Vec<egui::Event> {
    let mut events = vec![egui::Event::PointerMoved(position)];
    for pressed in [true, false] {
        events.push(egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    events
}

#[test]
fn a_consumed_quick_save_press_cannot_type_when_an_editor_gains_focus() {
    let (mut app, ctx) = running_with(ViewMode::ScreenOnly, false, Default::default());
    app.player
        .active_configuration
        .as_mut()
        .unwrap()
        .controls
        .host
        .frontend
        .save = "A".into();
    draw(&mut app, &ctx, None);
    let generation = app.host.status.generation;
    assert!(gui::keyboard_input::receive_key(
        &ctx,
        common::keyboard::key("KeyA", true),
        Some(generation),
    ));
    ui_frame(&mut app, &ctx, vec![]);
    common::wait(&mut app, &ctx, |app| app.host.status.quick_slots[0]);
    assert!(app.host.status.running);
    assert!(!guest_key_held(&mut app, 0x20));
    let first_save_cck = app.host.status.cck;
    common::wait(&mut app, &ctx, |app| app.host.status.cck > first_save_cck);

    app.load_target_addr_str.clear();
    let directory = tempfile::tempdir().unwrap();
    let binary_path = directory.path().join("fixture.bin");
    std::fs::write(&binary_path, [0x60, 0xfe]).unwrap();
    ctx.enable_accesskit();
    let _ = ctx.run_test_ui(
        egui::RawInput {
            dropped_files: vec![common::dropped_file(binary_path)],
            ..Default::default()
        },
        |ui| app.update_ui(ui),
    );
    assert!(app.load_binary_modal_open);
    let tree = ui_frame(&mut app, &ctx, vec![])
        .into_platform_output()
        .accesskit_update
        .unwrap();
    let field = tree
        .nodes
        .iter()
        .find_map(|(_, node)| {
            (node.role() == egui::accesskit::Role::TextInput).then(|| node.bounds().unwrap())
        })
        .unwrap();
    let position = egui::pos2(
        ((field.x0 + field.x1) / 2.0) as f32,
        ((field.y0 + field.y1) / 2.0) as f32,
    );
    gui::keyboard_input::receive_pointer_press(&ctx, position);
    ui_frame(&mut app, &ctx, editor_click(position));
    let focus = ctx.memory(|memory| memory.focused()).unwrap();
    assert!(egui::TextEdit::load_state(&ctx, focus).is_some());

    // Like the native adapter, forward a physical event and its text only if routing declines it.
    let mut forward = |key, physical, pressed, repeat, text: &str| {
        let mut input = common::keyboard::key(physical, pressed);
        input.repeat = repeat;
        let consumed = gui::keyboard_input::receive_key(&ctx, input, Some(generation));
        let mut events = Vec::new();
        if !consumed {
            events.push(egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed,
                repeat,
                modifiers: egui::Modifiers::NONE,
            });
            if pressed {
                events.push(egui::Event::Text(text.into()));
            }
        }
        ui_frame(&mut app, &ctx, events);
        consumed
    };
    let repeat_consumed = forward(egui::Key::A, "KeyA", true, true, "a");
    let release_consumed = forward(egui::Key::A, "KeyA", false, false, "a");
    assert!(!forward(egui::Key::B, "KeyB", true, false, "b"));
    assert!(!forward(egui::Key::B, "KeyB", false, false, "b"));
    common::settle(&mut app, &ctx);
    assert_eq!(app.load_target_addr_str, "b");
    assert!(repeat_consumed && release_consumed);
    assert!(!guest_key_held(&mut app, 0x20));
    assert!(!guest_key_held(&mut app, 0x35));
    assert_eq!(app.host.status.generation, generation);
    assert!(app.host.status.running);

    let reply = common::command(
        &mut app,
        HostCommand::LoadQuickSlot {
            slot: 1,
            ports: [
                debugger::launch::DeviceKind::Mouse,
                debugger::launch::DeviceKind::Joystick,
            ],
        },
    );
    assert!(reply.value.is_ok());
    assert!(
        reply.status.cck <= first_save_cck,
        "The repeat must not overwrite the quick save"
    );
}

#[test]
fn a_text_field_receives_the_first_character_in_the_same_batch_as_its_click() {
    let (mut app, ctx) = running_with(ViewMode::Developer, false, Default::default());
    common::command(&mut app, HostCommand::Pause);
    common::wait(&mut app, &ctx, |app| !app.host.status.running);
    app.player
        .active_configuration
        .as_mut()
        .unwrap()
        .controls
        .host
        .frontend
        .save = "A".into();
    let position = memory_search_position(&mut app, &ctx);
    let generation = app.host.status.generation;
    gui::keyboard_input::receive_pointer_press(&ctx, position);
    assert!(!gui::keyboard_input::receive_key(
        &ctx,
        common::keyboard::key("KeyA", true),
        Some(generation)
    ));
    let mut events = editor_click(position);
    events.push(egui::Event::Text("a".into()));
    ui_frame(&mut app, &ctx, events);
    assert_eq!(app.memory_search_state.query, "a");
    common::settle(&mut app, &ctx);
    assert!(!app.host.status.quick_slots[0]);
}

#[test]
fn an_open_binary_dialog_without_text_focus_suspends_gameplay_and_shortcuts() {
    let (mut app, ctx) = running_with(ViewMode::ScreenOnly, false, Default::default());
    let generation = app.host.status.generation;
    window_keys(&mut app, &ctx, &[("KeyA", true, generation)]);
    wait_guest_key(&mut app, &ctx, 0x20, true);
    app.load_binary_modal_open = true;
    draw(&mut app, &ctx, None);
    assert!(ctx.memory(|memory| memory.focused()).is_none());
    window_keys(
        &mut app,
        &ctx,
        &[
            ("F12", true, generation),
            ("F6", true, generation),
            ("KeyB", true, generation),
        ],
    );
    wait_guest_key(&mut app, &ctx, 0x20, false);
    assert_eq!(app.view_mode, ViewMode::ScreenOnly);
    assert!(!app.host.status.quick_slots[0]);
    assert!(!guest_key_held(&mut app, 0x35));
    app.load_binary_modal_open = false;
    draw(&mut app, &ctx, None);
    draw(&mut app, &ctx, None);
    let mut repeated = common::keyboard::key("KeyB", true);
    repeated.repeat = true;
    gui::keyboard_input::receive_key(&ctx, repeated, Some(generation));
    draw(&mut app, &ctx, None);
    delivered(&mut app, &ctx);
    assert!(!guest_key_held(&mut app, 0x35));
    window_keys(
        &mut app,
        &ctx,
        &[("KeyB", false, generation), ("KeyB", true, generation)],
    );
    wait_guest_key(&mut app, &ctx, 0x35, true);
    window_keys(&mut app, &ctx, &[("KeyB", false, generation)]);
    wait_guest_key(&mut app, &ctx, 0x35, false);
}

#[cfg(not(target_arch = "wasm32"))]
fn saved_progress_harness(
    configuration: gui::player_config::NamedConfiguration,
    library: &std::path::Path,
) -> egui_kittest::Harness<'static, EmulatorApp> {
    use gui::game_library::{GameLibrary, GameManifest};

    let (mut app, _) = running_with(ViewMode::ScreenOnly, true, configuration);
    // Associate the running fixture with a native game that has no saves.
    let directory = library.join("chooser");
    std::fs::create_dir(&directory).unwrap();
    let manifest = GameManifest {
        id: "chooser".into(),
        title: "Chooser test".into(),
        configuration: "standard".into(),
        disks: Vec::new(),
        mounted: [None, None, None, None],
        initial_state: None,
        start_states: Vec::new(),
        artwork: None,
        description: None,
    };
    std::fs::write(
        directory.join("game.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    app.player.active_game = Some(GameLibrary::discover(library).unwrap().games.remove(0));
    egui_kittest::Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(common::show_full_viewport, app)
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn saved_progress_chooser_blocks_session_input_until_fresh_input_after_close() {
    use egui_kittest::kittest::Queryable;
    use gui::host_input::HostInputEvent;
    use gui::player_config::{InputSource, NamedConfiguration};

    let mut configuration = NamedConfiguration::default();
    configuration.controls.ports[1].source = InputSource::Gamepad {
        identity: "pad".into(),
        mapping: "arrows".into(),
    };
    let library = tempfile::tempdir().unwrap();
    let mut h = saved_progress_harness(configuration, library.path());
    let ctx = h.ctx.clone();
    let generation = h.state().host.status.generation;
    let sender = h.state().host.runtime.as_ref().unwrap().input_sender();
    common::settle_harness(&mut h);
    assert!(h.state().player.captured, "{:?}", h.state().file_error);
    delivered(h.state_mut(), &ctx);
    pad(&sender, generation, false);
    pad(&sender, generation, true);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !sender.try_host_event(
        HostInputEvent::MouseButtons {
            port: 0,
            left: true,
            right: false,
            middle: false,
        },
        generation,
    ) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    queue_window_keys(&ctx, &[("KeyA", true, generation)]);
    delivered(h.state_mut(), &ctx);
    let before = owner_snapshot(h.state_mut());
    assert!(saved_key_held(&before, 0x20));
    assert_eq!(before.game_ports.sample_ciaa_port_a_inputs() & 0xc0, 0);
    assert!(h.state().player.captured);

    common::settle_harness(&mut h);
    let load = h.get_by_label("Load").rect().center();
    let mouse_button = |pressed| egui::Event::PointerButton {
        pos: load,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.input_mut()
        .events
        .extend([egui::Event::PointerMoved(load), mouse_button(true)]);
    h.step();
    h.input_mut().events.push(mouse_button(false));
    // Native ingress queues shortcuts before egui handles the click.
    queue_window_keys(&ctx, &[("F12", true, generation)]);
    h.step();
    assert!(h.state().player.show_saves);
    assert!(!h.state().player.captured);
    assert_eq!(h.state().view_mode, ViewMode::ScreenOnly);
    queue_window_keys(&ctx, &[("F12", false, generation)]);
    for physical in ["KeyB", "F12"] {
        assert!(
            !gui::keyboard_input::receive_key(
                &ctx,
                common::keyboard::key(physical, true),
                Some(generation)
            ),
            "{physical} still belongs to the session while Saved progress is open"
        );
    }
    h.get_by_label("Pause").click();
    h.step();
    delivered(h.state_mut(), &ctx);
    let suspended = owner_snapshot(h.state_mut());
    assert!(!saved_key_held(&suspended, 0x20));
    assert!(!saved_key_held(&suspended, 0x35));
    assert_eq!(
        suspended.game_ports.sample_ciaa_port_a_inputs() & 0xc0,
        0xc0
    );
    assert_eq!(h.state().view_mode, ViewMode::ScreenOnly);
    assert!(h.state().host.status.running);
    assert!(h.state().player.show_saves);

    h.get_by_label("Close").click();
    h.step();
    assert!(!h.state().player.show_saves);
    let mut repeat = common::keyboard::key("KeyB", true);
    repeat.repeat = true;
    gui::keyboard_input::receive_key(&ctx, repeat, Some(generation));
    pad(&sender, generation, true);
    delivered(h.state_mut(), &ctx);
    let closed = owner_snapshot(h.state_mut());
    assert!(!saved_key_held(&closed, 0x20));
    assert!(!saved_key_held(&closed, 0x35));
    assert_eq!(closed.game_ports.sample_ciaa_port_a_inputs() & 0xc0, 0xc0);
    assert!(!h.state().player.captured);

    queue_window_keys(
        &ctx,
        &[("KeyB", false, generation), ("KeyB", true, generation)],
    );
    pad(&sender, generation, false);
    pad(&sender, generation, true);
    delivered(h.state_mut(), &ctx);
    let resumed = owner_snapshot(h.state_mut());
    assert!(saved_key_held(&resumed, 0x35));
    assert_eq!(resumed.game_ports.sample_ciaa_port_a_inputs() & 0xc0, 0x40);
    assert_eq!(h.state().host.status.generation, generation);
    assert!(h.state().host.status.running);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gamepad_quick_slots_use_press_edges_and_discard_same_sample_pause_on_load() {
    use gui::player_config::{FrontendAction, GamepadShortcut, InputSource, NamedConfiguration};

    const QUICK_SAVE: usize = 4;
    const QUICK_LOAD: usize = 9;
    const PAUSE: usize = 8;

    let mut configuration = NamedConfiguration::default();
    configuration.controls.ports[1].source = InputSource::Gamepad {
        identity: "pad".into(),
        mapping: "arrows".into(),
    };
    configuration.controls.host.gamepad_shortcuts = [
        (QUICK_SAVE, FrontendAction::Save),
        (QUICK_LOAD, FrontendAction::Load),
        (PAUSE, FrontendAction::Pause),
    ]
    .map(|(button, action)| GamepadShortcut {
        identity: "pad".into(),
        button,
        action,
    })
    .into();
    let (app, _) = running_with(ViewMode::ScreenOnly, false, configuration);
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    common::settle_harness(&mut h);
    let generation = h.state().host.status.generation;
    let sender = h.state().host.runtime.as_ref().unwrap().input_sender();

    pad_buttons(&sender, generation, &[]);
    pad_buttons(&sender, generation, &[QUICK_SAVE]);
    common::wait_harness(&mut h, |app| app.host.status.quick_slots[0]);
    assert!(h.state().host.status.running);
    assert!(!h.state().player.show_saves);
    assert!(h.state().file_error.is_none());

    pad_buttons(&sender, generation, &[]);
    pad_buttons(&sender, generation, &[QUICK_LOAD, PAUSE]);
    common::wait_harness(&mut h, |app| app.host.status.generation > generation);
    let restored = h.state().host.status.generation;
    assert!(!h.state().host.status.running);
    assert!(!h.state().player.show_saves);
    assert!(h.state().file_error.is_none());

    pad_buttons(&sender, restored, &[QUICK_LOAD, PAUSE]);
    common::settle_harness(&mut h);
    assert_eq!(h.state().host.status.generation, restored);
    assert!(!h.state().host.status.running);
    pad_buttons(&sender, restored, &[]);
    pad_buttons(&sender, restored, &[PAUSE]);
    common::wait_harness(&mut h, |app| app.host.status.running);
}

#[test]
fn pending_keyboard_retry_overflow_across_frames_discards_backlog_and_releases_keys() {
    let (mut app, ctx) = running();
    let generation = app.host.status.generation;
    window_keys(&mut app, &ctx, &[("KeyB", true, generation)]);
    wait_guest_key(&mut app, &ctx, 0x35, true);
    // Hold the existing owner aside so GUI frames retain their pending submissions.
    let runtime = app.host.runtime.take().unwrap();
    let mut first = vec![
        ("CapsLock", true, generation),
        ("CapsLock", false, generation),
    ];
    for _ in 0..255 {
        first.extend([("KeyA", true, generation), ("KeyA", false, generation)]);
    }
    window_keys(&mut app, &ctx, &first);
    let mut second = Vec::new();
    for _ in 0..256 {
        second.extend([("KeyA", true, generation), ("KeyA", false, generation)]);
    }
    window_keys(&mut app, &ctx, &second);
    window_keys(
        &mut app,
        &ctx,
        &[("KeyA", true, generation), ("KeyA", false, generation)],
    );
    app.host.runtime = Some(runtime);
    draw(&mut app, &ctx, None);
    wait_guest_key(&mut app, &ctx, 0x35, false);
    assert!(!guest_key_held(&mut app, 0x20));
    assert!(
        !common::observe_input(&mut app, None).caps_lock_active,
        "Discarded Caps Lock press was replayed"
    );
    assert!(app.player.captured);
    assert!(app.host.status.running);
}
