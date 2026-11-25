#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use egui::accesskit::Role;
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use gui::game_library::{GameDisk, GameLibrary, GameManifest};
use gui::host_commands::HostCommand;
use gui::player::PlayerScreen;
use gui::player_config::{ConfigurationCatalog, InputSource, NamedConfiguration};
use gui::EmulatorApp;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn catalog_path(root: &Path) -> PathBuf {
    root.join("configurations.json")
}

fn library(root: &Path, catalog: &ConfigurationCatalog, marker: u8) {
    let mut rom = vec![0; 262144];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    rom[0x200] = marker;
    fs::write(root.join("kick.rom"), rom).unwrap();
    write_catalog(root, catalog);
}

fn write_catalog(root: &Path, catalog: &ConfigurationCatalog) {
    let mut contents = serde_json::to_value(catalog).unwrap();
    contents.as_object_mut().unwrap().remove("selected");
    fs::write(catalog_path(root), contents.to_string()).unwrap();
}

fn game(root: &Path, title: &str, configuration: Option<&str>, startup: Option<usize>) -> PathBuf {
    let directory = root.join(title);
    fs::create_dir_all(directory.join("disks")).unwrap();
    let mut manifest = GameManifest {
        id: title.into(),
        title: title.into(),
        configuration: configuration.unwrap_or_default().into(),
        disks: vec![GameDisk {
            id: "disk-1".into(),
            filename: "disk.adf".into(),
        }],
        mounted: [None, None, None, None],
        initial_state: None,
        artwork: None,
        description: None,
    };
    if let Some(drive) = startup {
        manifest.mounted[drive] = Some("disk-1".into());
    }
    fs::write(directory.join("disks/disk.adf"), vec![0x17; 901120]).unwrap();
    let mut document = serde_json::to_value(manifest).unwrap();
    if configuration.is_none() {
        document.as_object_mut().unwrap().remove("configuration");
    }
    document["custom_metadata"] = json!({"editor": "fixture"});
    let path = directory.join("game.json");
    fs::write(&path, document.to_string()).unwrap();
    path
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn fixture_catalog() -> ConfigurationCatalog {
    let mut catalog = ConfigurationCatalog::default();
    catalog.configurations[0].launch.rom = "kick.rom".into();
    catalog
}

fn browser() -> Harness<'static, EmulatorApp> {
    let mut app = EmulatorApp::default();
    app.navigate_player(PlayerScreen::Library);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1024.0, 900.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    common::settle_harness(&mut h);
    h
}

fn click(h: &mut Harness<'_, EmulatorApp>, label: &str) {
    h.get_by_label(label).click_accesskit();
    common::settle_harness(h);
}

fn open_dialog(h: &mut Harness<'_, EmulatorApp>, trigger: &str, dialog_button: &str) {
    click(h, trigger);
    // A newly opened dialog can still be in egui's invisible sizing pass after the host is idle.
    let deadline = Instant::now() + Duration::from_secs(10);
    while h
        .query_by_role_and_label(Role::Button, dialog_button)
        .is_none_or(|button| button.accesskit_node().is_hidden())
    {
        assert!(
            Instant::now() < deadline,
            "Dialog button {dialog_button:?} did not appear after clicking {trigger:?}"
        );
        h.step();
    }
}

fn choose(h: &mut Harness<'_, EmulatorApp>, field: &str, choice: &str) {
    h.get_by_role_and_label(Role::ComboBox, field).click();
    common::settle_harness(h);
    h.get_by_role_and_label(Role::Button, choice).click();
    common::settle_harness(h);
}

fn shown(h: &Harness<'_, EmulatorApp>, text: &str) {
    assert!(
        h.query_all_by(|node| node.value().is_some_and(|value| value.contains(text)))
            .next()
            .is_some(),
        "Missing text {text:?}",
    );
}

fn wait_for_profile_editor(h: &mut Harness<'_, EmulatorApp>, visible: bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while h.query_by_label("Create configuration").is_some() != visible {
        assert!(Instant::now() < deadline, "Profile editor did not update");
        common::settle_harness(h);
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn changing_library_cancels_dirty_selection_and_keeps_original_machine_and_mappings() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let a = fixture_catalog();
    let mut b = a.clone();
    b.mappings[0].keys[0] = 0x17;
    b.configurations[0].name = "Second library profile".into();
    b.configurations[0].controls.ports[1].source = InputSource::Keyboard("wasd".into());
    b.configurations[0].controls.host.mouse_sensitivity = 1.5;
    library(first.path(), &a, 0x11);
    library(second.path(), &b, 0x22);
    let first_manifest = game(first.path(), "Shared game", Some("standard"), Some(0));
    let second_manifest = game(second.path(), "Shared game", Some("standard"), None);
    let before_b = fs::read(catalog_path(second.path())).unwrap();
    let mut h = browser();
    assert!(h.get_by_label("Add game").accesskit_node().is_disabled());
    common::select_library(&mut h, first.path());
    common::wait_harness(&mut h, |app| app.player.library.games.len() == 1);
    click(&mut h, "Shared game");
    click(&mut h, "Start game");
    common::wait_harness(&mut h, |app| app.player.active_game.is_some());
    common::command(h.state_mut(), HostCommand::Pause);
    let snapshot = common::snapshot(h.state_mut());
    let save = gui::game_library::persist_quicksave(
        &GameLibrary::discover(first.path()).unwrap().games[0],
        &common::snapshot_json(h.state_mut()),
    )
    .unwrap();
    let saved_bytes = fs::read(&save).unwrap();

    h.state_mut().navigate_player(PlayerScreen::Configuration);
    common::settle_harness(&mut h);
    h.get_by_label("Choose library directory...");
    let editor = h.state_mut().player.editor.as_mut().unwrap();
    editor.catalog.configurations[0].name = "Uncommitted first profile".into();
    editor.library_root = Some(second.path().to_path_buf());
    click(&mut h, "Save");
    shown(&h, "Finish or discard your profile edits");
    assert_eq!(h.state().player.library_root.as_deref(), Some(first.path()));
    assert_eq!(h.state().player.catalog, a);
    assert_eq!(fs::read(catalog_path(second.path())).unwrap(), before_b);
    let mut external = a.clone();
    external.configurations[0].controls.host.mouse_sensitivity = 1.75;
    external.mappings[0].keys[0] = 0x17;
    write_catalog(first.path(), &external);
    common::wait_harness(&mut h, |app| app.player.catalog == external);
    let editor = h.state().player.editor.as_ref().unwrap();
    assert_eq!(
        editor.catalog.configurations[0].name,
        "Uncommitted first profile"
    );
    assert_eq!(editor.library_root.as_deref(), Some(second.path()));
    assert_eq!(h.state().player.active_configuration.as_ref(), a.selected());
    assert_eq!(h.state().player.active_mappings, a.mappings);
    assert_eq!(common::snapshot(h.state_mut()), snapshot);
    click(&mut h, "Cancel");
    click(&mut h, "Library");
    common::select_library(&mut h, second.path());
    assert_eq!(h.state().player.catalog, b);
    assert_eq!(h.state().player.active_mappings, a.mappings);
    assert_eq!(
        &h.state().player.active_game.as_ref().unwrap().directory,
        first_manifest.parent().unwrap()
    );
    assert_eq!(common::snapshot(h.state_mut()), snapshot);

    click(&mut h, "Configurations");
    click(&mut h, "Arrow keys");
    assert!(!h
        .get_by_label("Delete mapping")
        .accesskit_node()
        .is_disabled());
    click(&mut h, "Delete mapping");
    h.state_mut()
        .player
        .editor
        .as_mut()
        .unwrap()
        .catalog
        .configurations[0]
        .controls
        .host
        .mouse_sensitivity = 2.0;
    click(&mut h, "Save");
    assert_eq!(
        h.state()
            .player
            .active_configuration
            .as_ref()
            .unwrap()
            .controls
            .host
            .mouse_sensitivity,
        1.0
    );
    assert_eq!(h.state().player.active_mappings, a.mappings);
    assert!(h.state().player.catalog.mapping("arrows").is_none());
    assert_eq!(
        h.state().player.catalog.configurations[0]
            .controls
            .host
            .mouse_sensitivity,
        2.0
    );
    assert_eq!(common::snapshot(h.state_mut()), snapshot);

    click(&mut h, "Return to session");
    assert_eq!(fs::read(&save).unwrap(), saved_bytes);
    assert!(!second_manifest.parent().unwrap().join("saves").exists());
    click(&mut h, "Library");
    click(&mut h, "Shared game");
    click(&mut h, "Start game");
    click(&mut h, "Replace session");
    common::wait_harness(&mut h, |app| app.player.active_mappings == b.mappings[1..]);
    common::command(h.state_mut(), HostCommand::Pause);
    common::settle_harness(&mut h);
    assert_eq!(common::read_memory(h.state_mut(), 0xfc0200, 1), [0x22]);
}

#[test]
fn occupied_catalog_preserves_the_draft_and_machine_before_owner_reconfiguration() {
    use debugger::launch::DeviceKind;
    let root = tempfile::tempdir().unwrap();
    let catalog = fixture_catalog();
    library(root.path(), &catalog, 0);
    let mut h = browser();
    common::select_library(&mut h, root.path());
    h.state_mut().start_manual();
    common::wait_harness(&mut h, |app| app.player.active_configuration.is_some());
    h.state_mut().navigate_player(PlayerScreen::Configuration);
    common::settle_harness(&mut h);
    let before = common::snapshot(h.state_mut());
    let generation = h.state().host.status.generation;
    let path = catalog_path(root.path());
    let committed = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    let profile = &mut h
        .state_mut()
        .player
        .editor
        .as_mut()
        .unwrap()
        .catalog
        .configurations[0];
    profile.controls.ports[0] = gui::player_config::PortBinding {
        device: DeviceKind::Disconnected,
        source: gui::player_config::InputSource::None,
    };
    let draft = h.state().player.editor.as_ref().unwrap().catalog.clone();
    wait_for_profile_editor(&mut h, false);
    click(&mut h, "Save");
    shown(&h, "Could not prepare configuration save");
    assert_eq!(h.state().player.editor.as_ref().unwrap().catalog, draft);
    assert_eq!(h.state().player.catalog, catalog);
    assert_eq!(h.state().host.status.generation, generation);
    assert_eq!(common::snapshot(h.state_mut()), before);
    assert!(path.is_dir());
    fs::remove_dir(&path).unwrap();
    fs::write(&path, committed).unwrap();
    wait_for_profile_editor(&mut h, true);
    click(&mut h, "Save");
    assert!(h.state().player.editor.is_none());
    assert_eq!(h.state().host.status.ports[0], DeviceKind::Disconnected);
    assert_eq!(h.state().player.catalog, draft);
}

#[test]
fn absent_reference_is_listed_and_cancel_or_failed_write_preserves_the_live_session() {
    let root = tempfile::tempdir().unwrap();
    let catalog = fixture_catalog();
    library(root.path(), &catalog, 0x19);
    game(root.path(), "Current game", Some("standard"), None);
    let manifest = game(root.path(), "Repair game", None, Some(0));
    let before_manifest = fs::read(&manifest).unwrap();
    let mut h = browser();
    common::select_library(&mut h, root.path());
    common::wait_harness(&mut h, |app| app.player.library.games.len() == 2);
    click(&mut h, "Current game");
    click(&mut h, "Start game");
    common::wait_harness(&mut h, |app| app.player.active_game.is_some());
    click(&mut h, "Library");
    let before = common::snapshot(h.state_mut());
    let generation = h.state().host.status.generation;
    click(&mut h, "Repair game");
    open_dialog(&mut h, "Start game", "Save reference and continue");
    assert!(h
        .query_all_by_value("Configuration missing")
        .next()
        .is_some());
    assert!(h
        .get_by_label("Save reference and continue")
        .accesskit_node()
        .is_disabled());
    click(&mut h, "Cancel");
    assert_eq!(common::snapshot(h.state_mut()), before);
    assert_eq!(fs::read(&manifest).unwrap(), before_manifest);

    open_dialog(&mut h, "Start game", "Save reference and continue");
    choose(&mut h, "Game configuration", "Standard A500");
    let block = manifest.with_extension("json.pending");
    fs::write(&block, b"occupied").unwrap();
    click(&mut h, "Save reference and continue");
    shown(&h, "Could not save the configuration reference");
    assert_eq!(fs::read(&manifest).unwrap(), before_manifest);
    assert_eq!(fs::read(&block).unwrap(), b"occupied");
    assert_eq!(h.state().host.status.generation, generation);
    assert_eq!(common::snapshot(h.state_mut()), before);
    fs::remove_file(&block).unwrap();
    click(&mut h, "Save reference and continue");
    let mut expected: Value = serde_json::from_slice(&before_manifest).unwrap();
    expected["configuration"] = "standard".into();
    assert_eq!(read_json(&manifest), expected);
    click(&mut h, "Replace session");
    common::wait_harness(&mut h, |app| {
        app.player
            .active_game
            .as_ref()
            .is_some_and(|game| game.manifest.title == "Repair game")
    });
    assert!(h.state().host.status.drives[0].inserted);
    assert_eq!(
        GameLibrary::discover(root.path()).unwrap().games[1]
            .manifest
            .configuration,
        "standard"
    );
}

#[test]
fn deletion_rejects_duplicate_ids_and_a_game_changed_after_confirmation_opens() {
    let root = tempfile::tempdir().unwrap();
    let mut catalog = fixture_catalog();
    let mut replacement = catalog.configurations[0].clone();
    replacement.id = "replacement".into();
    replacement.name = "Replacement A500".into();
    catalog.configurations.push(replacement);
    library(root.path(), &catalog, 0);
    let affected = game(root.path(), "Affected game", Some("standard"), Some(0));
    let unrelated = game(root.path(), "Unrelated game", Some("replacement"), None);
    let mut duplicate = read_json(&unrelated);
    duplicate["id"] = json!("Affected game");
    fs::write(&unrelated, duplicate.to_string()).unwrap();
    let original = fs::read(catalog_path(root.path())).unwrap();
    let mut h = browser();
    common::select_library(&mut h, root.path());
    click(&mut h, "Configurations");
    click(&mut h, "Delete configuration");
    shown(&h, "Duplicate game ID: Affected game");
    assert!(h.query_by_label("Confirm deletion").is_none());
    assert_eq!(fs::read(catalog_path(root.path())).unwrap(), original);

    fs::remove_file(unrelated).unwrap();
    open_dialog(&mut h, "Delete configuration", "Confirm deletion");
    shown(&h, "Affected game");
    h.get_by_role_and_label(Role::ComboBox, "Replacement configuration")
        .click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_role_and_label(Role::Button, "Replacement A500")
        .click_accesskit();
    common::settle_harness(&mut h);
    let mut revised = read_json(&affected);
    revised["title"] = json!("New title on disk");
    fs::write(&affected, revised.to_string()).unwrap();
    click(&mut h, "Confirm deletion");
    shown(&h, "Affected games changed since confirmation");
    shown(&h, "No game references were written.");
    assert_eq!(read_json(&affected), revised);
    assert_eq!(fs::read(catalog_path(root.path())).unwrap(), original);
}

#[test]
fn reassignment_validates_all_games_and_reports_partial_writes_before_retry() {
    let root = tempfile::tempdir().unwrap();
    let mut catalog = fixture_catalog();
    catalog.configurations[0].launch.drives[1] = true;
    let mut replacement = catalog.configurations[0].clone();
    replacement.id = "replacement".into();
    replacement.name = "Replacement A500".into();
    replacement.launch.drives[1] = false;
    catalog.configurations.push(replacement);
    catalog.configurations.push(NamedConfiguration {
        id: "spare".into(),
        name: "Another A500".into(),
        ..NamedConfiguration::default()
    });
    library(root.path(), &catalog, 0);
    let a = game(root.path(), "First game", Some("standard"), Some(0));
    let b = game(root.path(), "Second game", Some("standard"), Some(1));
    let c = game(root.path(), "Unrelated game", Some("spare"), None);
    let unrelated = fs::read(&c).unwrap();
    let save = a.parent().unwrap().join("progress.bin");
    fs::write(&save, b"saved progress").unwrap();
    let original = fs::read(catalog_path(root.path())).unwrap();
    let mut h = browser();
    common::select_library(&mut h, root.path());
    click(&mut h, "Configurations");
    open_dialog(&mut h, "Delete configuration", "Confirm deletion");
    shown(&h, "Delete Standard A500?");
    shown(&h, "First game");
    shown(&h, "Second game");
    assert!(h
        .get_by_label("Confirm deletion")
        .accesskit_node()
        .is_disabled());
    click(&mut h, "Cancel deletion");
    assert_eq!(fs::read(catalog_path(root.path())).unwrap(), original);
    open_dialog(&mut h, "Delete configuration", "Confirm deletion");
    choose(&mut h, "Replacement configuration", "Replacement A500");
    click(&mut h, "Confirm deletion");
    shown(&h, "DF1 is disconnected");
    assert_eq!(read_json(&a)["configuration"], "standard");
    assert_eq!(read_json(&b)["configuration"], "standard");
    assert_eq!(fs::read(catalog_path(root.path())).unwrap(), original);

    // The watcher reloads an outside catalog edit while the clean editor is open.
    catalog.configurations[1].launch.drives[1] = true;
    write_catalog(root.path(), &catalog);
    common::wait_harness(&mut h, |app| {
        app.player
            .editor
            .as_ref()
            .is_some_and(|editor| editor.catalog == catalog)
    });
    let mut expected_a = read_json(&a);
    let mut expected_b = read_json(&b);
    let block = b.with_extension("json.pending");
    fs::write(&block, b"occupied").unwrap();
    open_dialog(&mut h, "Delete configuration", "Confirm deletion");
    choose(&mut h, "Replacement configuration", "Replacement A500");
    click(&mut h, "Confirm deletion");
    shown(&h, "Game references written before the failure: First game");
    expected_a["configuration"] = "replacement".into();
    assert_eq!(read_json(&a), expected_a);
    assert_eq!(read_json(&b), expected_b);
    assert_eq!(h.state().player.catalog, catalog);
    assert_eq!(fs::read(&block).unwrap(), b"occupied");
    common::wait_harness(&mut h, |app| {
        app.player.library.games.iter().any(|game| {
            game.manifest.title == "First game" && game.manifest.configuration == "replacement"
        })
    });

    fs::remove_file(block).unwrap();
    open_dialog(&mut h, "Delete configuration", "Confirm deletion");
    shown(&h, "Second game");
    assert!(h.query_by_value("First game").is_none());
    choose(&mut h, "Replacement configuration", "Replacement A500");
    click(&mut h, "Confirm deletion");
    assert_eq!(read_json(&a), expected_a);
    expected_b["configuration"] = "replacement".into();
    assert_eq!(read_json(&b), expected_b);
    assert_eq!(fs::read(&c).unwrap(), unrelated);
    assert_eq!(fs::read(&save).unwrap(), b"saved progress");
    assert_eq!(
        fs::read(a.parent().unwrap().join("disks/disk.adf")).unwrap(),
        vec![0x17; 901120]
    );
    let contents = read_json(&catalog_path(root.path()));
    assert_eq!(
        contents["configurations"],
        json!(&catalog.configurations[1..])
    );
    assert_eq!(contents["mappings"], json!(catalog.mappings));
}

#[cfg(windows)]
#[test]
fn final_catalog_write_failure_keeps_old_profile_and_reports_all_reassigned_games() {
    use std::os::windows::fs::OpenOptionsExt;

    let root = tempfile::tempdir().unwrap();
    let mut catalog = fixture_catalog();
    let mut replacement = catalog.configurations[0].clone();
    replacement.id = "replacement".into();
    replacement.name = "Replacement A500".into();
    catalog.configurations.push(replacement);
    library(root.path(), &catalog, 0);
    let first = game(root.path(), "First game", Some("standard"), Some(0));
    let second = game(root.path(), "Second game", Some("standard"), Some(0));
    let path = catalog_path(root.path());
    let original = fs::read(&path).unwrap();
    let expected = [&first, &second].map(|manifest| {
        let mut document = read_json(manifest);
        document["configuration"] = "replacement".into();
        document
    });
    let mut h = browser();
    common::select_library(&mut h, root.path());
    click(&mut h, "Configurations");
    open_dialog(&mut h, "Delete configuration", "Confirm deletion");
    h.get_by_role_and_label(Role::ComboBox, "Replacement configuration")
        .click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_role_and_label(Role::Button, "Replacement A500")
        .click_accesskit();
    common::settle_harness(&mut h);

    // FILE_SHARE_READ permits the catalog checks and reload but refuses replacement.
    const FILE_SHARE_READ: u32 = 1;
    let held_catalog = fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&path)
        .unwrap();
    click(&mut h, "Confirm deletion");
    shown(&h, "Kept the configuration because deletion stopped");
    shown(
        &h,
        "Game references written before the failure: First game, Second game.",
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(h.state().player.catalog, catalog);
    assert_eq!([read_json(&first), read_json(&second)], expected);
    common::wait_harness(&mut h, |app| {
        app.player.library.games.len() == 2
            && app
                .player
                .library
                .games
                .iter()
                .all(|game| game.manifest.configuration == "replacement")
    });
    let games = &h.state().player.library.games;
    assert_eq!(games.len(), 2);
    assert!(games
        .iter()
        .all(|game| game.manifest.configuration == "replacement"));

    drop(held_catalog);
    open_dialog(&mut h, "Delete configuration", "Confirm deletion");
    shown(&h, "No games in this library use this configuration.");
    click(&mut h, "Confirm deletion");
    assert_eq!(
        read_json(&path)["configurations"],
        json!(&catalog.configurations[1..])
    );
    assert_eq!([read_json(&first), read_json(&second)], expected);
}

#[test]
fn last_unreferenced_profile_can_be_deleted_and_repair_guides_the_player_to_create_one() {
    let root = tempfile::tempdir().unwrap();
    library(root.path(), &fixture_catalog(), 0);
    let game_manifest = game(root.path(), "Unavailable profile", Some("obsolete"), None);
    let before = fs::read(&game_manifest).unwrap();
    let mut h = browser();
    common::select_library(&mut h, root.path());
    common::wait_harness(&mut h, |app| app.player.library.games.len() == 1);
    click(&mut h, "Configurations");
    open_dialog(&mut h, "Delete configuration", "Confirm deletion");
    shown(&h, "No games in this library use this configuration.");
    click(&mut h, "Confirm deletion");
    assert!(h.state().player.catalog.configurations.is_empty());
    assert_eq!(
        read_json(&catalog_path(root.path()))["configurations"],
        json!([])
    );
    click(&mut h, "Cancel");
    click(&mut h, "Unavailable profile");
    open_dialog(&mut h, "Start game", "Open Configurations");
    shown(&h, "Create a configuration for this library");
    click(&mut h, "Open Configurations");
    assert_eq!(h.state().player.screen, PlayerScreen::Configuration);
    assert_eq!(fs::read(game_manifest).unwrap(), before);
}
