#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;
use common::TestContext as _;

use egui::{accesskit::Role, RawInput, ViewportCommand, ViewportEvent, ViewportId};
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use gui::game_library::{
    persist_quicksave, persist_save, GameLibrary, GameManifest, GameStartState, LibraryGame,
};
use gui::host_commands::HostCommand;
use gui::player::PlayerScreen;
use gui::{EmulatorApp, ViewMode};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

type PlayerHarness = Harness<'static, EmulatorApp>;

fn read_manifest(game: &LibraryGame) -> GameManifest {
    serde_json::from_slice(&fs::read(game.directory.join("game.json")).unwrap()).unwrap()
}

fn save_manifest(game: &LibraryGame) {
    fs::write(
        game.directory.join("game.json"),
        serde_json::to_vec(&game.manifest).unwrap(),
    )
    .unwrap();
}

fn state_file(game: &LibraryGame, entry: &GameStartState) -> PathBuf {
    game.directory
        .join(format!("initial_states/{}.json", entry.id))
}

fn add_state(game: &mut LibraryGame, suffix: &str, name: &str, bytes: Option<&[u8]>) {
    let entry = GameStartState {
        id: format!("start-{suffix}"),
        name: name.into(),
    };
    if let Some(bytes) = bytes {
        let path = state_file(game, &entry);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    game.manifest.start_states.push(entry);
    save_manifest(game);
}

fn fixture() -> (tempfile::TempDir, Vec<LibraryGame>) {
    let root = tempfile::tempdir().unwrap();
    let mut rom = vec![0; physical_memory::KICKSTART_ROM_SIZE];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    fs::write(root.path().join("kick.rom"), rom).unwrap();
    for id in ["First game", "Other game"] {
        let directory = root.path().join(id);
        fs::create_dir_all(directory.join("disks")).unwrap();
        fs::write(directory.join("disks/disk.adf"), vec![0x11; 901_120]).unwrap();
        fs::write(
            directory.join("game.json"),
            serde_json::to_vec(&serde_json::json!({
                "id": id, "title": id, "configuration": "standard",
                "disks": [{"id": "disk", "filename": "disk.adf"}],
                "mounted": ["disk", null, null, null],
                "custom": "keep"
            }))
            .unwrap(),
        )
        .unwrap();
    }
    let library = GameLibrary::discover(root.path()).unwrap();
    assert!(library.errors.is_empty(), "{:?}", library.errors);
    (root, library.games)
}

fn new_app(root: &Path) -> EmulatorApp {
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.player.catalog.configurations[0].launch.rom = "kick.rom".into();
    app.player.catalog.configurations[0].launch.drives[1] = true;
    let mut contents = serde_json::to_value(&app.player.catalog).unwrap();
    contents.as_object_mut().unwrap().remove("selected");
    fs::write(root.join("configurations.json"), contents.to_string()).unwrap();
    app.player.library_root = Some(root.to_path_buf());
    app.player.library = GameLibrary::discover(root).unwrap();
    app
}

fn harness(app: EmulatorApp) -> PlayerHarness {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 1000.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    common::settle_harness(&mut h);
    h
}

fn click(h: &mut PlayerHarness, label: &str) {
    h.get_by_label(label).click_accesskit();
    common::settle_harness(h);
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !ready() {
        assert!(
            Instant::now() < deadline,
            "The player did not reach the expected state"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn confirm_session_replacement(h: &mut PlayerHarness) {
    // The machine can be idle while egui is still sizing the confirmation dialog.
    wait_until(|| {
        h.step();
        h.query_by_role_and_label(Role::Button, "Replace session")
            .is_some_and(|button| !button.accesskit_node().is_disabled())
    });
    click(h, "Replace session");
}

fn library_game(h: &mut PlayerHarness, name: &str) {
    click(h, name);
    wait_until(|| {
        h.step();
        h.query_by_label("Edit game")
            .is_some_and(|button| !button.accesskit_node().is_disabled())
    });
}

fn name_entry(h: &mut PlayerHarness, name: &str) {
    h.get_by_role_and_label(Role::TextInput, "Start state name")
        .set_text_value_accesskit(name);
    common::settle_harness(h);
}

fn set_d7(app: &mut EmulatorApp, value: u32) {
    common::command(
        app,
        HostCommand::SetRegister {
            register: gui::EditRegister::D(7),
            value,
        },
    )
    .value
    .unwrap();
}

fn paused_game(root: &Path, game: &LibraryGame) -> EmulatorApp {
    let mut app = new_app(root);
    app.start_library_game(game.clone(), None, false);
    let ctx = egui::Context::default();
    common::settle(&mut app, &ctx);
    assert_eq!(app.player.screen, PlayerScreen::Session);
    assert!(app.host.status.running);
    common::command(&mut app, HostCommand::Pause).value.unwrap();
    common::wait(&mut app, &ctx, |app| !app.host.status.running);
    app
}

fn snapshot_with_time(app: &mut EmulatorApp, time: u64, marker: u32) -> debugger::SaveState {
    set_d7(app, marker);
    let mut saved = debugger::SaveState::from_json(&common::snapshot_json(app)).unwrap();
    saved.header.timestamp = time;
    saved
}

fn promote(h: &mut PlayerHarness, row: usize, name: &str) {
    h.get_all_by_label("Add to start states")
        .nth(row)
        .unwrap()
        .click_accesskit();
    common::settle_harness(h);
    name_entry(h, name);
    click(h, "Add start state");
}

#[test]
fn promotion_copies_ordinary_progress_and_current_quicksave_then_loads_the_named_copy() {
    let (root, games) = fixture();
    let game = &games[0];
    let mut app = paused_game(root.path(), game);
    let ordinary = snapshot_with_time(&mut app, 10, 0x1111_1111);
    let ordinary_json = ordinary.to_json().unwrap();
    let ordinary_file = persist_save(game, &ordinary_json).unwrap();
    let quick = snapshot_with_time(&mut app, 20, 0x2222_2222);
    let quick_json = quick.to_json_pretty().unwrap();
    let quick_file = persist_quicksave(game, &quick_json).unwrap();
    let pointer = quick_file.with_file_name("quicksave.txt");
    let original_pointer = fs::read(&pointer).unwrap();
    set_d7(&mut app, 0x3333_3333);
    let live = common::snapshot(&mut app);
    let mut h = harness(app);

    common::wait_harness(&mut h, |app| app.player.saves.len() == 2);
    h.get_by_label("Quicksave");
    h.get_all_by_label("Add to start states")
        .next()
        .unwrap()
        .click_accesskit();
    common::settle_harness(&mut h);
    name_entry(&mut h, "Cancelled");
    click(&mut h, "Cancel");
    assert!(read_manifest(game).start_states.is_empty());
    assert!(!game.directory.join("initial_states").exists());

    promote(&mut h, 1, "Intro");
    promote(&mut h, 0, "Level 1");
    let entries = read_manifest(game).start_states;
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        ["Intro", "Level 1"]
    );
    assert_ne!(entries[0].id, entries[1].id);
    assert_eq!(fs::read(&ordinary_file).unwrap(), ordinary_json.as_bytes());
    assert_eq!(fs::read(&quick_file).unwrap(), quick_json.as_bytes());
    assert_eq!(fs::read(&pointer).unwrap(), original_pointer);
    assert_eq!(common::snapshot(h.state_mut()), live);
    let copies = [state_file(game, &entries[0]), state_file(game, &entries[1])];
    assert_eq!(fs::read(&copies[0]).unwrap(), ordinary_json.as_bytes());
    assert_eq!(fs::read(&copies[1]).unwrap(), quick_json.as_bytes());

    h.get_all_by_label("Add to start states")
        .next()
        .unwrap()
        .click_accesskit();
    common::settle_harness(&mut h);
    name_entry(&mut h, "Stale quicksave");
    let replacement = persist_quicksave(game, &ordinary_json).unwrap();
    click(&mut h, "Add start state");
    assert_eq!(read_manifest(game).start_states, entries);
    assert!(h
        .query_all_by(|node| node
            .value()
            .is_some_and(|value| value.contains("The current quicksave changed")))
        .next()
        .is_some());
    click(&mut h, "Cancel");
    click(&mut h, "Library");
    library_game(&mut h, "First game");
    click(&mut h, "Rename Intro");
    name_entry(&mut h, "Menu");
    click(&mut h, "Save name");
    let amended = read_manifest(game).start_states;
    assert_eq!(amended[0].id, entries[0].id);
    assert_eq!(amended[0].name, "Menu");
    assert_eq!(amended[1], entries[1]);
    assert_eq!(fs::read(&copies[0]).unwrap(), ordinary_json.as_bytes());
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(game.directory.join("game.json")).unwrap()).unwrap();
    assert_eq!(metadata["custom"], "keep");

    fs::remove_file(ordinary_file).unwrap();
    fs::remove_file(quick_file).unwrap();
    assert_eq!(
        fs::read_to_string(pointer).unwrap(),
        replacement.file_name().unwrap().to_str().unwrap()
    );
    drop(h);
    let moved = root.path().join("Renamed folder");
    const ERROR_ACCESS_DENIED: i32 = 5;
    const ERROR_SHARING_VIOLATION: i32 = 32;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match fs::rename(&game.directory, &moved) {
            Ok(()) => break,
            // On Windows the background library watcher can still be releasing its handles.
            Err(error)
                if cfg!(windows)
                    && matches!(
                        error.raw_os_error(),
                        Some(ERROR_ACCESS_DENIED | ERROR_SHARING_VIOLATION)
                    )
                    && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("Could not move the game directory: {error}"),
        }
    }

    let mut app = new_app(root.path());
    app.navigate_player(PlayerScreen::Library);
    let mut h = harness(app);
    library_game(&mut h, "First game");
    assert!(h
        .query_by_label("Boot normally without initial state")
        .is_none());
    click(&mut h, "Start game");
    common::wait_harness(&mut h, |app| app.host.status.running);
    click(&mut h, "Pause");
    common::wait_harness(&mut h, |app| !app.host.status.running);
    assert_eq!(common::snapshot(h.state_mut()).cpu.d_long(7), 0);
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk"));

    click(&mut h, "Library");
    let generation = h.state().host.status.generation;
    click(&mut h, "Load Level 1");
    confirm_session_replacement(&mut h);
    common::wait_harness(&mut h, |app| {
        app.host.status.generation > generation && app.host.status.cck > quick.cck()
    });
    assert!(h.state().host.status.running);
    click(&mut h, "Pause");
    common::wait_harness(&mut h, |app| !app.host.status.running);
    assert_eq!(common::snapshot(h.state_mut()).cpu.d_long(7), 0x2222_2222);
}

#[test]
fn promotion_failures_and_unavailable_start_states_preserve_the_live_game_and_healthy_siblings() {
    let (root, mut games) = fixture();
    let mut app = paused_game(root.path(), &games[0]);
    let game = &mut games[0];
    let saved = snapshot_with_time(&mut app, 10, 0x1234);
    let json = saved.to_json().unwrap();
    let source = persist_save(game, &json).unwrap();
    fs::write(source.with_file_name("broken.json"), b"corrupt save").unwrap();
    let mut foreign = saved;
    foreign.header.association.game_id = Some("Other game".into());
    let foreign = foreign.to_json().unwrap();
    set_d7(&mut app, 0x4321);
    let before = common::snapshot(&mut app);
    let mut h = harness(app);
    click(&mut h, "Library");
    library_game(&mut h, "First game");
    assert_eq!(h.get_all_by_label("Add to start states").count(), 1);

    let manifest_path = game.directory.join("game.json");
    let baseline = fs::read(&manifest_path).unwrap();
    let assets = game.directory.join("initial_states");
    fs::write(&assets, b"blocked").unwrap();
    promote(&mut h, 0, "Intro");
    assert_eq!(fs::read(&manifest_path).unwrap(), baseline);
    assert_eq!(fs::read(&assets).unwrap(), b"blocked");
    fs::remove_file(&assets).unwrap();

    let manifest_blocker = game.directory.join("game.json.pending");
    fs::write(&manifest_blocker, b"blocked").unwrap();
    click(&mut h, "Add start state");
    assert_eq!(fs::read(&manifest_path).unwrap(), baseline);
    assert_eq!(fs::read(&manifest_blocker).unwrap(), b"blocked");
    assert_eq!(fs::read_dir(&assets).unwrap().count(), 0);
    fs::remove_file(&manifest_blocker).unwrap();

    fs::write(&source, foreign.as_bytes()).unwrap();
    click(&mut h, "Add start state");
    assert_eq!(fs::read(&manifest_path).unwrap(), baseline);
    assert_eq!(fs::read(&source).unwrap(), foreign.as_bytes());
    fs::remove_file(&source).unwrap();
    click(&mut h, "Add start state");
    assert_eq!(fs::read(&manifest_path).unwrap(), baseline);
    assert_eq!(fs::read_dir(&assets).unwrap().count(), 0);
    assert_eq!(common::snapshot(h.state_mut()), before);
    click(&mut h, "Cancel");

    add_state(game, "0000000000000001", "Missing", None);
    add_state(
        game,
        "0000000000000002",
        "Foreign",
        Some(foreign.as_bytes()),
    );
    add_state(
        game,
        "0000000000000003",
        "Corrupt",
        Some(b"broken snapshot"),
    );
    add_state(game, "0000000000000004", "Healthy", Some(json.as_bytes()));
    let mut incompatible: serde_json::Value = serde_json::from_str(&json).unwrap();
    let version = incompatible["header"]["version"].as_u64().unwrap();
    incompatible["header"]["version"] = (version - 1).into();
    add_state(
        game,
        "0000000000000005",
        "Old",
        Some(&serde_json::to_vec(&incompatible).unwrap()),
    );
    wait_until(|| {
        h.step();
        h.query_by_label("Load Healthy").is_some()
    });
    for label in ["Load Missing", "Load Foreign", "Load Corrupt", "Load Old"] {
        assert!(
            h.get_by_label(label).accesskit_node().is_disabled(),
            "{label}"
        );
    }
    assert!(!h
        .get_by_label("Load Healthy")
        .accesskit_node()
        .is_disabled());
    assert!(!h.get_by_label("Start game").accesskit_node().is_disabled());
    assert!(h
        .query_all_by(|node| node
            .value()
            .is_some_and(|value| value.contains("another game")))
        .next()
        .is_some());
}

#[test]
fn named_deletion_can_be_undone_after_navigation_and_close_cancels_it_before_the_deadline() {
    let (root, mut games) = fixture();
    let mut app = paused_game(root.path(), &games[0]);
    let (first, others) = games.split_at_mut(1);
    let game = &mut first[0];
    let other = &mut others[0];
    let saved = snapshot_with_time(&mut app, 20, 42).to_json().unwrap();
    let source = persist_quicksave(game, &saved).unwrap();
    let pointer = fs::read(source.with_file_name("quicksave.txt")).unwrap();
    let live = common::snapshot(&mut app);
    let mut h = harness(app);
    click(&mut h, "Library");
    library_game(&mut h, "First game");

    game.manifest.initial_state = Some("legacy.json".into());
    fs::write(game.directory.join("legacy.json"), &saved).unwrap();
    add_state(game, "0000000000000001", "Intro", Some(saved.as_bytes()));
    add_state(game, "0000000000000002", "Menu", Some(saved.as_bytes()));
    add_state(game, "0000000000000003", "Level 1", Some(saved.as_bytes()));
    let intro = state_file(game, &game.manifest.start_states[0]);
    let menu = state_file(game, &game.manifest.start_states[1]);
    let level = state_file(game, &game.manifest.start_states[2]);
    wait_until(|| {
        h.step();
        h.query_by_label("Load Level 1")
            .is_some_and(|button| !button.accesskit_node().is_disabled())
    });
    h.get_by_label("Boot normally without initial state");

    click(&mut h, "Delete Intro");
    assert!(h.get_by_label("Load Intro").accesskit_node().is_disabled());
    click(&mut h, "Other game");
    other.manifest.title = "Updated other game".into();
    save_manifest(other);
    wait_until(|| {
        h.step();
        h.query_all_by_label("Updated other game").count() > 0
    });
    assert_eq!(read_manifest(game).start_states, game.manifest.start_states);
    assert_eq!(fs::read(&intro).unwrap(), saved.as_bytes());
    click(&mut h, "Undo");
    library_game(&mut h, "First game");
    assert!(!h.get_by_label("Load Intro").accesskit_node().is_disabled());

    click(&mut h, "Delete Menu");
    click(&mut h, "Updated other game");
    wait_until(|| {
        h.step();
        !menu.exists()
    });
    assert_eq!(
        read_manifest(game).start_states,
        [
            game.manifest.start_states[0].clone(),
            game.manifest.start_states[2].clone()
        ]
    );
    assert!(h.query_by_label("Undo").is_none());
    assert_eq!(common::snapshot(h.state_mut()), live);

    library_game(&mut h, "First game");
    click(&mut h, "Delete Level 1");
    assert!(level.exists());
    let ctx = h.ctx.clone();
    let app = h.state_mut();
    let mut input = RawInput::default();
    input
        .viewports
        .get_mut(&ViewportId::ROOT)
        .unwrap()
        .events
        .push(ViewportEvent::Close);
    let requested = ctx.run_test_ui(input, |ui| app.update_ui(ui));
    let commands = &requested.viewport_output[&ViewportId::ROOT].commands;
    assert!(commands.contains(&ViewportCommand::CancelClose));
    if !commands.contains(&ViewportCommand::Close) {
        wait_until(|| {
            let output = ctx.run_test_ui(RawInput::default(), |ui| app.update_ui(ui));
            output.viewport_output[&ViewportId::ROOT]
                .commands
                .contains(&ViewportCommand::Close)
        });
    }
    assert!(app.host.status.shutdown);
    assert_eq!(fs::read(&level).unwrap(), saved.as_bytes());
    let reopened = GameLibrary::discover(root.path()).unwrap();
    assert_eq!(
        reopened.games[0].manifest.start_states,
        [
            game.manifest.start_states[0].clone(),
            game.manifest.start_states[2].clone()
        ]
    );
    assert_eq!(fs::read(&intro).unwrap(), saved.as_bytes());
    assert_eq!(
        fs::read(game.directory.join("legacy.json")).unwrap(),
        saved.as_bytes()
    );
    assert_eq!(fs::read(&source).unwrap(), saved.as_bytes());
    assert_eq!(
        fs::read(source.with_file_name("quicksave.txt")).unwrap(),
        pointer
    );
    assert!(reopened.games[1].manifest.start_states.is_empty());

    let mut reopened = harness(new_app(root.path()));
    click(&mut reopened, "Library");
    library_game(&mut reopened, "First game");
    assert!(!reopened
        .get_by_label("Load Level 1")
        .accesskit_node()
        .is_disabled());
    assert!(reopened.query_by_label("Undo").is_none());
}

#[test]
fn named_state_with_missing_kickstart_reports_it_only_when_loaded() {
    let (root, mut games) = fixture();
    let game = &mut games[0];
    let mut app = paused_game(root.path(), game);
    let saved = snapshot_with_time(&mut app, 20, 42).to_json().unwrap();
    drop(app);
    add_state(game, "0000000000000001", "Intro", Some(saved.as_bytes()));

    let original = root.path().join("kick.rom");
    let replacement = root.path().join("replacement.rom");
    let mut other_rom = fs::read(&original).unwrap();
    other_rom[0x202] = 1;
    fs::write(&replacement, other_rom).unwrap();
    fs::remove_file(&original).unwrap();
    let mut app = new_app(root.path());
    app.player.catalog.configurations[0].launch.rom = "replacement.rom".into();
    let mut contents = serde_json::to_value(&app.player.catalog).unwrap();
    contents.as_object_mut().unwrap().remove("selected");
    fs::write(
        root.path().join("configurations.json"),
        contents.to_string(),
    )
    .unwrap();
    let mut h = harness(app);
    click(&mut h, "Library");
    library_game(&mut h, "First game");

    let generation = h.state().host.status.generation;
    assert!(!h.get_by_label("Load Intro").accesskit_node().is_disabled());
    assert!(h.state().file_error.is_none());
    click(&mut h, "Load Intro");
    common::wait_harness(&mut h, |app| app.file_error.is_some());
    assert_eq!(
        h.state().file_error.as_deref(),
        Some("Kickstart ROM (kick.rom) content does not match the save")
    );
    h.get_by_label("File operation failed");
    assert_eq!(h.state().host.status.generation, generation);
    assert!(!h.state().host.status.running);
    assert_eq!(read_manifest(game).start_states, game.manifest.start_states);
    assert_eq!(
        fs::read(state_file(game, &game.manifest.start_states[0])).unwrap(),
        saved.as_bytes()
    );
}

#[test]
fn loading_a_named_state_can_repair_its_games_missing_configuration() {
    let (root, mut games) = fixture();
    let mut app = paused_game(root.path(), &games[0]);
    let mut saved = snapshot_with_time(&mut app, 9, 0x3456);
    let live = common::snapshot(&mut app);
    let mut h = harness(app);
    click(&mut h, "Library");
    library_game(&mut h, "Other game");

    let game = &mut games[1];
    saved.header.association.game_id = Some(game.manifest.id.clone());
    game.manifest.configuration = "missing".into();
    add_state(
        game,
        "0000000000000001",
        "Intro",
        Some(saved.to_json().unwrap().as_bytes()),
    );
    wait_until(|| {
        h.step();
        h.query_by_label("Load Intro")
            .is_some_and(|button| !button.accesskit_node().is_disabled())
    });

    click(&mut h, "Load Intro");
    confirm_session_replacement(&mut h);
    click(&mut h, "Cancel");
    assert_eq!(read_manifest(game).configuration, "missing");
    assert_eq!(common::snapshot(h.state_mut()), live);

    click(&mut h, "Load Intro");
    confirm_session_replacement(&mut h);
    h.get_by_role_and_label(Role::ComboBox, "Game configuration")
        .click();
    common::settle_harness(&mut h);
    h.get_by_role_and_label(Role::Button, "Standard A500")
        .click();
    common::settle_harness(&mut h);
    let generation = h.state().host.status.generation;
    click(&mut h, "Save reference and continue");
    common::wait_harness(&mut h, |app| app.host.status.generation > generation);
    assert!(h.state().host.status.running);
    assert_eq!(read_manifest(game).configuration, "standard");
    assert_eq!(
        h.state().player.active_game.as_ref().unwrap().manifest.id,
        game.manifest.id
    );
    click(&mut h, "Pause");
    common::wait_harness(&mut h, |app| !app.host.status.running);
    assert_eq!(common::snapshot(h.state_mut()).cpu.d_long(7), 0x3456);
}
