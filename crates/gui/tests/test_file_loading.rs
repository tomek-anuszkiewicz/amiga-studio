#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use egui_kittest::{kittest::Queryable, Harness};
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
    let before = app.session.save_state();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    harness.run_steps(2);
    harness.get_by_label("Load into Memory").click();
    harness.run_steps(3);
    assert!(harness.state().load_binary_modal_open);
    assert_eq!(harness.state().session.save_state(), before);
    let error = harness.state().file_error.clone().unwrap();
    assert!(error.contains("256 KiB (262144 bytes)"));
    assert!(error.contains("received 524288 bytes"));
    harness.get_by_label(&error);
    harness.run_steps(3);
    assert_eq!(harness.state().file_error.as_ref(), Some(&error));

    std::fs::write(&path, vec![0x4E; 262144]).unwrap();
    harness.get_by_label("Load into Memory").click();
    harness.run_steps(3);
    assert!(!harness.state().load_binary_modal_open);
    assert!(harness.state().file_error.is_none());
    assert_eq!(
        harness.state().session.bus().read_word_debug(0xFC0000),
        0x4E4E
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn invalid_snapshot_rom_displays_a_dismissible_error_without_restoring() {
    let path = fixture_path("state.json");
    let mut app = EmulatorApp::default();
    let before = app.session.save_state();
    let mut json = app.session.save_state_to_json().unwrap();
    let key = json.find("\"kickstart_rom\"").unwrap();
    let start = key + json[key..].find('[').unwrap() + 1;
    let end = start + json[start..].find(']').unwrap();
    let doubled = format!("{},{}", &json[start..end], &json[start..end]);
    json.replace_range(start..end, &doubled);
    std::fs::write(&path, json).unwrap();
    app.load_state_file(&path);
    assert_eq!(app.session.save_state(), before);
    let error = app.file_error.clone().unwrap();
    assert!(error.contains("received 524288 bytes"));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    harness.run_steps(2);
    harness.get_by_label(&error);
    harness.get_by_label("Close").click();
    harness.run_steps(2);
    assert!(harness.state().file_error.is_none());
    assert_eq!(harness.state().session.save_state(), before);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn firmware_load_boots_reset_vectors_and_preserves_the_active_rom_on_failure() {
    let path = fixture_path("boot.rom");
    let mut rom = vec![0xff; 262144];
    rom[..4].copy_from_slice(&0x0004_0000_u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0x00fc_0008_u32.to_be_bytes());
    rom[8..10].copy_from_slice(&[0x60, 0xfe]); // BRA *
    std::fs::write(&path, &rom).unwrap();
    let mut app = EmulatorApp::default();
    app.set_view_mode(gui::ViewMode::ScreenOnly);
    app.session
        .load_binary(0x1000, &[0x60, 0xfe], true)
        .unwrap();
    app.session.toggle_run();
    app.session.run_frame_slice(25);
    app.load_rom_file(&path);
    let name = path.file_name().unwrap().to_str().unwrap();
    assert_eq!(app.loaded_rom_name.as_deref(), Some(name));
    assert!(app.file_error.is_none());
    assert!(!app.session.is_running);
    assert_eq!(app.session.machine.cck, 0);
    assert_eq!(app.session.machine.cpu.state.instruction_pc(), 0xfc0008);
    assert_eq!(app.session.machine.cpu.state.a_long(7), 0x40000);
    assert_eq!(app.session.bus().read_word_debug(0), 0x0004);
    assert_eq!(app.session.bus().kickstart_rom(), rom);
    app.session.toggle_run();
    app.session
        .run_frame_slice(debugger::session::MAX_FRAME_SLICE_CCK);
    assert_eq!(app.session.machine.denise.frame_builder.frame_number, 1);
    let before = app.session.save_state();
    app.load_rom_data("invalid.rom", &[0; 1024]);
    assert_eq!(app.session.save_state(), before);
    assert!(app.session.is_running);
    assert_eq!(app.loaded_rom_name.as_deref(), Some(name));
    assert!(app
        .file_error
        .as_deref()
        .unwrap()
        .contains("received 1024 bytes"));
    app.session.request_pause();
    app.file_error = None;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    harness.run_steps(2);
    harness.get_by_label(&format!("ROM: {name}"));
    std::fs::remove_file(path).unwrap();
}
