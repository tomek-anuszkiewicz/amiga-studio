#![allow(clippy::unwrap_used, clippy::panic)]
mod common;
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
        |ctx| app.update_ui(ctx),
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
#[test]
fn enter_bound_to_release_does_not_recapture_through_a_keyboard_canvas_click() {
    let (mut app, ctx) = running();
    app.player
        .active_configuration
        .as_mut()
        .unwrap()
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
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    common::settle_harness(&mut harness);
    let save = harness.get_by_label("Save").raw_bounds().unwrap();
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
        |ctx| app.update_ui(ctx),
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
    let before = app.host.status.input_deliveries;
    common::wait(app, ctx, |app| {
        app.host.status.input_deliveries >= before + 2
    });
}
fn fire(app: &mut EmulatorApp) -> bool {
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
                app.host.status = runtime.status();
                return saved.state.game_ports.sample_ciaa_port_a_inputs() & 0x80 == 0;
            }
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn gamepad_plays_without_capture_survives_mouse_release_and_stops_outside_session() {
    let (mut app, ctx) = running();
    app.player.active_configuration.as_mut().unwrap().ports[1].source =
        gui::player_config::InputSource::Gamepad {
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
        app.update_ui(ctx);
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
        app.update_ui(ctx);
    });
    pad(&sender, generation, true);
    delivered(&mut app, &ctx);
    assert!(
        !fire(&mut app),
        "Text editing must release and suppress gamepad contacts"
    );
    ctx.memory_mut(|memory| memory.surrender_focus(text_id));
    draw(&mut app, &ctx, None);
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
            |ctx| app.update_ui(ctx),
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
