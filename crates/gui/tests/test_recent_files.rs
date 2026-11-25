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
        app.loaded_rom_name.is_some() || app.host.status.cck != 0 || app.host.status.running;
    app.load_rom_file(path);
    if confirmation {
        common::confirm_replacement(app);
    } else {
        common::settle(app, &egui::Context::default());
    }
}

#[test]
fn rom_history_keeps_five_successful_files_and_persists_them() {
    let mut app = EmulatorApp::default();
    let mut roms = Vec::new();
    for index in 0..6 {
        let firmware = fixture(&format!("firmware-{index}.rom"), &rom());
        select_rom(&mut app, &firmware);
        roms.push(firmware);
    }
    assert_eq!(
        app.file_history.roms,
        roms[1..].iter().rev().cloned().collect::<Vec<_>>()
    );

    select_rom(&mut app, &roms[3]);
    assert_eq!(app.file_history.roms.len(), 5);
    assert_eq!(app.file_history.roms[0], roms[3]);

    let before = app.file_history.clone();
    let wrong_size = fixture("wrong-size.bin", &[0x4e, 0x71]);
    app.load_rom_file(&wrong_size);
    common::settle(&mut app, &egui::Context::default());
    app.load_rom_file(&wrong_size.with_extension("missing"));
    common::settle(&mut app, &egui::Context::default());
    assert_eq!(
        app.file_history, before,
        "Failed ROM loads must not change recent files"
    );

    let mut storage = MemoryStorage::default();
    app.save(&mut storage);
    let restored: FileHistory = eframe::get_value(&storage, "file_history").unwrap();
    assert_eq!(restored, before);
    assert!(storage.get_string(eframe::APP_KEY).is_some());
    let mut context = eframe::CreationContext::_new_kittest(egui::Context::default());
    context.storage = Some(&storage);
    assert_eq!(EmulatorApp::new(&context).file_history, before);
    let empty_storage = MemoryStorage::default();
    context.storage = Some(&empty_storage);
    assert_eq!(
        EmulatorApp::new(&context).file_history,
        FileHistory::default()
    );
    for path in roms.into_iter().chain([wrong_size]) {
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn reopening_and_saving_history_discards_old_program_paths() {
    #[derive(serde::Serialize)]
    struct LegacyProgram {
        path: PathBuf,
        target: u32,
        auto_prime: bool,
    }

    #[derive(serde::Serialize)]
    struct LegacyHistory {
        binary_directory: Option<PathBuf>,
        rom_directory: Option<PathBuf>,
        disk_directory: Option<PathBuf>,
        binaries: Vec<LegacyProgram>,
        roms: Vec<PathBuf>,
    }

    let old = LegacyHistory {
        binary_directory: Some(PathBuf::from("programs")),
        rom_directory: Some(PathBuf::from("firmware")),
        disk_directory: Some(PathBuf::from("disks")),
        binaries: vec![LegacyProgram {
            path: PathBuf::from("programs/program.bin"),
            target: 0x1000,
            auto_prime: true,
        }],
        roms: vec![PathBuf::from("firmware/kickstart.rom")],
    };
    let mut storage = MemoryStorage::default();
    eframe::set_value(&mut storage, "file_history", &old);
    let mut context = eframe::CreationContext::_new_kittest(egui::Context::default());
    context.storage = Some(&storage);
    let mut reopened = EmulatorApp::new(&context);
    assert_eq!(reopened.file_history.roms, old.roms);
    assert_eq!(reopened.file_history.rom_directory, old.rom_directory);
    assert_eq!(reopened.file_history.disk_directory, old.disk_directory);
    reopened.save(&mut storage);
    let persisted = storage.get_string("file_history").unwrap();
    for retired in ["binary_directory", "binaries", "program.bin", "auto_prime"] {
        assert!(
            !persisted.contains(retired),
            "{retired} persisted after saving"
        );
    }
}

#[test]
fn game_menu_reloads_rom_and_keeps_disk_directory_separate() {
    let path = fixture("menu-firmware.bin", &rom());
    let disk_directory = tempfile::tempdir().unwrap();
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.file_history.disk_directory = Some(disk_directory.path().to_owned());
    app.load_rom_file(&path);
    common::settle(&mut app, &egui::Context::default());
    assert_eq!(
        app.file_history.disk_picker_directory(),
        Some(disk_directory.path())
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
    harness.get_by_label("File").click_accesskit();
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    harness.get_by_label("menu-firmware.bin").click_accesskit();
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
    harness.get_by_label("File").click_accesskit();
    harness.run_steps(2);
    common::settle_harness(&mut harness);
    harness.get_by_label("menu-firmware.bin").click_accesskit();
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
    assert_eq!(
        harness.state().file_history.disk_picker_directory(),
        Some(disk_directory.path())
    );
    assert_eq!(common::snapshot(harness.state_mut()), before);
}

#[test]
fn rom_picker_uses_nearest_existing_parent_without_changing_the_disk_directory() {
    let path = fixture(
        "picker-directory/removed/nested/firmware.rom",
        &[0x4e, 0x71],
    );
    let directory = path.parent().unwrap();
    let parent = directory.parent().unwrap();
    let existing = parent.parent().unwrap();
    let mut history = FileHistory {
        rom_directory: Some(directory.to_owned()),
        disk_directory: Some(existing.to_owned()),
        ..FileHistory::default()
    };
    assert_eq!(history.rom_picker_directory(), Some(directory));
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(directory).unwrap();
    assert_eq!(history.rom_picker_directory(), Some(parent));
    std::fs::remove_dir(parent).unwrap();
    assert_eq!(history.rom_picker_directory(), Some(existing));
    assert_eq!(history.disk_picker_directory(), Some(existing));
    // An ancestor replaced by a file must also be skipped.
    std::fs::write(parent, []).unwrap();
    assert_eq!(history.rom_picker_directory(), Some(existing));
    std::fs::remove_file(parent).unwrap();
    history.rom_directory = None;
    assert_eq!(history.rom_picker_directory(), None);
    assert_eq!(history.disk_picker_directory(), Some(existing));
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
        reopened.file_history.rom_picker_directory(),
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
        reopened.file_history.rom_picker_directory(),
        Some(rom_directory.as_path())
    );
}
