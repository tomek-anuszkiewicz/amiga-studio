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
    app.pending_binary_path = Some(path.clone());
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
