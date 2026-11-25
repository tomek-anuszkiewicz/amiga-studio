#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
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
    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
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

fn select_rom(app: &mut EmulatorApp, path: &std::path::Path) {
    let confirmation =
        !app.file_history.roms.is_empty() || app.host.status.cck != 0 || app.host.status.running;
    app.load_rom_file(path);
    if confirmation {
        common::confirm_replacement(app);
    } else {
        common::settle(app, &egui::Context::default());
    }
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
        common::settle(&mut app, &egui::Context::default());
        select_rom(&mut app, &firmware);
        common::settle(&mut app, &egui::Context::default());
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

    common::settle(&mut app, &egui::Context::default());
    select_rom(&mut app, &roms[3]);
    common::settle(&mut app, &egui::Context::default());
    assert_eq!(app.file_history.binaries.len(), 5);
    assert_eq!(app.file_history.binaries[0].path, binaries[2]);
    assert_eq!(app.file_history.binaries[0].target, 0x2000);
    assert!(!app.file_history.binaries[0].auto_prime);
    assert_eq!(app.file_history.roms.len(), 5);
    assert_eq!(app.file_history.roms[0], roms[3]);

    let before = app.file_history.clone();
    app.load_rom_file(&binaries[1]);
    common::settle(&mut app, &egui::Context::default());
    app.load_binary_file(&binaries[1], 0xfc0000, false);
    common::settle(&mut app, &egui::Context::default());
    app.load_binary_file(&binaries[1].with_extension("missing"), 0x1000, false);
    common::settle(&mut app, &egui::Context::default());
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
fn recorded_binary_can_reload_current_bytes_with_saved_options_and_report_missing_files() {
    let path = fixture("menu-program.bin", &[0x4e, 0x71]);
    let mut app = EmulatorApp::default();
    app.load_binary_file(&path, 0x2000, false);
    common::settle(&mut app, &egui::Context::default());
    let pc = common::inspect(&mut app).cpu.pc();
    std::fs::write(&path, [0x70, 0x2a]).unwrap();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    assert!(harness.query_by_label("File").is_none());
    let recent = harness.state().file_history.binaries[0].clone();
    harness
        .state_mut()
        .load_binary_file(&recent.path, recent.target, recent.auto_prime);
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    assert_eq!(
        common::inspect(harness.state_mut())
            .memory
            .read_word_debug(0x2000),
        0x702a
    );
    assert_eq!(common::inspect(harness.state_mut()).cpu.pc(), pc);
    assert!(!harness.state().load_binary_modal_open);
    assert_eq!(harness.state().file_history.binaries.len(), 1);

    std::fs::remove_file(&path).unwrap();
    let before = common::snapshot(harness.state_mut());
    harness
        .state_mut()
        .load_binary_file(&recent.path, recent.target, recent.auto_prime);
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    assert!(harness.state().file_error.is_some());
    assert_eq!(common::snapshot(harness.state_mut()), before);
    assert!(harness.state().file_history.binaries.is_empty());
    assert!(harness
        .state()
        .file_error
        .as_ref()
        .unwrap()
        .contains("Removed from recent binaries"));
    let mut storage = MemoryStorage::default();
    harness.state_mut().save(&mut storage);
    let restored: FileHistory = eframe::get_value(&storage, "file_history").unwrap();
    assert!(restored.binaries.is_empty());
}

#[test]
fn recorded_rom_can_reload_and_keep_binary_directory_separate() {
    let path = fixture("menu-firmware.rom", &rom());
    let binary = fixture("separate-folder/program.bin", &[0x4e, 0x71]);
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.load_binary_file(&binary, 0x1000, true);
    common::settle(&mut app, &egui::Context::default());
    app.load_rom_file(&path);
    common::settle(&mut app, &egui::Context::default());
    assert_eq!(
        app.file_history.binary_directory.as_deref(),
        binary.parent()
    );
    assert_eq!(app.file_history.rom_directory.as_deref(), path.parent());
    let step = common::command(&mut app, gui::host_commands::HostCommand::StepCck);
    assert!(step.value.is_ok());
    assert_eq!(step.status.cck, 1);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    assert!(harness.query_by_label("File").is_none());
    let recent = harness.state().file_history.roms[0].clone();
    harness.state_mut().load_rom_file(&recent);
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    assert!(
        harness.state().host.status.cck > 0,
        "ROM replacement needs confirmation"
    );
    harness.get_by_label("Replace session").click_accesskit();
    common::settle_harness(&mut harness);
    assert_eq!(harness.state().host.status.cck, 0);
    assert_eq!(
        common::inspect(harness.state_mut()).cpu.instruction_pc(),
        0xfc0008
    );
    assert!(!harness.state().host.status.running);
    assert_eq!(harness.state().file_history.roms.len(), 1);
    std::fs::remove_file(&path).unwrap();
    let before = common::snapshot(harness.state_mut());
    harness.state_mut().load_rom_file(&recent);
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    assert!(harness.state().file_error.is_some());
    assert!(harness
        .state()
        .file_error
        .as_ref()
        .unwrap()
        .contains("Removed from recent ROMs"));
    assert!(harness.state().file_history.roms.is_empty());
    assert_eq!(harness.state().file_history.binaries[0].path, binary);
    assert_eq!(common::snapshot(harness.state_mut()), before);
    std::fs::remove_file(binary).unwrap();
}

#[test]
fn picker_uses_nearest_existing_parent_without_changing_the_other_category() {
    let path = fixture("picker-directory/removed/nested/program.bin", &[0x4e, 0x71]);
    let directory = path.parent().unwrap();
    let parent = directory.parent().unwrap();
    let existing = parent.parent().unwrap();
    let mut history = FileHistory {
        binary_directory: Some(directory.to_owned()),
        rom_directory: Some(existing.to_owned()),
        ..FileHistory::default()
    };
    assert_eq!(history.picker_directory(true), Some(directory));
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(directory).unwrap();
    assert_eq!(history.picker_directory(true), Some(parent));
    std::fs::remove_dir(parent).unwrap();
    assert_eq!(history.picker_directory(true), Some(existing));
    assert_eq!(history.picker_directory(false), Some(existing));
    // An ancestor replaced by a file must also be skipped.
    std::fs::write(parent, []).unwrap();
    assert_eq!(history.picker_directory(true), Some(existing));
    std::fs::remove_file(parent).unwrap();
    history.binary_directory = None;
    assert_eq!(history.picker_directory(true), None);
    assert_eq!(history.picker_directory(false), Some(existing));
}

#[test]
fn confirmed_manual_disk_directory_is_restored_and_falls_back_independently() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("import").join("selected");
    let rom_directory = root.path().join("ROM");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::create_dir(&rom_directory).unwrap();
    let disk = directory.join("Manual.adf");
    std::fs::write(&disk, vec![0x33; 880 * 1024]).unwrap();
    let mut app = EmulatorApp::default();
    app.file_history.rom_directory = Some(rom_directory.clone());
    app.load_setup_disk(0, &disk);
    assert_eq!(
        app.file_history.disk_picker_directory(),
        Some(directory.as_path())
    );
    assert!(app.player.library_root.is_none());

    let mut storage = MemoryStorage::default();
    app.save(&mut storage);
    let mut context = eframe::CreationContext::_new_kittest(egui::Context::default());
    context.storage = Some(&storage);
    let reopened = EmulatorApp::new(&context);
    assert_eq!(
        reopened.file_history.disk_picker_directory(),
        Some(directory.as_path())
    );
    assert_eq!(
        reopened.file_history.picker_directory(false),
        Some(rom_directory.as_path())
    );
    std::fs::remove_file(disk).unwrap();
    std::fs::remove_dir(&directory).unwrap();
    assert_eq!(
        reopened.file_history.disk_picker_directory(),
        directory.parent()
    );
    assert_eq!(reopened.file_history.disk_directory, Some(directory));
    assert_eq!(
        reopened.file_history.picker_directory(false),
        Some(rom_directory.as_path())
    );
}
