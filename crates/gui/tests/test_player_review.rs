#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]
mod common;
use common::TestContext as _;
use egui_kittest::{kittest::Queryable, Harness};
use gui::host_commands::HostCommand;
use gui::player::PlayerScreen;
use gui::player_config::InputSource;
use gui::{EmulatorApp, ViewMode};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.agent/tmp/player-review")
        .join(format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
    std::fs::create_dir_all(&path).unwrap();
    path
}
fn rom(marker: u8) -> Vec<u8> {
    let mut bytes = vec![0; 262144];
    bytes[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    bytes[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    bytes[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    bytes[0x200] = marker;
    bytes
}
fn started(root: &Path) -> EmulatorApp {
    let rom_path = root.join("kick.rom");
    std::fs::write(&rom_path, rom(0x11)).unwrap();
    let mut draft = gui::library_edit::GameDraft::new("standard".into());
    draft.name = "Review game".into();
    let game = if root.join(&draft.name).exists() {
        gui::game_library::GameLibrary::discover(root)
            .unwrap()
            .games
            .remove(0)
    } else {
        draft.save(root, [true; 4]).unwrap().game
    };
    let mut app = EmulatorApp::default();
    app.player.library_root = Some(root.to_path_buf());
    app.player.catalog.configurations[0].launch.rom = "kick.rom".into();
    app.start_library_game(game, None, false);
    common::settle(&mut app, &egui::Context::default());
    common::command(&mut app, HostCommand::Pause);
    common::wait(&mut app, &egui::Context::default(), |app| {
        !app.host.status.running
    });
    app
}
fn harness(app: EmulatorApp) -> Harness<'static, EmulatorApp> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, app: &mut EmulatorApp| {
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui);
                if ui.ctx().output(|out| out.num_completed_passes) == 0 {
                    ui.ctx().request_discard("repeat confirmation layout");
                }
            },
            app,
        );
    common::settle_harness(&mut harness);
    harness
}
#[test]
fn returning_from_configuration_applies_new_host_bindings_without_replacing_hardware() {
    use gui::input_mapping::GamepadSample;

    let root = root();
    let mut app = started(&root);
    let ctx = egui::Context::default();
    app.navigate_player(PlayerScreen::Configuration);
    common::settle(&mut app, &ctx);
    let before = common::snapshot(&mut app);
    app.player
        .active_configuration
        .as_mut()
        .unwrap()
        .controls
        .ports[1]
        .source = InputSource::Gamepad {
        identity: "old".into(),
        mapping: "arrows".into(),
    };
    let edited = &mut app.player.editor.as_mut().unwrap().catalog.configurations[0];
    edited.controls.ports[1].source = InputSource::Gamepad {
        identity: "new".into(),
        mapping: "arrows".into(),
    };
    edited.controls.host.keyboard = vec![(0x20, 0x21)];
    edited.controls.host.frontend.release = "F3".into();
    edited.controls.host.mouse_sensitivity = 1.5;
    edited.launch.preset = config::A500Preset::ExpandedPowerUser;
    let sender = app.host.runtime.as_ref().unwrap().input_sender();
    common::wait(&mut app, &ctx, |app| {
        if app.player.gamepads.iter().any(|pad| pad.identity == "new") {
            return true;
        }
        sender.try_gamepads(
            vec![GamepadSample {
                identity: "new".into(),
                name: "Reconnected controller".into(),
                axes: vec![0.0; 2],
                buttons: vec![false; 16],
            }],
            app.host.status.generation,
            true,
        );
        false
    });
    app.save_configuration_edit();
    common::wait(&mut app, &ctx, |app| {
        app.player.editor.is_none() && app.player.screen == PlayerScreen::Session
    });
    app.navigate_player(PlayerScreen::Session);
    let active = app.player.active_configuration.as_ref().unwrap();
    assert!(
        matches!(&active.controls.ports[1].source,InputSource::Gamepad{identity,..} if identity=="new")
    );
    assert_eq!(active.controls.host.keyboard, vec![(0x20, 0x21)]);
    assert_eq!(active.controls.host.frontend.release, "F3");
    assert_eq!(active.controls.host.mouse_sensitivity, 1.5);
    assert_eq!(active.launch.preset, config::A500Preset::Standard1Mb);
    common::settle(&mut app, &ctx);
    assert_eq!(common::snapshot(&mut app), before);
}

#[test]
fn imported_save_uses_active_identity_and_current_controls_in_developer_studio() {
    let root = root();
    let mut app = started(&root);
    let saved = common::snapshot_json(&mut app);
    let original = common::snapshot(&mut app);
    let path = root.join("progress.json");
    std::fs::write(&path, &saved).unwrap();
    app.navigate_player(PlayerScreen::Configuration);
    common::settle(&mut app, &egui::Context::default());
    let draft = &mut app.player.editor.as_mut().unwrap().catalog;
    draft.configurations[0].controls.ports.swap(0, 1);
    draft.configurations[0].controls.host.keyboard = vec![(0x20, 0x21)];
    draft
        .configurations
        .push(gui::player_config::NamedConfiguration {
            id: "future".into(),
            ..Default::default()
        });
    draft.selected = "future".into();
    app.save_configuration_edit();
    common::settle(&mut app, &egui::Context::default());
    app.set_view_mode(ViewMode::Developer);
    common::settle(&mut app, &egui::Context::default());
    let controls = app
        .player
        .active_configuration
        .as_ref()
        .unwrap()
        .controls
        .clone();
    app.load_state_file(&path);
    common::settle(&mut app, &egui::Context::default());
    assert!(app.file_error.is_none(), "{:?}", app.file_error);
    assert_eq!(
        app.player.active_configuration.as_ref().unwrap().id,
        "standard"
    );
    assert_eq!(
        app.player.active_configuration.as_ref().unwrap().controls,
        controls
    );
    assert_eq!(
        app.host.status.ports,
        [
            debugger::launch::DeviceKind::Joystick,
            debugger::launch::DeviceKind::Mouse
        ]
    );
    let restored = common::snapshot(&mut app);
    assert_eq!(restored.cpu, original.cpu);
    assert_eq!(restored.cck, original.cck);
    assert_eq!(restored.physical_memory, original.physical_memory);
    assert_eq!(restored.floppy, original.floppy);
}

#[test]
fn gamepad_save_preserves_capture_generation_and_held_contacts() {
    use gui::host_input::HostInputEvent;
    use gui::input_mapping::GamepadSample;
    use gui::player_config::{FrontendAction, GamepadShortcut};
    let root = root();
    let mut app = started(&root);
    let ctx = egui::Context::default();
    let sender = app.host.runtime.as_ref().unwrap().input_sender();
    let sample = |generation, buttons: Vec<bool>, axis: f32| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !sender.try_gamepads(
            vec![GamepadSample {
                identity: "save-pad".into(),
                name: "Save controller".into(),
                axes: vec![axis, 0.0],
                buttons: buttons.clone(),
            }],
            generation,
            true,
        ) {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
    };
    sample(app.host.status.generation, vec![false; 16], 0.0);
    app.navigate_player(PlayerScreen::Configuration);
    common::wait(&mut app, &ctx, |app| {
        app.player
            .gamepads
            .iter()
            .any(|pad| pad.identity == "save-pad")
    });
    let controls = &mut app.player.editor.as_mut().unwrap().catalog.configurations[0].controls;
    controls.ports[1].source = InputSource::Gamepad {
        identity: "save-pad".into(),
        mapping: "arrows".into(),
    };
    controls.host.gamepad_shortcuts.push(GamepadShortcut {
        identity: "save-pad".into(),
        button: 9,
        action: FrontendAction::Save,
    });
    controls.host.gamepad_shortcuts.push(GamepadShortcut {
        identity: "save-pad".into(),
        button: 8,
        action: FrontendAction::Release,
    });
    app.save_configuration_edit();
    common::settle(&mut app, &ctx);
    assert!(app.player.editor.is_none());
    common::command(&mut app, HostCommand::Run);
    common::wait(&mut app, &ctx, |app| app.host.status.running);
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::unique("amiga-screen-input")));
    app.player.captured = true;
    common::settle(&mut app, &ctx);
    let generation = app.host.status.generation;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !sender.set_enabled(true, true, true) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    let activation = app.host.status.input_deliveries;
    common::wait(&mut app, &ctx, |app| {
        app.host.status.input_deliveries >= activation + 2
    });
    sample(generation, vec![false; 16], 0.0);
    let mut held = vec![false; 16];
    held[0] = true;
    sample(generation, held.clone(), 1.0);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !sender.try_host_event(
        HostInputEvent::MouseButtons {
            port: 0,
            left: true,
            right: true,
            middle: false,
        },
        generation,
    ) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    let deliveries = app.host.status.input_deliveries;
    common::wait(&mut app, &ctx, |app| {
        app.host.status.input_deliveries >= deliveries + 2
    });
    assert!(app.player.captured);
    let before = common::observe_input(&mut app, None);
    assert!(
        before.fire1_pressed[0],
        "mouse input must be held before Save"
    );
    assert!(
        before.fire1_pressed[1],
        "joystick input must be held before Save"
    );
    let previous_saves = app.player.saves.len();
    held[9] = true;
    sample(generation, held.clone(), 1.0);
    common::wait(&mut app, &ctx, |app| {
        !app.player.busy && app.player.saves.len() == previous_saves + 1
    });
    assert!(app.player.captured);
    assert!(app.host.status.running);
    assert_eq!(app.host.status.generation, generation);
    let save_path = &app.player.saves[0].path;
    let slot_name = std::fs::read_to_string(save_path.with_file_name("slot-1.txt"))
        .expect("Controller Quick save did not publish slot 1");
    assert_eq!(
        slot_name.trim(),
        save_path.file_name().unwrap().to_str().unwrap(),
        "Quick slot 1 must point to the new save"
    );
    let json = std::fs::read_to_string(save_path).unwrap();
    let saved = debugger::SaveState::from_json(&json).unwrap();
    assert!(saved.game_ports().fire1_port1());
    assert!(saved.game_ports().fire1_port2());
    assert_ne!(saved.game_ports().pins.direction_pins[1], 0xf);
    held[9] = false;
    sample(generation, held.clone(), 1.0);
    held[8] = true;
    sample(generation, held, 1.0);
    common::wait(&mut app, &ctx, |app| !app.player.captured);
    assert_eq!(app.host.status.generation, generation);
}
#[test]
fn editing_native_rom_reference_does_not_boot_cached_previous_rom() {
    let root = root();
    let mut app = EmulatorApp::default();
    let path = root.join("new.rom");
    std::fs::write(&path, rom(0x22)).unwrap();
    app.player.rom = Some(rom(0x11));
    app.player.rom_configuration_id = Some("standard".into());
    app.player.library_root = Some(root);
    app.player.catalog.configurations[0].launch.rom = "new.rom".into();
    app.start_manual();
    common::settle(&mut app, &egui::Context::default());
    common::command(&mut app, HostCommand::Pause);
    common::wait(&mut app, &egui::Context::default(), |app| {
        !app.host.status.running
    });
    assert_eq!(common::read_memory(&mut app, 0xfc0200, 1), [0x22]);
}
#[test]
fn library_start_requires_confirmation_and_cancel_preserves_the_current_session() {
    let root = root();
    let mut app = started(&root);
    app.player.selected_game = app
        .player
        .active_game
        .as_ref()
        .map(|game| game.manifest.id.clone());
    app.navigate_player(PlayerScreen::Library);
    let mut h = harness(app);
    common::wait_harness(&mut h, |app| app.player.library.games.len() == 1);
    let before = common::snapshot(h.state_mut());
    let generation = h.state().host.status.generation;
    h.get_by_label("Start game").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(
        h.state().host.status.generation,
        generation,
        "Start replaced the machine before confirmation"
    );
    h.get_by_label("Keep current session").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(common::snapshot(h.state_mut()), before);
    h.get_by_label("Start game").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Replace session").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.state().host.status.generation > generation);
}
fn open_slot_menu(h: &mut Harness<'_, EmulatorApp>) {
    h.get_by_label("State").click();
    common::settle_harness(h);
    h.get_by_label_contains("Slots 2–5").hover();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while h.query_by_label("Save #4").is_none() {
        h.step();
        assert!(
            std::time::Instant::now() < deadline,
            "Slots submenu did not open"
        );
    }
}

#[test]
fn durable_slot_four_is_loadable_from_the_menu_after_reopening() {
    let root = root();
    let mut app = started(&root);
    common::command(
        &mut app,
        HostCommand::WriteMemory {
            address: 0xc00100,
            bytes: vec![0xca, 0xfe],
            restart_instruction: false,
        },
    );
    let mut h = harness(app);
    open_slot_menu(&mut h);
    h.get_by_label("Save #4").click_accesskit();
    common::wait_harness(&mut h, |app| {
        !app.player.busy && !app.player.saves.is_empty()
    });
    drop(h);
    let app = started(&root);
    assert!(!app.host.status.quick_slots[3]);
    let mut h = harness(app);
    open_slot_menu(&mut h);
    h.get_by_label("Load #4").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Replace session").click_accesskit();
    common::settle_harness(&mut h);
    common::command(h.state_mut(), HostCommand::Pause);
    common::wait_harness(&mut h, |app| !app.host.status.running);
    assert_eq!(
        common::read_memory(h.state_mut(), 0xc00100, 2),
        [0xca, 0xfe]
    );
}
#[test]
fn focused_host_text_prevents_frontend_shortcuts() {
    let mut app = EmulatorApp::default();
    let ctx = egui::Context::default();
    common::settle(&mut app, &ctx);
    let mut text = String::new();
    for key in [
        egui::Key::F5,
        egui::Key::F6,
        egui::Key::F9,
        egui::Key::F2,
        egui::Key::F12,
    ] {
        let _ = ctx.run_test_ui(egui::RawInput::default(), |ui| {
            egui::Window::new("Host editor").show(ui.ctx(), |ui| {
                ui.add(egui::TextEdit::singleline(&mut text).id(egui::Id::unique("host-editor")))
                    .request_focus();
            });
            app.update_ui(ui);
        });
        let _ = ctx.run_test_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| {
                egui::Window::new("Host editor").show(ui.ctx(), |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut text).id(egui::Id::unique("host-editor")),
                    )
                    .request_focus();
                });
                common::keyboard::inject_taps(ui.ctx(), app.host.status.generation);
                app.update_ui(ui);
            },
        );
        common::settle(&mut app, &ctx);
        assert!(!app.host.status.running, "{key:?} escaped host text focus");
        assert_eq!(app.view_mode, ViewMode::Developer);
        assert!(!app.host.status.quick_slots[0]);
        assert!(app.file_error.is_none());
    }
}

#[test]
fn direct_rom_replacement_requires_confirmation_and_can_be_cancelled() {
    let root = root();
    let mut app = started(&root);
    let path = root.join("replacement.rom");
    std::fs::write(&path, rom(0x22)).unwrap();
    app.load_rom_file(&path);
    let mut h = harness(app);
    assert_eq!(
        common::read_memory(h.state_mut(), 0xfc0200, 1),
        [0x11],
        "Direct ROM loading replaced the machine before confirmation"
    );
    h.get_by_label("Keep current session").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(common::read_memory(h.state_mut(), 0xfc0200, 1), [0x11]);
    h.state_mut().load_rom_file(&path);
    common::settle_harness(&mut h);
    h.get_by_label("Replace session").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(common::read_memory(h.state_mut(), 0xfc0200, 1), [0x22]);
}
