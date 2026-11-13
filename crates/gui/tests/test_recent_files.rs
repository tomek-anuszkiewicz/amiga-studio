#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use eframe::{App, Storage};
use egui_kittest::{kittest::Queryable, Harness};
use gui::{file_actions::FileHistory, EmulatorApp, ViewMode};
use std::{collections::HashMap, path::PathBuf};

#[derive(Default)]
struct MemoryStorage(HashMap<String, String>);

impl Storage for MemoryStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.into(), value);
    }
    fn flush(&mut self) {}
}

fn fixture(name: &str, data: &[u8]) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.agent/tmp/recent-files")
        .join(std::process::id().to_string());
    std::fs::create_dir_all(&directory).unwrap();
    let path = std::path::absolute(directory.join(name)).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, data).unwrap();
    path
}

fn rom() -> Vec<u8> {
    let mut data = vec![0xff; 262144];
    data[..4].copy_from_slice(&0x0004_0000_u32.to_be_bytes());
    data[4..8].copy_from_slice(&0x00fc_0008_u32.to_be_bytes());
    data[8..10].copy_from_slice(&[0x60, 0xfe]);
    data
}

#[test]
fn separate_histories_keep_five_successful_files_and_persist_load_options() {
    let mut app = EmulatorApp::default();
    let mut binaries = Vec::new();
    let mut roms = Vec::new();
    for index in 0..6 {
        let binary = fixture(&format!("program-{index}.bin"), &[0x4e, 0x71]);
        let firmware = fixture(&format!("firmware-{index}.rom"), &rom());
        app.load_binary_file(&binary, 0x1000, true);
        app.load_rom_file(&firmware);
        binaries.push(binary);
        roms.push(firmware);
    }
    assert_eq!(app.file_history.binaries.len(), 5);
    assert_eq!(app.file_history.roms.len(), 5);
    assert_eq!(app.file_history.binaries[0].path, binaries[5]);
    assert_eq!(app.file_history.binaries[4].path, binaries[1]);
    assert_eq!(
        app.file_history.roms,
        roms[1..].iter().rev().cloned().collect::<Vec<_>>()
    );

    app.load_binary_file(&binaries[2], 0x2000, false);
    app.load_rom_file(&roms[3]);
    assert_eq!(app.file_history.binaries.len(), 5);
    assert_eq!(app.file_history.binaries[0].path, binaries[2]);
    assert_eq!(app.file_history.binaries[0].target, 0x2000);
    assert!(!app.file_history.binaries[0].auto_prime);
    assert_eq!(app.file_history.roms.len(), 5);
    assert_eq!(app.file_history.roms[0], roms[3]);

    let before = app.file_history.clone();
    app.load_rom_file(&binaries[1]);
    app.load_binary_file(&binaries[1], 0xfc0000, false);
    app.load_binary_file(&binaries[1].with_extension("missing"), 0x1000, false);
    assert_eq!(
        app.file_history, before,
        "Failed loads must not change recent files"
    );

    let mut storage = MemoryStorage::default();
    app.save(&mut storage);
    let restored: FileHistory = eframe::get_value(&storage, "file_history").unwrap();
    assert_eq!(restored, before);
    assert!(storage.get_string(eframe::APP_KEY).is_some());
    let mut context = eframe::CreationContext::_new_kittest(egui::Context::default());
    context.storage = Some(&storage);
    let reopened = EmulatorApp::new(&context);
    assert_eq!(reopened.file_history, before);
    let empty_storage = MemoryStorage::default();
    context.storage = Some(&empty_storage);
    assert_eq!(
        EmulatorApp::new(&context).file_history,
        FileHistory::default()
    );
    for path in binaries.iter().chain(&roms) {
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn recent_menu_reloads_current_bytes_with_saved_options_and_reports_missing_files() {
    let path = fixture("menu-program.bin", &[0x4e, 0x71]);
    let mut app = EmulatorApp::default();
    app.load_binary_file(&path, 0x2000, false);
    let pc = app.session.machine.cpu.state.pc();
    std::fs::write(&path, [0x70, 0x2a]).unwrap();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    harness.run_steps(2);
    harness.get_by_label("File").click();
    harness.run_steps(2);
    harness.get_by_label("Recent binaries");
    harness.get_by_label("menu-program.bin").click();
    harness.run_steps(2);
    assert_eq!(
        harness.state().session.bus().read_word_debug(0x2000),
        0x702a
    );
    assert_eq!(harness.state().session.machine.cpu.state.pc(), pc);
    assert!(!harness.state().load_binary_modal_open);
    assert_eq!(harness.state().file_history.binaries.len(), 1);

    std::fs::remove_file(&path).unwrap();
    let before = harness.state().session.save_state();
    harness.get_by_label("File").click();
    harness.run_steps(2);
    harness.get_by_label("menu-program.bin").click();
    harness.run_steps(2);
    assert!(harness.state().file_error.is_some());
    assert_eq!(harness.state().session.save_state(), before);
    assert_eq!(harness.state().file_history.binaries.len(), 1);
}

#[test]
fn game_menu_reloads_rom_and_keeps_binary_directory_separate() {
    let path = fixture("menu-firmware.rom", &rom());
    let binary = fixture("separate-folder/program.bin", &[0x4e, 0x71]);
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.load_binary_file(&binary, 0x1000, true);
    app.load_rom_file(&path);
    assert_eq!(
        app.file_history.binary_directory.as_deref(),
        binary.parent()
    );
    assert_eq!(app.file_history.rom_directory.as_deref(), path.parent());
    app.session.machine.step_cycles(10);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    harness.run_steps(2);
    harness.get_by_label("File").click();
    harness.run_steps(2);
    harness.get_by_label("menu-firmware.rom").click();
    harness.run_steps(2);
    assert_eq!(harness.state().session.machine.cck, 0);
    assert_eq!(
        harness.state().session.machine.cpu.state.instruction_pc(),
        0xfc0008
    );
    assert!(!harness.state().session.is_running);
    assert_eq!(harness.state().file_history.roms.len(), 1);
    std::fs::remove_file(&path).unwrap();
    let before = harness.state().file_history.clone();
    harness.state_mut().load_rom_file(&path);
    assert!(harness.state().file_error.is_some());
    assert_eq!(harness.state().file_history, before);
    std::fs::remove_file(binary).unwrap();
}
