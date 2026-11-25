#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use eframe::{App, Storage};
use egui::accesskit::Role;
use egui_kittest::{
    kittest::{By, NodeT, Queryable},
    Harness,
};
use gui::game_library::{GameLibrary, GameManifest};
use gui::host_commands::HostCommand;
use gui::player::PlayerScreen;
use gui::player_config::{ConfigurationCatalog, NamedConfiguration};
use gui::EmulatorApp;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SIZE: usize = 262_144;
const LEGACY: &str = "_shared/kickstart-1.3.rom";
const ADOPTED: &str = "kickstarts/kickstart-1.3.rom";

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

fn reopen(storage: &MemoryStorage) -> EmulatorApp {
    let mut context = eframe::CreationContext::_new_kittest(egui::Context::default());
    context.storage = Some(storage);
    EmulatorApp::new(&context)
}

fn open(root: &Path) -> EmulatorApp {
    let mut storage = MemoryStorage::default();
    #[derive(serde::Serialize)]
    struct NativeSettings<'a> {
        root: Option<&'a Path>,
        selected: &'a str,
        migrated_legacy: bool,
    }
    eframe::set_value(
        &mut storage,
        "native_player_settings",
        &NativeSettings {
            root: Some(root),
            selected: "standard",
            migrated_legacy: true,
        },
    );
    reopen(&storage)
}

fn rom(marker: u8) -> Vec<u8> {
    let mut bytes = vec![0; SIZE];
    bytes[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    bytes[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    bytes[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    bytes[0x200] = marker;
    bytes
}

fn write(path: &Path, bytes: impl AsRef<[u8]>) -> PathBuf {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
    path.to_path_buf()
}

fn two_profiles(standard: &str, other: &str) -> ConfigurationCatalog {
    let mut catalog = ConfigurationCatalog::default();
    catalog.configurations[0].name = "First profile".into();
    catalog.configurations[0].launch.rom = standard.into();
    let mut second = NamedConfiguration {
        id: "other".into(),
        name: "Second profile".into(),
        ..Default::default()
    };
    second.launch.rom = other.into();
    catalog.configurations.push(second);
    catalog
}

fn persist(root: &Path, catalog: &ConfigurationCatalog) -> PathBuf {
    let mut data = serde_json::to_value(catalog).unwrap();
    data.as_object_mut().unwrap().remove("selected");
    write(&root.join("configurations.json"), data.to_string())
}

fn game(root: &Path, id: &str, profile: &str, initial_state: Option<&str>) -> PathBuf {
    let manifest: GameManifest = serde_json::from_value(json!({
        "id": id,
        "title": id,
        "configuration": profile,
        "disks": [],
        "mounted": [null, null, null, null],
        "initial_state": initial_state,
    }))
    .unwrap();
    write(
        &root.join(id).join("game.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
}

fn harness(app: EmulatorApp) -> Harness<'static, EmulatorApp> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1120.0, 900.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    common::settle_harness(&mut harness);
    harness
}

fn click(harness: &mut Harness<'_, EmulatorApp>, label: &str) {
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    common::settle_harness(harness);
}

fn choose_profile(harness: &mut Harness<'_, EmulatorApp>, current: &str, next: &str) {
    harness
        .get(By::new().role(Role::ComboBox).value(current))
        .click_accesskit();
    common::settle_harness(harness);
    click(harness, next);
}

fn wait_for_dialog_button(harness: &mut Harness<'_, EmulatorApp>, label: &str) {
    // A new modal's sizing pass can follow host settlement.
    let deadline = Instant::now() + Duration::from_secs(10);
    while harness
        .query_by_role_and_label(Role::Button, label)
        .is_none_or(|button| button.accesskit_node().is_hidden())
    {
        assert!(
            Instant::now() < deadline,
            "Dialog button {label:?} did not appear"
        );
        harness.step();
    }
}

fn choose_rom(harness: &mut Harness<'_, EmulatorApp>, name: &str) {
    harness
        .get_by_role_and_label(Role::ComboBox, "Kickstart")
        .click();
    common::settle_harness(harness);
    harness.get_by_role_and_label(Role::Button, name).click();
    common::settle_harness(harness);
}

fn stop_and_check(harness: &mut Harness<'_, EmulatorApp>, marker: u8) {
    common::wait_harness(harness, |app| app.host.status.running);
    click(harness, "Pause");
    common::wait_harness(harness, |app| !app.host.status.running);
    assert_eq!(
        common::read_memory(harness.state_mut(), 0xfc0200, 1),
        [marker]
    );
}

#[test]
fn add_and_select_kickstarts_for_two_relocated_profiles() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    persist(&root, &two_profiles("", ""));
    let source_a = write(&temp.path().join("first/same.rom"), rom(0x11));
    let source_b = write(&temp.path().join("second/same.rom"), rom(0x22));
    let first_manifest = game(&root, "First game", "standard", None);
    let second_manifest = game(&root, "Second game", "other", None);
    let original_manifests =
        [&first_manifest, &second_manifest].map(|path| fs::read(path).unwrap());
    let mut app = open(&root);
    app.import_kickstart(&source_a).unwrap();
    app.import_kickstart(&source_b).unwrap();
    assert_eq!(
        fs::read(root.join("kickstarts/same.rom")).unwrap(),
        rom(0x11)
    );
    assert_eq!(
        fs::read(root.join("kickstarts/same (2).rom")).unwrap(),
        rom(0x22)
    );
    let disk = write(&temp.path().join("disks/disk.adf"), vec![0; 901_120]);
    app.load_setup_disk(0, &disk);
    let mut storage = MemoryStorage::default();
    app.save(&mut storage);
    let mut app = reopen(&storage);
    assert_eq!(app.file_history.disk_picker_directory(), disk.parent());
    assert_eq!(app.file_history.rom_picker_directory(), source_b.parent());
    fs::remove_file(&source_a).unwrap();
    fs::remove_file(&source_b).unwrap();
    fs::remove_dir(source_b.parent().unwrap()).unwrap();
    assert_eq!(app.file_history.rom_picker_directory(), Some(temp.path()));
    app.navigate_player(PlayerScreen::Configuration);
    let mut h = harness(app);
    choose_rom(&mut h, "same.rom");
    h.get_by_label("Unrecognized / custom");
    h.get_by_label("Emulator support unverified");
    choose_profile(&mut h, "First profile", "Second profile");
    choose_rom(&mut h, "same (2).rom");
    click(&mut h, "Save");
    drop(h);

    let moved = temp.path().join("relocated");
    fs::rename(&root, &moved).unwrap();
    let isolated = temp.path().join("other library");
    persist(&isolated, &two_profiles("", ""));
    assert!(!isolated.join("kickstarts").exists());
    let mut other = open(&isolated);
    other.navigate_player(PlayerScreen::Setup);
    let other = harness(other);
    other.get_by_label("Add a Kickstart in Configurations.");
    assert!(other.get_by_label("Start").accesskit_node().is_disabled());
    for (path, original) in [first_manifest, second_manifest]
        .into_iter()
        .zip(original_manifests)
    {
        assert_eq!(
            fs::read(moved.join(path.strip_prefix(&root).unwrap())).unwrap(),
            original
        );
    }
    let library = GameLibrary::discover(&moved).unwrap();
    for (game, expected) in library.games.into_iter().zip([0x11, 0x22]) {
        let mut app = open(&moved);
        app.start_library_game(game, None, false);
        let mut h = harness(app);
        stop_and_check(&mut h, expected);
    }
}

#[test]
fn transfer_preserves_profiles_and_recovers_from_conflict_and_catalog_write_failure() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let image = rom(0x61);
    let mut baseline = two_profiles(LEGACY, LEGACY);
    let mut unrelated = NamedConfiguration {
        id: "unrelated".into(),
        name: "Unrelated profile".into(),
        ..Default::default()
    };
    unrelated.launch.rom = "existing.rom".into();
    baseline.configurations.push(unrelated);
    let catalog = persist(root, &baseline);
    let original = fs::read(&catalog).unwrap();
    let manifest = game(root, "Assigned game", "other", None);
    let original_manifest = fs::read(&manifest).unwrap();
    let conflict = write(&root.join(ADOPTED), rom(0x72));
    let missing = open(root);
    assert_eq!(
        missing.player.catalog.configurations,
        baseline.configurations
    );
    assert_eq!(fs::read(&catalog).unwrap(), original);
    let source = write(&root.join(LEGACY), [0u8; 17]);
    let invalid = open(root);
    assert_eq!(
        invalid.player.catalog.configurations,
        baseline.configurations
    );
    assert_eq!(fs::read(&catalog).unwrap(), original);
    write(&source, &image);
    let conflicting = open(root);
    assert_eq!(
        conflicting.player.catalog.configurations,
        baseline.configurations
    );
    assert_eq!(fs::read(&conflict).unwrap(), rom(0x72));
    assert_eq!(fs::read(&catalog).unwrap(), original);
    fs::remove_file(&conflict).unwrap();

    let permissions = fs::metadata(&catalog).unwrap().permissions();
    let mut readonly = permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&catalog, readonly).unwrap();
    let interrupted = open(root);
    fs::set_permissions(&catalog, permissions).unwrap();
    assert_eq!(
        interrupted.player.catalog.configurations,
        baseline.configurations
    );
    assert_eq!(fs::read(&catalog).unwrap(), original);
    assert_eq!(fs::read(&conflict).unwrap(), image);

    let mut expected = interrupted.player.catalog.clone();
    for profile in &mut expected.configurations[..2] {
        profile.launch.rom = ADOPTED.into();
    }
    let app = open(root);
    assert_eq!(app.player.catalog, expected);
    assert_eq!(fs::read(&source).unwrap(), image);
    assert_eq!(fs::read(&manifest).unwrap(), original_manifest);
    let committed = fs::read(&catalog).unwrap();
    assert_eq!(open(root).player.catalog, expected);
    assert_eq!(fs::read(&catalog).unwrap(), committed);
    assert_eq!(fs::read_dir(root.join("kickstarts")).unwrap().count(), 1);
    let mut app = app;
    let game = GameLibrary::discover(root).unwrap().games.remove(0);
    app.start_library_game(game, None, false);
    stop_and_check(&mut harness(app), 0x61);

    for profile in &mut expected.configurations[..2] {
        profile.launch.rom.clear();
    }
    persist(root, &expected);
    fs::remove_file(&conflict).unwrap();
    assert_eq!(open(root).player.catalog, expected);
    assert!(!conflict.exists());
    assert_eq!(fs::read(source).unwrap(), image);
}

#[test]
fn managed_deletion_preserves_the_session_and_restores_from_an_explicit_renamed_source() {
    // Slow RAM remains visible while this synthetic ROM leaves the boot overlay active.
    const SLOW_RAM_ADDRESS: u32 = 0xc0_0000;
    const RAM_MARKER: [u8; 2] = [0x31, 0x32];
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let catalog = persist(&root, &two_profiles("kickstarts/A.rom", "kickstarts/A.rom"));
    let a = write(&temp.path().join("A.rom"), rom(0x31));
    let b = write(&temp.path().join("B.rom"), rom(0x64));
    let mut app = open(&root);
    app.import_kickstart(&a).unwrap();
    app.import_kickstart(&b).unwrap();
    app.navigate_player(PlayerScreen::Setup);
    let mut h = harness(app);
    assert!(h.query_by_label("Add Kickstart").is_none());
    click(&mut h, "Start");
    stop_and_check(&mut h, 0x31);
    common::command(
        h.state_mut(),
        HostCommand::WriteMemory {
            address: SLOW_RAM_ADDRESS,
            bytes: RAM_MARKER.to_vec(),
            restart_instruction: false,
        },
    )
    .value
    .unwrap();
    common::command(
        h.state_mut(),
        HostCommand::SetRegister {
            register: gui::EditRegister::D(7),
            value: 0x3132_3334,
        },
    )
    .value
    .unwrap();
    assert_eq!(
        common::read_memory(h.state_mut(), SLOW_RAM_ADDRESS, RAM_MARKER.len()),
        RAM_MARKER
    );
    let progress = common::snapshot(h.state_mut());
    let state = write(
        &temp.path().join("export.json"),
        common::snapshot_json(h.state_mut()),
    );
    let state_bytes = fs::read(&state).unwrap();
    click(&mut h, "Quick save");
    assert!(h.state().host.status.has_quicksave);

    click(&mut h, "Manual setup");
    choose_rom(&mut h, "B.rom");
    click(&mut h, "Return to session");
    assert_eq!(common::snapshot(h.state_mut()), progress);
    click(&mut h, "Resume");
    stop_and_check(&mut h, 0x31);
    click(&mut h, "Reset");
    stop_and_check(&mut h, 0x31);
    click(&mut h, "Manual setup");
    click(&mut h, "Start");
    click(&mut h, "Keep current session");
    assert_eq!(common::read_memory(h.state_mut(), 0xfc0200, 1), [0x31]);

    click(&mut h, "Configurations");
    click(&mut h, "Manage Kickstarts");
    h.query_all_by_label("Delete Kickstart")
        .next()
        .unwrap()
        .click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_role(Role::Dialog).get_by_label("Kickstart is used by saved configurations: First profile, Second profile. Change their Kickstart choices and save first.");
    assert!(root.join("kickstarts/A.rom").exists());
    click(&mut h, "Done");
    choose_rom(&mut h, "B.rom");
    choose_profile(&mut h, "First profile", "Second profile");
    choose_rom(&mut h, "B.rom");
    click(&mut h, "Save");
    click(&mut h, "Configurations");
    let saved_catalog = fs::read(&catalog).unwrap();
    click(&mut h, "Manage Kickstarts");
    h.query_all_by_label("Delete Kickstart")
        .next()
        .unwrap()
        .click_accesskit();
    common::settle_harness(&mut h);
    wait_for_dialog_button(&mut h, "Confirm deletion");
    h.get_by_role(Role::Dialog).get_by_label(
        "Saved state files may still need these exact Kickstart bytes from another source.",
    );
    click(&mut h, "Cancel deletion");
    assert!(root.join("kickstarts/A.rom").exists());
    h.query_all_by_label("Delete Kickstart")
        .next()
        .unwrap()
        .click_accesskit();
    common::settle_harness(&mut h);
    wait_for_dialog_button(&mut h, "Confirm deletion");
    click(&mut h, "Confirm deletion");
    assert!(!root.join("kickstarts/A.rom").exists());
    assert_eq!(fs::read(root.join("kickstarts/B.rom")).unwrap(), rom(0x64));
    assert_eq!(fs::read(&a).unwrap(), rom(0x31));
    assert_eq!(fs::read(&catalog).unwrap(), saved_catalog);
    assert_eq!(fs::read(&state).unwrap(), state_bytes);
    click(&mut h, "Done");
    click(&mut h, "Return to session");
    assert_eq!(common::read_memory(h.state_mut(), 0xfc0200, 1), [0x31]);
    assert_eq!(
        common::read_memory(h.state_mut(), SLOW_RAM_ADDRESS, RAM_MARKER.len()),
        [0, 0]
    );
    let generation = h.state().host.status.generation;
    click(&mut h, "Quick load");
    common::wait_harness(&mut h, |app| app.host.status.generation > generation);
    stop_and_check(&mut h, 0x31);
    assert_eq!(
        common::read_memory(h.state_mut(), SLOW_RAM_ADDRESS, RAM_MARKER.len()),
        RAM_MARKER
    );
    assert_eq!(common::snapshot(h.state_mut()).cpu.d_long(7), 0x3132_3334);

    let mut fresh = open(&root);
    let before = common::snapshot(&mut fresh);
    let status = fresh.host.status;
    let ctx = egui::Context::default();
    let managed_b = root.join("kickstarts/B.rom");
    let offline_b = temp.path().join("offline-B.rom");
    fs::rename(&managed_b, &offline_b).unwrap();
    for wanted in ["source is unavailable", "content does not match"] {
        fresh.load_state_file(&state);
        common::settle(&mut fresh, &ctx);
        let error = fresh.file_error.take().unwrap();
        assert!(
            error.contains("Kickstart ROM") && error.contains(wanted),
            "{error}"
        );
        assert_eq!(fresh.host.status.generation, status.generation);
        assert_eq!(fresh.host.status.running, status.running);
        assert_eq!(fresh.player.mounted, [None, None, None, None]);
        assert_eq!(common::snapshot(&mut fresh), before);
        if offline_b.exists() {
            fs::rename(&offline_b, &managed_b).unwrap();
        }
    }
    let renamed = write(&temp.path().join("renamed.bin"), fs::read(&a).unwrap());
    fresh.supply_restore_kickstart(&renamed).unwrap();
    fresh.load_state_file(&state);
    common::settle(&mut fresh, &ctx);
    assert!(fresh.file_error.is_none(), "{:?}", fresh.file_error);
    assert_eq!(common::snapshot(&mut fresh), progress);
    assert_eq!(
        fresh.player.catalog.selected().unwrap().launch.rom,
        "kickstarts/B.rom"
    );
    assert!(!root.join("kickstarts/A.rom").exists());
}

#[test]
fn deletion_rechecks_configurations_written_after_the_confirmation_opens() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    persist(&root, &two_profiles("", ""));
    let a = write(&temp.path().join("A.rom"), rom(0x63));
    let mut app = open(&root);
    app.import_kickstart(&a).unwrap();
    app.navigate_player(PlayerScreen::Configuration);
    let mut h = harness(app);
    click(&mut h, "Manage Kickstarts");
    click(&mut h, "Delete Kickstart");
    wait_for_dialog_button(&mut h, "Confirm deletion");
    let configurations = persist(&root, &two_profiles("", "kickstarts/A.rom"));
    let changed = fs::read(&configurations).unwrap();
    click(&mut h, "Confirm deletion");
    assert_eq!(fs::read(root.join("kickstarts/A.rom")).unwrap(), rom(0x63));
    assert_eq!(fs::read(configurations).unwrap(), changed);
    wait_for_dialog_button(&mut h, "Done");
    h.get_by_role(Role::Dialog).get_by_label("Kickstart is used by saved configurations: Second profile. Change their Kickstart choices and save first.");
}

#[test]
fn exact_size_custom_kickstarts_keep_the_preceding_selection_and_history() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    persist(&root, &two_profiles(ADOPTED, ""));
    let good = write(&temp.path().join("kickstart-1.3.rom"), rom(0x41));
    let mut app = open(&root);
    app.import_kickstart(&good).unwrap();
    let prior_history = app.file_history.clone();
    let selection = app.player.catalog.selected().unwrap().clone();
    let unchanged = common::snapshot(&mut app);
    for size in [262_143, 262_145, 524_288] {
        let source = write(&temp.path().join("invalid/other.rom"), vec![0x99; size]);
        let error = app.import_kickstart(&source).unwrap_err();
        assert!(
            error.contains("expected 262144 bytes") && error.contains(&format!("got {size} bytes"))
        );
        assert_eq!(app.file_history, prior_history);
        assert_eq!(app.player.catalog.selected().unwrap(), &selection);
        assert_eq!(common::snapshot(&mut app), unchanged);
        assert_eq!(fs::read_dir(root.join("kickstarts")).unwrap().count(), 1);
    }
    app.navigate_player(PlayerScreen::Setup);
    let mut h = harness(app);
    h.get_by_label("Unrecognized / custom");
    h.get_by_label("Emulator support unverified");
    click(&mut h, "Start");
    stop_and_check(&mut h, 0x41);
}

#[test]
fn manual_game_and_legacy_initial_state_use_their_own_firmware() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    persist(&root, &two_profiles("kickstarts/A.rom", "kickstarts/B.rom"));
    let a = write(&temp.path().join("A.rom"), rom(0x1a));
    let b = write(&temp.path().join("B.rom"), rom(0x2b));
    let manifest = game(&root, "Demo", "other", Some("initial.json"));
    let mut recorded = gui::DebuggerSession::new();
    recorded.load_rom(&rom(0x1a)).unwrap();
    recorded.machine.cpu.state.set_d_long(7, 0xdef0_1234);
    let mut snapshot =
        debugger::SaveState::capture(&recorded.machine, recorded.file_sources()).unwrap();
    snapshot.header.association.game_id = Some("Demo".into());
    write(
        &manifest.with_file_name("initial.json"),
        snapshot.to_json().unwrap(),
    );
    let mut app = open(&root);
    app.import_kickstart(&a).unwrap();
    app.import_kickstart(&b).unwrap();
    app.navigate_player(PlayerScreen::Setup);
    let mut h = harness(app);
    click(&mut h, "Start");
    stop_and_check(&mut h, 0x1a);
    click(&mut h, "Manual setup");
    choose_rom(&mut h, "B.rom");
    let before = h.state().host.status.generation;
    click(&mut h, "Start");
    assert_eq!(h.state().host.status.generation, before);
    assert_eq!(common::read_memory(h.state_mut(), 0xfc0200, 1), [0x1a]);
    click(&mut h, "Replace session");
    stop_and_check(&mut h, 0x2b);

    click(&mut h, "Manual setup");
    click(&mut h, "Library");
    common::wait_harness(&mut h, |app| !app.player.library.games.is_empty());
    click(&mut h, "Demo");
    click(&mut h, "Start game");
    click(&mut h, "Replace session");
    stop_and_check(&mut h, 0x1a);
    assert_eq!(common::snapshot(h.state_mut()).cpu.d_long(7), 0xdef0_1234);
    click(&mut h, "Manual setup");
    click(&mut h, "Library");
    click(&mut h, "Demo");
    let before = common::snapshot(h.state_mut());
    click(&mut h, "Boot normally without initial state");
    click(&mut h, "Keep current session");
    assert_eq!(common::snapshot(h.state_mut()), before);
    click(&mut h, "Boot normally without initial state");
    click(&mut h, "Replace session");
    stop_and_check(&mut h, 0x2b);
    assert_ne!(common::snapshot(h.state_mut()).cpu.d_long(7), 0xdef0_1234);
}

#[test]
#[ignore = "supply an authorized genuine image through AMIGA_TEST_KICKSTART"]
fn recognizes_authorized_kickstart_under_another_filename() {
    use sha1::Sha1;
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    fn digest_hex(bytes: &[u8]) -> String {
        let mut hex = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            write!(hex, "{byte:02x}").unwrap();
        }
        hex
    }

    let supplied = std::env::var_os("AMIGA_TEST_KICKSTART").unwrap();
    let mut bytes = fs::read(supplied).unwrap();
    assert_eq!(bytes.len(), SIZE);
    assert_eq!(
        digest_hex(&Sha1::digest(&bytes)),
        "891e9a547772fe0c6c19b610baf8bc4ea7fcb785"
    );
    assert_eq!(
        digest_hex(&Sha256::digest(&bytes)),
        "ee05862d8102a08436ac4056da7d549db31625c7d47b24dfb7b3c9a5c113ca53"
    );
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    persist(&root, &two_profiles("kickstarts/renamed.bin", ""));
    let genuine = write(&temp.path().join("renamed.bin"), &bytes);
    bytes[0x200] ^= 1;
    let custom = write(&temp.path().join("kickstart-1.3.rom"), &bytes);
    let mut app = open(&root);
    app.import_kickstart(&genuine).unwrap();
    app.import_kickstart(&custom).unwrap();
    app.navigate_player(PlayerScreen::Setup);
    let mut h = harness(app);
    h.get_by_label("Recognized official release");
    h.get_by_label("Supported (PAL A500 boot)");
    choose_rom(&mut h, "kickstart-1.3.rom");
    h.get_by_label("Unrecognized / custom");
    h.get_by_label("Emulator support unverified");
    choose_rom(
        &mut h,
        "Kickstart 1.3 rev 34.5 (A500/A1000/A2000) (renamed.bin)",
    );
    click(&mut h, "Start");
    common::wait_harness(&mut h, |app| {
        app.host.status.running && app.host.status.cck > 0
    });
    click(&mut h, "Pause");
    assert_eq!(
        common::snapshot(h.state_mut())
            .physical_memory
            .kickstart_rom(),
        fs::read(genuine).unwrap()
    );
}
