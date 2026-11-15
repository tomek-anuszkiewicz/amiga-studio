#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use egui_kittest::{kittest::Queryable, Harness};
use gui::host_commands::HostCommand;
use gui::EmulatorApp;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.agent/tmp");
    std::fs::create_dir_all(&directory).unwrap();
    directory.join(format!("kickstart-{name}-{}", std::process::id()))
}

#[test]
fn invalid_rom_keeps_the_dialog_open_and_can_be_retried() {
    let path = fixture_path("load.rom");
    std::fs::write(&path, vec![0x55; 524288]).unwrap();
    let mut app = EmulatorApp::default();
    app.pending_binary = Some(gui::BinarySource::File(path.clone()));
    app.load_binary_modal_open = true;
    app.load_target_addr_str = "FC0000".to_string();
    let before = common::snapshot(&mut app);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    common::settle_harness(&mut harness);
    harness.get_by_label("Load into Memory").click();
    common::settle_harness(&mut harness);
    assert!(harness.state().load_binary_modal_open);
    assert_eq!(common::snapshot(harness.state_mut()), before);
    let error = harness.state().file_error.clone().unwrap();
    assert!(error.contains("256 KiB (262144 bytes)"));
    assert!(error.contains("received 524288 bytes"));
    harness.get_by_label(&error);
    common::settle_harness(&mut harness);
    assert_eq!(harness.state().file_error.as_ref(), Some(&error));

    std::fs::write(&path, vec![0x4E; 262144]).unwrap();
    harness.get_by_label("Load into Memory").click();
    common::settle_harness(&mut harness);
    assert!(!harness.state().load_binary_modal_open);
    assert!(harness.state().file_error.is_none());
    assert_eq!(
        common::inspect(harness.state_mut())
            .memory
            .read_word_debug(0xFC0000),
        0x4E4E
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn invalid_snapshot_rom_displays_a_dismissible_error_without_restoring() {
    let path = fixture_path("state.json");
    let mut app = EmulatorApp::default();
    let before = common::snapshot(&mut app);
    let mut json = common::snapshot_json(&mut app);
    let key = json.find("\"kickstart_rom\"").unwrap();
    let start = key + json[key..].find('[').unwrap() + 1;
    let end = start + json[start..].find(']').unwrap();
    let doubled = format!("{},{}", &json[start..end], &json[start..end]);
    json.replace_range(start..end, &doubled);
    std::fs::write(&path, json).unwrap();
    app.load_state_file(&path);
    common::settle(&mut app, &egui::Context::default());
    assert_eq!(common::snapshot(&mut app), before);
    let error = app.file_error.clone().unwrap();
    assert!(error.contains("received 524288 bytes"));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    common::settle_harness(&mut harness);
    harness.get_by_label(&error);
    harness.get_by_label("Close").click();
    common::settle_harness(&mut harness);
    assert!(harness.state().file_error.is_none());
    assert_eq!(common::snapshot(harness.state_mut()), before);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn firmware_load_boots_reset_vectors_and_preserves_the_active_rom_on_failure() {
    let path = fixture_path("boot.rom");
    let invalid_path = fixture_path("invalid.rom");
    std::fs::write(&invalid_path, [0; 1024]).unwrap();
    let mut rom = vec![0xff; 262144];
    rom[..4].copy_from_slice(&0x0004_0000_u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0x00fc_0008_u32.to_be_bytes());
    rom[8..10].copy_from_slice(&[0x60, 0xfe]); // BRA *
    std::fs::write(&path, &rom).unwrap();
    let mut session = Box::new(gui::DebuggerSession::new());
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    session.toggle_run();
    session.run_frame_slice(25);
    let mut app = EmulatorApp::with_session(session);
    app.set_view_mode(gui::ViewMode::ScreenOnly);
    app.load_rom_file(&path);
    common::settle(&mut app, &egui::Context::default());
    let name = path.file_name().unwrap().to_str().unwrap();
    assert_eq!(app.loaded_rom_name.as_deref(), Some(name));
    assert!(app.file_error.is_none());
    assert!(!app.host.status.running);
    assert_eq!(app.host.status.cck, 0);
    assert_eq!(common::inspect(&mut app).cpu.instruction_pc(), 0xfc0008);
    assert_eq!(common::inspect(&mut app).cpu.a_long(7), 0x40000);
    assert_eq!(common::inspect(&mut app).memory.read_word_debug(0), 0x0004);
    assert_eq!(common::inspect(&mut app).memory.kickstart_rom(), rom);
    common::command(&mut app, HostCommand::Run);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.host.status.field_number == 0 {
        common::settle(&mut app, &egui::Context::default());
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(app.host.status.field_number >= 1);
    app.load_rom_file(&invalid_path);
    common::settle(&mut app, &egui::Context::default());
    assert!(app.host.status.running);
    assert_eq!(app.loaded_rom_name.as_deref(), Some(name));
    assert!(app
        .file_error
        .as_deref()
        .unwrap()
        .contains("received 1024 bytes"));
    common::command(&mut app, HostCommand::Pause);
    while app.host.status.running {
        common::settle(&mut app, &egui::Context::default());
        assert!(std::time::Instant::now() < deadline);
    }
    let before = common::snapshot(&mut app);
    app.load_rom_file(&invalid_path);
    common::settle(&mut app, &egui::Context::default());
    assert_eq!(common::snapshot(&mut app), before);
    assert_eq!(common::inspect(&mut app).memory.kickstart_rom(), rom);
    app.file_error = None;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    common::settle_harness(&mut harness);
    harness.get_by_label(&format!("ROM: {name}"));
    std::fs::remove_file(invalid_path).unwrap();
    std::fs::remove_file(path).unwrap();
}
