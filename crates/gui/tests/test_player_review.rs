#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]
mod common;
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
        draft.save(root).unwrap()
    };
    let mut app = EmulatorApp::default();
    app.player.catalog.library_root = root.to_string_lossy().into_owned();
    app.player.catalog.configurations[0].rom = rom_path.to_string_lossy().into_owned();
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
        .build_state(
            |ctx, app: &mut EmulatorApp| {
                app.update_ui(ctx);
                if ctx.output(|out| out.num_completed_passes) == 0 {
                    ctx.request_discard("repeat confirmation layout");
                }
            },
            app,
        );
    common::settle_harness(&mut harness);
    harness
}
#[test]
fn returning_from_configuration_applies_new_host_bindings_without_replacing_hardware() {
    let root = root();
    let mut app = started(&root);
    app.navigate_player(PlayerScreen::Configuration);
    common::settle(&mut app, &egui::Context::default());
    let before = common::snapshot(&mut app);
    app.player.active_configuration.as_mut().unwrap().ports[1].source = InputSource::Gamepad {
        identity: "old".into(),
        mapping: "arrows".into(),
    };
    let edited = &mut app.player.catalog.configurations[0];
    edited.ports[1].source = InputSource::Gamepad {
        identity: "new".into(),
        mapping: "arrows".into(),
    };
    edited.keyboard = vec![(0x20, 0x21)];
    edited.frontend.release = "F3".into();
    edited.mouse_sensitivity = 1.5;
    edited.preset = config::A500Preset::ExpandedPowerUser;
    app.player.gamepads = vec![gui::input_mapping::GamepadInfo {
        identity: "new".into(),
        name: "Reconnected controller".into(),
    }];
    app.navigate_player(PlayerScreen::Session);
    let active = app.player.active_configuration.as_ref().unwrap();
    assert!(matches!(&active.ports[1].source,InputSource::Gamepad{identity,..} if identity=="new"));
    assert_eq!(active.keyboard, vec![(0x20, 0x21)]);
    assert_eq!(active.frontend.release, "F3");
    assert_eq!(active.mouse_sensitivity, 1.5);
    assert_eq!(active.preset, config::A500Preset::Standard1Mb);
    common::settle(&mut app, &egui::Context::default());
    assert_eq!(common::snapshot(&mut app), before);
}
#[test]
fn editing_native_rom_reference_does_not_boot_cached_previous_rom() {
    let root = root();
    let mut app = EmulatorApp::default();
    let path = root.join("new.rom");
    std::fs::write(&path, rom(0x22)).unwrap();
    app.player.rom = Some(rom(0x11));
    app.player.rom_configuration_id = Some("standard".into());
    app.player.catalog.configurations[0].rom = path.to_string_lossy().into_owned();
    app.start_manual();
    common::settle(&mut app, &egui::Context::default());
    common::command(&mut app, HostCommand::Pause);
    common::wait(&mut app, &egui::Context::default(), |app| {
        !app.host.status.running
    });
    let state = common::snapshot(&mut app);
    assert_eq!(state.physical_memory.read_byte_debug(0xfc0200), 0x22);
}
#[test]
fn library_start_requires_confirmation_and_cancel_preserves_the_current_session() {
    let root = root();
    let mut app = started(&root);
    app.player.library = gui::game_library::GameLibrary::discover(&root).unwrap();
    app.player.selected_game = Some(0);
    app.navigate_player(PlayerScreen::Library);
    let mut h = harness(app);
    let before = common::snapshot(h.state_mut());
    let generation = h.state().host.status.generation;
    h.get_by_label("Start game").click();
    common::settle_harness(&mut h);
    assert_eq!(
        h.state().host.status.generation,
        generation,
        "Start replaced the machine before confirmation"
    );
    h.get_by_label("Keep current session").click();
    common::settle_harness(&mut h);
    assert_eq!(common::snapshot(h.state_mut()), before);
    h.get_by_label("Start game").click();
    common::settle_harness(&mut h);
    h.get_by_label("Replace session").click();
    common::settle_harness(&mut h);
    assert!(h.state().host.status.generation > generation);
}
fn open_slot_menu(h: &mut Harness<'_, EmulatorApp>) {
    h.get_by_label("State").click();
    common::settle_harness(h);
    h.get_by_label("Slots 2–5").hover();
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
    h.get_by_label("Save #4").click();
    common::wait_harness(&mut h, |app| {
        !app.player.busy && !app.player.saves.is_empty()
    });
    drop(h);
    let app = started(&root);
    assert!(!app.host.status.quick_slots[3]);
    let mut h = harness(app);
    open_slot_menu(&mut h);
    h.get_by_label("Load #4").click();
    common::settle_harness(&mut h);
    h.get_by_label("Replace session").click();
    common::settle_harness(&mut h);
    common::command(h.state_mut(), HostCommand::Pause);
    common::wait_harness(&mut h, |app| !app.host.status.running);
    assert_eq!(
        common::snapshot(h.state_mut())
            .physical_memory
            .read_word_debug(0xc00100),
        0xcafe
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
        let _ = ctx.run(
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
            |ctx| {
                egui::Window::new("Host editor").show(ctx, |ui| {
                    ui.add(egui::TextEdit::singleline(&mut text).id(egui::Id::new("host-editor")))
                        .request_focus();
                });
                app.update_ui(ctx);
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
        common::snapshot(h.state_mut())
            .physical_memory
            .read_byte_debug(0xfc0200),
        0x11,
        "Direct ROM loading replaced the machine before confirmation"
    );
    h.get_by_label("Keep current session").click();
    common::settle_harness(&mut h);
    assert_eq!(
        common::snapshot(h.state_mut())
            .physical_memory
            .read_byte_debug(0xfc0200),
        0x11
    );
    h.state_mut().load_rom_file(&path);
    common::settle_harness(&mut h);
    h.get_by_label("Replace session").click();
    common::settle_harness(&mut h);
    assert_eq!(
        common::snapshot(h.state_mut())
            .physical_memory
            .read_byte_debug(0xfc0200),
        0x22
    );
}
