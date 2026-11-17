#![allow(clippy::unwrap_used, clippy::panic)]
mod common;
use egui_kittest::kittest::NodeT;
use gui::host_commands::{HostCommand, HostValue};
use gui::{EmulatorApp, ViewMode};

fn running() -> (EmulatorApp, egui::Context) {
    let mut session = Box::new(gui::DebuggerSession::new());
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    let mut app = EmulatorApp::with_session(session);
    app.set_view_mode(ViewMode::ScreenOnly);
    let ctx = egui::Context::default();
    common::settle(&mut app, &ctx);
    common::command(&mut app, HostCommand::Run);
    app.player.active_configuration = Some(Default::default());
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("amiga-screen-input")));
    app.player.captured = true;
    draw(&mut app, &ctx, None);
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
    let _ = ctx.run(
        egui::RawInput {
            focused: true,
            events,
            ..Default::default()
        },
        |ctx| {
            common::keyboard::inject_taps(ctx, app.host.status.generation);
            app.update_ui(ctx)
        },
    );
}
#[test]
fn captured_tab_keeps_guest_focus_instead_of_navigating_host_controls() {
    let (mut app, ctx) = running();
    assert!(app.player.captured);
    draw(&mut app, &ctx, Some(egui::Key::Tab));
    assert!(ctx.memory(|memory| memory.has_focus(egui::Id::new("amiga-screen-input"))));
    assert!(app.player.captured);
}

fn window_keys(app: &mut EmulatorApp, ctx: &egui::Context, edges: &[(&str, bool, u64)]) {
    let raw: Vec<_> = edges
        .iter()
        .map(|(physical, pressed, generation)| {
            (
                (*physical).to_owned(),
                "different logical key".to_owned(),
                *pressed,
                false,
                false,
                *generation,
            )
        })
        .collect();
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("window_keyboard_events"), raw));
    draw(app, ctx, None);
}

fn guest_key_held(app: &mut EmulatorApp, code: usize) -> bool {
    let keyboard = serde_json::to_value(&owner_snapshot(app).keyboard).unwrap();
    keyboard["held"][code / 64].as_u64().unwrap() & (1 << (code % 64)) != 0
}

fn wait_guest_key(app: &mut EmulatorApp, ctx: &egui::Context, code: usize, held: bool) {
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
fn window_ingress_rejects_stale_generations_and_delivers_physical_hold_and_release() {
    let (mut app, ctx) = running();
    let generation = app.host.status.generation;
    // Reproduce a GUI capture request whose host activation has not yet applied.
    let sender = app.host.runtime.as_ref().unwrap().input_sender();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !sender.set_enabled(false, true) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    window_keys(
        &mut app,
        &ctx,
        &[
            ("KeyA", true, generation.wrapping_sub(1)),
            ("KeyB", true, generation),
        ],
    );
    wait_guest_key(&mut app, &ctx, 0x35, true);
    assert!(!guest_key_held(&mut app, 0x20), "Stale A reached the guest");
    assert!(app.player.captured);
    window_keys(&mut app, &ctx, &[("KeyB", false, generation)]);
    wait_guest_key(&mut app, &ctx, 0x35, false);
    assert!(app.player.captured);
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
        ],
    );
    wait_guest_key(&mut app, &ctx, 0x20, false);
    assert!(!app.player.captured);
    assert!(app.host.status.running);
    assert!(!guest_key_held(&mut app, 0x20));
    assert!(
        !guest_key_held(&mut app, 0x35),
        "B escaped focus-loss ordering"
    );

    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("amiga-screen-input")));
    app.player.captured = true;
    window_keys(&mut app, &ctx, &[("KeyA", true, generation)]);
    wait_guest_key(&mut app, &ctx, 0x20, true);
    let overflow = vec![("KeyB", true, generation); gui::host_input::INPUT_QUEUE_CAPACITY + 1];
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
    draw(&mut app, &ctx, Some(egui::Key::Enter));
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
        .build_state(
            |ctx, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ctx, app.host.status.generation);
                app.update_ui(ctx)
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
fn escape_and_f2_remain_guest_keys_and_optional_release_surrenders_focus() {
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
    draw(&mut app, &ctx, Some(egui::Key::F4));
    assert!(!app.player.captured);
    assert!(!ctx.memory(|m| m.has_focus(egui::Id::new("amiga-screen-input"))));
    assert!(app.host.status.running);
}

#[test]
fn middle_mouse_releases_capture_without_pause_or_keyboard_configuration() {
    let (mut app, ctx) = running();
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("capture_requested"), true));
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::PointerButton {
                pos: egui::pos2(400.0, 250.0),
                button: egui::PointerButton::Middle,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ctx| {
            common::keyboard::inject_taps(ctx, app.host.status.generation);
            app.update_ui(ctx)
        },
    );
    assert!(!app.player.captured);
    assert!(app.host.status.running);
    assert!(!ctx.memory(|m| m.has_focus(egui::Id::new("amiga-screen-input"))));
    draw(&mut app, &ctx, None);
    assert!(
        !app.player.captured,
        "Window activation must not recapture input"
    );
}

fn pad(sender: &gui::host_runtime::HostInputSender, generation: u64, pressed: bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let sample = gui::input_mapping::GamepadSample {
            identity: "pad".into(),
            name: "Test controller".into(),
            axes: vec![0.0, 0.0],
            buttons: vec![pressed; 16],
        };
        if sender.try_gamepads(vec![sample], generation, true) {
            return;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
}
fn delivered(app: &mut EmulatorApp, ctx: &egui::Context) {
    common::settle(app, ctx);
    // Establish an owner barrier after submission; cached GUI telemetry may predate it.
    let _ = fire(app);
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
fn owner_snapshot(app: &mut EmulatorApp) -> Box<debugger::A500State> {
    // Observe the owner without drawing a default, focused GUI frame.
    assert!(
        app.host.is_idle(),
        "GUI replies must be settled before observation"
    );
    let runtime = app.host.runtime.as_mut().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut command = HostCommand::SaveSnapshot;
    let request = loop {
        match runtime.submit(command) {
            Ok(request) => break request,
            Err(busy) => command = busy.0,
        }
        runtime.poll().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    loop {
        for reply in runtime.poll().unwrap() {
            if reply.id == request {
                let Ok(HostValue::Snapshot(json)) = reply.value else {
                    panic!("Missing machine snapshot");
                };
                let saved = debugger::SaveState::from_json(&json).unwrap();
                app.host.status = reply.status;
                return saved.state;
            }
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn gamepad_plays_without_capture_survives_mouse_release_and_stops_outside_session() {
    let (mut app, ctx) = running();
    app.player
        .active_configuration
        .as_mut()
        .unwrap()
        .controls
        .ports[1]
        .source = gui::player_config::InputSource::Gamepad {
        identity: "pad".into(),
        mapping: "arrows".into(),
    };
    app.player.captured = false;
    draw(&mut app, &ctx, None);
    let sender = app.host.runtime.as_ref().unwrap().input_sender();
    let generation = app.host.status.generation;
    delivered(&mut app, &ctx);
    pad(&sender, generation, false);
    pad(&sender, generation, true);
    delivered(&mut app, &ctx);
    assert!(fire(&mut app));
    assert!(!app.player.captured);
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::Window::new("Session control").show(ctx, |ui| {
            ui.button("Ordinary button").request_focus();
        });
        {
            common::keyboard::inject_taps(ctx, app.host.status.generation);
            app.update_ui(ctx)
        };
    });
    delivered(&mut app, &ctx);
    assert!(
        fire(&mut app),
        "A focused session button must not disable the gamepad"
    );
    for captured in [true, false] {
        app.player.captured = captured;
        draw(&mut app, &ctx, None);
        delivered(&mut app, &ctx);
        assert!(
            fire(&mut app),
            "Pointer capture changes must preserve controller contacts"
        );
    }
    pad(&sender, generation.wrapping_add(1), false);
    delivered(&mut app, &ctx);
    assert!(
        fire(&mut app),
        "Obsolete-generation controller samples must be discarded"
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !sender.try_host_event(
        gui::host_input::HostInputEvent::ReleaseCapture {
            joysticks: [None; 2],
        },
        generation,
    ) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    delivered(&mut app, &ctx);
    assert!(
        fire(&mut app),
        "Browser capture-loss ingress must retain the held controller after capture is disabled"
    );
    let mut text = String::new();
    let text_id = egui::Id::new("session-text");
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::Window::new("Session editor").show(ctx, |ui| {
            ui.add(egui::TextEdit::singleline(&mut text).id(text_id))
                .request_focus();
        });
        {
            common::keyboard::inject_taps(ctx, app.host.status.generation);
            app.update_ui(ctx)
        };
    });
    pad(&sender, generation, true);
    delivered(&mut app, &ctx);
    assert!(
        !fire(&mut app),
        "Text editing must release and suppress gamepad contacts"
    );
    ctx.memory_mut(|memory| memory.surrender_focus(text_id));
    draw(&mut app, &ctx, None);
    common::settle(&mut app, &ctx);
    pad(&sender, generation, false);
    pad(&sender, generation, true);
    delivered(&mut app, &ctx);
    assert!(fire(&mut app));
    app.player.screen = gui::player::PlayerScreen::Configuration;
    draw(&mut app, &ctx, None);
    pad(&sender, generation, true);
    delivered(&mut app, &ctx);
    assert!(!fire(&mut app));
    app.player.screen = gui::player::PlayerScreen::Session;
    draw(&mut app, &ctx, None);
    pad(&sender, generation, true);
    delivered(&mut app, &ctx);
    assert!(!fire(&mut app), "Held controls must not replay on return");
    pad(&sender, generation, false);
    pad(&sender, generation, true);
    delivered(&mut app, &ctx);
    assert!(fire(&mut app));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let _ = ctx.run(
            egui::RawInput {
                focused: false,
                ..Default::default()
            },
            |ctx| {
                common::keyboard::inject_taps(ctx, app.host.status.generation);
                app.update_ui(ctx)
            },
        );
        pad(&sender, generation, true);
        if app.host.is_idle() && !fire(&mut app) {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(!fire(&mut app));
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
    let keyboard = serde_json::to_value(&owner_snapshot(&mut app).keyboard).unwrap();
    assert_eq!(
        keyboard["caps_lock_active"], false,
        "Discarded Caps Lock press was replayed"
    );
    assert!(app.player.captured);
    assert!(app.host.status.running);
}
