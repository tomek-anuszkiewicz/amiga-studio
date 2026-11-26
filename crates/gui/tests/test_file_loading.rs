#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use egui_kittest::{kittest::Queryable, Harness};
use gui::host_commands::HostCommand;
use gui::player::PlayerScreen;
use gui::EmulatorApp;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.agent/tmp");
    std::fs::create_dir_all(&directory).unwrap();
    directory.join(format!("kickstart-{name}-{}", std::process::id()))
}

#[test]
fn invalid_rom_displays_a_dismissible_error_and_can_be_retried() {
    let library = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let path = source.path().join("load.rom");
    std::fs::write(&path, vec![0x55; 524288]).unwrap();
    let mut app = EmulatorApp::default();
    app.player.library_root = Some(library.path().to_owned());
    app.navigate_player(PlayerScreen::Configuration);
    let before = common::snapshot(&mut app);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    let error = harness.state_mut().import_kickstart(&path).unwrap_err();
    common::settle_harness(&mut harness);
    assert_eq!(common::snapshot(harness.state_mut()), before);
    assert!(error.contains("expected 262144 bytes (256 KiB), got 524288 bytes"));
    assert!(!library.path().join("kickstarts").exists());
    harness.get_by_label("File operation failed");
    assert_eq!(harness.get_all_by_label(&error).count(), 2);
    common::settle_harness(&mut harness);
    assert_eq!(harness.state().file_error.as_ref(), Some(&error));

    harness.get_by_label("Close").click_accesskit();
    common::settle_harness(&mut harness);
    assert!(harness.state().file_error.is_none());
    let rom = boot_rom();
    std::fs::write(&path, &rom).unwrap();
    harness.state_mut().import_kickstart(&path).unwrap();
    common::settle_harness(&mut harness);
    assert_eq!(common::snapshot(harness.state_mut()), before);
    assert_eq!(
        std::fs::read(library.path().join("kickstarts/load.rom")).unwrap(),
        rom
    );
    common::choose_manual_kickstart(&mut harness, "load.rom");
    let generation = harness.state().host.status.generation;
    harness.get_by_label("Start").click_accesskit();
    common::wait_harness(&mut harness, |app| app.host.status.running);
    assert_eq!(harness.state().host.status.generation, generation + 1);
    harness.get_by_label("Pause").click_accesskit();
    common::wait_harness(&mut harness, |app| !app.host.status.running);
    assert!(harness.state().file_error.is_none());
    assert_eq!(
        common::inspect(harness.state_mut()).memory.kickstart_rom(),
        rom
    );
}

#[test]
fn invalid_snapshot_sources_display_a_dismissible_error_without_restoring() {
    let path = fixture_path("state.json");
    let mut app = EmulatorApp::default();
    let before = common::snapshot(&mut app);
    let mut source = gui::DebuggerSession::new();
    let rom = vec![0x55; physical_memory::KICKSTART_ROM_SIZE];
    source.load_rom(&rom).unwrap();
    source.machine.cpu.state.set_d_long(7, 0xcafe_beef);
    let json = source.save_state_to_json().unwrap();
    std::fs::write(&path, &json).unwrap();
    let ctx = egui::Context::default();
    let generation = app.host.status.generation;
    for size in [0, 262143, 524288, 262144] {
        app.player.restore_kickstart = Some(debugger::SourceImage {
            name: None,
            bytes: vec![0x11; size],
        });
        app.load_state_file(&path);
        common::settle(&mut app, &ctx);
        assert_eq!(common::snapshot(&mut app), before);
        assert_eq!(app.host.status.generation, generation);
        let error = app.file_error.take().unwrap();
        assert!(error.contains("Kickstart ROM"), "{error}");
        if size == 262144 {
            assert!(error.contains("content does not match"), "{error}");
        } else {
            assert!(error.contains("262144 bytes"), "{error}");
            assert!(error.contains(&format!("received {size} bytes")), "{error}");
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), json);
    }

    app.player.restore_kickstart = Some(debugger::SourceImage {
        name: None,
        bytes: rom,
    });
    let mut broken: serde_json::Value = serde_json::from_str(&json).unwrap();
    broken["state"]["physical_memory"]["kickstart_rom"]["sha256"] = "bad digest".into();
    std::fs::write(&path, broken.to_string()).unwrap();
    app.load_state_file(&path);
    common::settle(&mut app, &ctx);
    assert_eq!(common::snapshot(&mut app), before);
    let error = app.file_error.clone().unwrap();
    assert!(error.contains("SHA-256 digest"));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    common::settle_harness(&mut harness);
    harness.get_by_label(&error);
    harness.get_by_label("Close").click_accesskit();
    common::settle_harness(&mut harness);
    assert!(harness.state().file_error.is_none());
    assert_eq!(common::snapshot(harness.state_mut()), before);

    std::fs::write(&path, &json).unwrap();
    let generation = harness.state().host.status.generation;
    harness.state_mut().load_state_file(&path);
    common::settle_harness(&mut harness);
    assert!(harness.state().file_error.is_none());
    assert!(harness.state().host.status.generation > generation);
    assert_eq!(
        common::snapshot(harness.state_mut()),
        source.machine.save_state()
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn firmware_start_boots_reset_vectors_and_preserves_the_active_rom_on_failed_import() {
    let library = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let path = source.path().join("boot.rom");
    let invalid_path = source.path().join("invalid.rom");
    std::fs::write(&invalid_path, [0; 1024]).unwrap();
    let rom = boot_rom();
    std::fs::write(&path, &rom).unwrap();
    let mut app = common::app_with_program(&[0x60, 0xfe]);
    app.player.library_root = Some(library.path().to_owned());
    app.navigate_player(PlayerScreen::Configuration);
    assert!(common::command(&mut app, HostCommand::StepCck)
        .value
        .is_ok());
    let before = common::snapshot(&mut app);
    app.import_kickstart(&path).unwrap();
    assert_eq!(common::snapshot(&mut app), before);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    common::settle_harness(&mut harness);
    common::choose_manual_kickstart(&mut harness, "boot.rom");
    let generation = harness.state().host.status.generation;
    harness.get_by_label("Start").click_accesskit();
    common::settle_harness(&mut harness);
    assert_eq!(common::snapshot(harness.state_mut()), before);
    harness.get_by_label("Replace session").click_accesskit();
    common::wait_harness(&mut harness, |app| app.host.status.field_number > 0);
    let app = harness.state_mut();
    assert!(app.host.status.running);
    assert_eq!(app.host.status.generation, generation + 1);
    assert_eq!(app.file_history.roms, [std::path::absolute(&path).unwrap()]);
    let history = app.file_history.clone();
    let error = app.import_kickstart(&invalid_path).unwrap_err();
    assert!(error.contains("got 1024 bytes"));
    assert!(app.host.status.running);
    assert_eq!(app.file_history, history);
    assert_eq!(app.host.status.generation, generation + 1);

    let ctx = egui::Context::default();
    common::command(app, HostCommand::Pause);
    common::wait(app, &ctx, |app| !app.host.status.running);
    let before = common::snapshot(app);
    assert_eq!(app.import_kickstart(&invalid_path).unwrap_err(), error);
    assert_eq!(common::snapshot(app), before);
    let inspection = common::inspect(app);
    assert_eq!(inspection.cpu.instruction_pc(), 0xfc0008);
    assert_eq!(inspection.cpu.a_long(7), 0x40000);
    assert_eq!(inspection.memory.read_word_debug(0), 0x0004);
    assert_eq!(inspection.memory.kickstart_rom(), rom);
    app.file_error = None;
    common::settle_harness(&mut harness);
    assert!(harness.query_by_label("ROM: boot.rom").is_none());
}

fn boot_rom() -> Vec<u8> {
    let mut rom = vec![0xff; 262144];
    rom[..4].copy_from_slice(&0x0004_0000_u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0x00fc_0008_u32.to_be_bytes());
    rom[8..10].copy_from_slice(&[0x60, 0xfe]); // BRA *
    rom
}

#[test]
fn export_with_an_unavailable_patched_rom_leaves_destination_files_intact() {
    let existing = fixture_path("existing-snapshot.json");
    let absent = fixture_path("new-snapshot.json");
    std::fs::write(&existing, b"previous file").unwrap();
    assert!(!absent.exists());
    let mut app = EmulatorApp::default();
    common::command(
        &mut app,
        HostCommand::WriteMemory {
            address: config::address_map::KICKSTART_BASE + 512,
            bytes: vec![0x25],
            restart_instruction: false,
        },
    )
    .value
    .unwrap();
    let before = common::snapshot(&mut app);
    let generation = app.host.status.generation;
    let ctx = egui::Context::default();
    for path in [&existing, &absent] {
        app.save_state_file(path);
        common::settle(&mut app, &ctx);
        let message = app.file_error.take().unwrap();
        assert!(
            message.contains("Kickstart ROM source is unavailable"),
            "{message}"
        );
        assert_eq!(app.host.status.generation, generation);
        assert_eq!(common::snapshot(&mut app), before);
    }
    assert_eq!(std::fs::read(&existing).unwrap(), b"previous file");
    assert!(!absent.exists());
    std::fs::remove_file(existing).unwrap();
}

#[test]
#[ignore = "requires the generated files from the Chromium save exchange run"]
fn exchange_browser_save_through_native_player() {
    let directory = PathBuf::from(std::env::var_os("AMIGA_SAVE_STATE_EXCHANGE").unwrap());
    let rom_path = directory.join("test.rom");
    let first_path = directory.join("first.adf");
    let second_path = directory.join("second.adf");
    let source_path = directory.join("browser.json");
    let output = directory.join("native.json");
    let original = std::fs::read(&source_path).unwrap();
    let saved = debugger::SaveState::from_bytes(&original).unwrap();
    let expected_header = saved.header.clone();
    assert_eq!(expected_header.version, 17);
    assert_eq!(
        expected_header.association.mounted[0].as_deref(),
        Some("manual-1")
    );
    let sources = debugger::MediaSources {
        rom: Some(debugger::SourceImage {
            name: Some("test.rom".into()),
            bytes: std::fs::read(&rom_path).unwrap(),
        }),
        disks: [
            Some(debugger::SourceImage {
                name: Some("second.adf".into()),
                bytes: std::fs::read(&second_path).unwrap(),
            }),
            None,
            None,
            None,
        ],
    };
    let mut expected = saved.restore(&sources).unwrap();
    expected.keyboard.release_keys();
    expected.game_ports.release_controls();

    let mut app = EmulatorApp::default();
    let library = tempfile::tempdir().unwrap();
    app.player.library_root = Some(library.path().to_owned());
    app.navigate_player(PlayerScreen::Configuration);
    app.import_kickstart(&rom_path).unwrap();
    let ctx = egui::Context::default();
    app.load_setup_disk(0, &first_path);
    app.load_setup_disk(0, &second_path);
    // Boot with only the ROM so the saved disk reference must use setup sources.
    app.player.setup_mounted.fill(None);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    common::settle_harness(&mut harness);
    common::choose_manual_kickstart(&mut harness, "test.rom");
    harness.get_by_label("Start").click_accesskit();
    common::wait_harness(&mut harness, |app| app.host.status.running);
    harness.get_by_label("Pause").click_accesskit();
    common::wait_harness(&mut harness, |app| !app.host.status.running);
    let mut app = std::mem::take(harness.state_mut());
    assert!(app.file_error.is_none(), "{:?}", app.file_error);
    let generation = app.host.status.generation;
    app.load_state_file(&source_path);
    common::settle(&mut app, &ctx);
    assert!(app.file_error.is_none(), "{:?}", app.file_error);
    assert_eq!(app.host.status.generation, generation + 1);
    assert_eq!(app.host.status.cck, expected.cck);
    assert!(!app.host.status.running);
    assert_eq!(app.player.mounted, expected_header.association.mounted);
    assert_eq!(common::snapshot(&mut app), *expected);

    app.save_state_file(&output);
    common::settle(&mut app, &ctx);
    assert!(app.file_error.is_none(), "{:?}", app.file_error);
    let reopened = debugger::SaveState::from_bytes(&std::fs::read(&output).unwrap()).unwrap();
    assert_eq!(reopened.header.association, expected_header.association);
    assert_eq!(
        reopened.header.screenshot_png,
        expected_header.screenshot_png
    );
    assert_eq!(*reopened.restore(&sources).unwrap(), *expected);
    assert_eq!(std::fs::read(source_path).unwrap(), original);
}
