#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]
mod common;
use egui_kittest::kittest::NodeT;
use gui::game_library::{GameDisk, GameLibrary, GameManifest, LibraryGame};
use gui::host_commands::HostCommand;
use gui::player::PlayerScreen;
use gui::{EmulatorApp, ViewMode};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.agent/tmp/player-workflow")
        .join(format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}
fn rom() -> Vec<u8> {
    let mut rom = vec![0; 262144];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    rom
}
fn library() -> (PathBuf, GameLibrary) {
    let root = root();
    std::fs::write(root.join("kick.rom"), rom()).unwrap();
    for id in ["one", "two"] {
        let directory = root.join(id);
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(
            directory.join("disk.adf"),
            vec![if id == "one" { 0x11 } else { 0x22 }; 901120],
        )
        .unwrap();
        let manifest = GameManifest {
            id: id.into(),
            title: id.into(),
            configuration: "standard".into(),
            disks: vec![GameDisk {
                id: "disk-one".into(),
                label: "Disk 1".into(),
                path: "disk.adf".into(),
            }],
            mounted: [Some("disk-one".into()), None, None, None],
            initial_state: None,
            artwork: Some("missing-cover.png".into()),
            description: None,
        };
        std::fs::write(
            directory.join("game.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
    }
    let library = GameLibrary::discover(&root).unwrap();
    assert!(library.errors.is_empty());
    assert_eq!(library.games.len(), 2);
    (root, library)
}
#[test]
fn launch_save_reopen_relocate_and_restore_embedded_media_without_original_assets() {
    let (root, library) = library();
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.player.catalog.configurations[0].launch.rom =
        root.join("kick.rom").to_string_lossy().into_owned();
    app.start_library_game(library.games[0].clone(), None, false);
    let ctx = egui::Context::default();
    common::settle(&mut app, &ctx);
    assert!(app.host.status.running);
    assert!(app.host.status.drives[0].inserted);
    assert_eq!(app.player.screen, PlayerScreen::Session);
    common::command(&mut app, HostCommand::Pause);
    common::wait(&mut app, &ctx, |app| !app.host.status.running);
    let json = common::snapshot_json(&mut app);
    let state = debugger::SaveState::from_json(&json).unwrap();
    assert!(state.header.timestamp > 0);
    assert!(!state.header.screenshot_png.is_empty());
    assert_eq!(state.header.association.game_id.as_deref(), Some("one"));
    let first = gui::game_library::persist_save(&library.games[0], &json, Some(1)).unwrap();
    let second = gui::game_library::persist_save(&library.games[0], &json, None).unwrap();
    assert_ne!(first, second);
    assert_eq!(
        gui::game_library::saves(&library.games[0]).unwrap().len(),
        2
    );
    assert!(gui::game_library::saves(&library.games[1])
        .unwrap()
        .is_empty());
    // A fresh app and a moved game directory retain IDs; saved media are self-contained.
    let moved = root.join("relocated");
    std::fs::rename(&library.games[0].directory, &moved).unwrap();
    std::fs::remove_file(moved.join("disk.adf")).unwrap();
    std::fs::remove_file(root.join("kick.rom")).unwrap();
    let library = GameLibrary::discover(&root).unwrap();
    let game = library
        .games
        .iter()
        .find(|g| g.manifest.id == "one")
        .unwrap()
        .clone();
    let saves = gui::game_library::saves(&game).unwrap();
    let mut reopened = EmulatorApp::default();
    reopened.player.catalog = app.player.catalog.clone();
    reopened.player.catalog.configurations[0].name = "Renamed configuration".into();
    reopened.player.catalog.configurations[0].launch.preset = config::A500Preset::ExpandedPowerUser;
    reopened.player.catalog.configurations[0]
        .controls
        .ports
        .swap(0, 1);
    reopened.start_library_game(game, Some(saves[0].path.clone()), false);
    common::settle(&mut reopened, &ctx);
    assert!(reopened.host.status.running);
    common::command(&mut reopened, HostCommand::Pause);
    common::wait(&mut reopened, &ctx, |app| !app.host.status.running);
    let restored = common::snapshot(&mut reopened);
    assert_eq!(
        restored.config.active_preset(),
        config::A500Preset::Standard1Mb
    );
    assert!(restored.floppy.drives[0].disk_inserted);
    assert_eq!(
        reopened.host.status.ports,
        [
            debugger::launch::DeviceKind::Joystick,
            debugger::launch::DeviceKind::Mouse
        ]
    );
    assert_eq!(reopened.player.mounted[0].as_deref(), Some("disk-one"));
    // A save from the other game fails before replacing the healthy paused machine.
    let before = common::snapshot(&mut reopened);
    let other = library
        .games
        .iter()
        .find(|g| g.manifest.id == "two")
        .unwrap()
        .clone();
    reopened.start_library_game(other, Some(saves[0].path.clone()), false);
    common::settle(&mut reopened, &ctx);
    assert!(reopened.file_error.is_some());
    assert_eq!(common::snapshot(&mut reopened), before);
}
#[test]
fn invalid_launch_and_mount_keep_previous_machine_and_confirmed_labels() {
    let mut app = EmulatorApp::default();
    let ctx = egui::Context::default();
    common::settle(&mut app, &ctx);
    let before = common::snapshot(&mut app);
    let setup = debugger::launch::MachineSetup {
        config: config::A500Config::default(),
        ports: [
            debugger::launch::DeviceKind::Mouse,
            debugger::launch::DeviceKind::Joystick,
        ],
        drives: [true; 4],
        rom: vec![0; 12],
        disks: std::array::from_fn(|_| None),
    };
    assert!(common::command(
        &mut app,
        HostCommand::Launch {
            setup,
            association: debugger::SaveAssociation::default()
        }
    )
    .value
    .is_err());
    assert_eq!(common::snapshot(&mut app), before);
    assert!(common::command(
        &mut app,
        HostCommand::MountDisk {
            drive: 0,
            bytes: Some(vec![0; 10]),
            disk_id: Some("bad".into())
        }
    )
    .value
    .is_err());
    assert_eq!(common::snapshot(&mut app), before);
    assert_eq!(app.player.mounted, [None, None, None, None]);
}
#[test]
fn manual_setup_navigation_keeps_the_running_sessions_disk_assignments() {
    let mut app = EmulatorApp::default();
    app.player.mounted[0] = Some("active".into());
    app.navigate_player(PlayerScreen::Setup);
    app.player.setup_mounted[0] = Some("draft".into());
    assert_eq!(app.player.mounted[0].as_deref(), Some("active"));
    let ctx = egui::Context::default();
    common::settle(&mut app, &ctx);
    assert!(!app.host.status.running);
    assert!(!app.player.captured);
}

#[test]
fn library_continue_restores_selected_games_save_instead_of_active_game() {
    use egui_kittest::{kittest::Queryable, Harness};
    let (root, library) = library();
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.player.catalog.library_root = root.to_string_lossy().into_owned();
    app.player.catalog.configurations[0].launch.rom =
        root.join("kick.rom").to_string_lossy().into_owned();
    app.start_library_game(library.games[0].clone(), None, false);
    common::command(&mut app, HostCommand::Pause);
    common::command(
        &mut app,
        HostCommand::SetRegister {
            register: gui::EditRegister::D(7),
            value: 0x12345678,
        },
    );
    let saved = common::snapshot_json(&mut app);
    gui::game_library::persist_save(&library.games[0], &saved, None).unwrap();
    app.start_library_game(library.games[1].clone(), None, false);
    common::command(&mut app, HostCommand::Pause);
    let before = common::snapshot(&mut app);
    app.navigate_player(PlayerScreen::Library);
    app.player.library = GameLibrary::discover(&root).unwrap();
    app.player.selected_game = None;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    common::settle_harness(&mut h);
    h.get_by_label("one").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.saves.len(), 1);
    h.get_by_label_contains("Continue - ").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Keep current session").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(common::snapshot(h.state_mut()), before);
    h.get_by_label_contains("Continue - ").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Replace session").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(
        h.state().player.active_game.as_ref().unwrap().manifest.id,
        "one"
    );
    assert_eq!(h.state().player.screen, PlayerScreen::Session);
    common::command(h.state_mut(), HostCommand::Pause);
    let restored = debugger::SaveState::from_json(&common::snapshot_json(h.state_mut())).unwrap();
    assert_eq!(restored.state.cpu.d_long(7), 0x12345678);
    assert_eq!(restored.header.association.game_id.as_deref(), Some("one"));
    assert_eq!(
        restored.header.association.mounted[0].as_deref(),
        Some("disk-one")
    );
}

#[test]
fn library_continue_uses_the_selected_configuration_when_two_roots_reuse_a_game_id() {
    use debugger::launch::DeviceKind;
    use egui_kittest::kittest::Queryable;
    use gui::library_edit::GameDraft;

    let (first_root, first_library) = library();
    let (second_root, second_library) = library();
    let first = &first_library.games[0];
    let mut draft = GameDraft::edit(second_library.games[0].clone());
    draft.configuration = "swapped".into();
    let second = draft.save(&second_root).unwrap();
    assert_eq!(first.manifest.id, second.manifest.id);

    let mut app = game_app(&first_root);
    let mut configuration = app.player.catalog.configurations[0].clone();
    configuration.id = "swapped".into();
    configuration.name = "Swapped ports".into();
    configuration.controls.ports.swap(0, 1);
    app.player.catalog.configurations.push(configuration);
    app.start_library_game(second.clone(), None, false);
    common::command(&mut app, HostCommand::Pause);
    let json = common::snapshot_json(&mut app);
    gui::game_library::persist_save(&second, &json, None).unwrap();

    app.start_library_game(first.clone(), None, false);
    let mut h = session_harness(app);
    common::settle_harness(&mut h);
    assert_eq!(
        h.state().host.status.ports,
        [DeviceKind::Mouse, DeviceKind::Joystick]
    );
    h.get_by_label("Library").click_accesskit();
    common::settle_harness(&mut h);
    // Supply the path the native folder picker would return.
    h.state_mut().player.catalog.library_root = second_root.to_string_lossy().into_owned();
    h.get_by_label("Refresh").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("one").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label_contains("Continue - ").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Replace session").click_accesskit();
    common::settle_harness(&mut h);

    let app = h.state();
    assert!(app.file_error.is_none());
    assert_eq!(app.player.screen, PlayerScreen::Session);
    assert_eq!(
        app.player.active_configuration.as_ref().unwrap().id,
        "swapped"
    );
    assert_eq!(
        app.host.status.ports,
        [DeviceKind::Joystick, DeviceKind::Mouse]
    );
}

#[test]
fn player_disk_selection_and_ejection_publish_only_acknowledged_mounts() {
    use egui_kittest::{kittest::Queryable, Harness};
    fn open_df0(h: &mut Harness<'_, EmulatorApp>) {
        let row = h
            .get_all_by_value("DF0:")
            .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Label)
            .unwrap()
            .accesskit_node()
            .bounding_box()
            .unwrap();
        h.get_all_by_role(egui::accesskit::Role::ComboBox)
            .find(|n| {
                n.accesskit_node()
                    .bounding_box()
                    .is_some_and(|b| b.x0 >= row.x1 && b.y0 < row.y1 && b.y1 > row.y0)
            })
            .unwrap()
            .click();
        h.step();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while h.query_by_label("Eject / Empty").is_none() {
            h.step();
            assert!(
                std::time::Instant::now() < deadline,
                "DF0 menu did not open"
            );
        }
    }
    let (root, mut library) = library();
    let game = &mut library.games[0];
    std::fs::write(game.directory.join("second.adf"), vec![0x33; 901120]).unwrap();
    game.manifest.disks.push(GameDisk {
        id: "disk-two".into(),
        label: "Disk 2".into(),
        path: "second.adf".into(),
    });
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.player.catalog.configurations[0].launch.rom =
        root.join("kick.rom").to_string_lossy().into_owned();
    app.start_library_game(game.clone(), None, false);
    common::command(&mut app, HostCommand::Pause);
    gui::game_library::persist_save(game, &common::snapshot_json(&mut app), Some(1)).unwrap();
    app.navigate_player(PlayerScreen::Session);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    common::settle_harness(&mut h);
    open_df0(&mut h);
    h.get_by_label("2 - Disk 2").click();
    h.step();
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-one"));
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-two"));
    assert!(h.state().host.status.drives[0].inserted);
    let state = common::snapshot(h.state_mut());
    let mut expected = debugger::DebuggerSession::new();
    assert!(expected.machine.floppy.drives[0].insert_disk(&vec![0x33; 901120]));
    assert_eq!(
        serde_json::to_value(&state.floppy.drives[0]).unwrap()["track_data"],
        serde_json::to_value(&expected.machine.floppy.drives[0]).unwrap()["track_data"]
    );
    common::settle_harness(&mut h);
    open_df0(&mut h);
    h.get_by_label("Eject / Empty").click();
    h.step();
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-two"));
    common::settle_harness(&mut h);
    assert!(h.state().player.mounted[0].is_none());
    assert!(!h.state().host.status.drives[0].inserted);
    assert!(!common::snapshot(h.state_mut()).floppy.drives[0].disk_inserted);

    let source = game.directory.join("second.adf");
    assert_eq!(std::fs::read(&source).unwrap(), vec![0x33; 901120]);
    std::fs::remove_file(&source).unwrap();
    h.state_mut().set_view_mode(ViewMode::Developer);
    common::settle_harness(&mut h);
    h.get_by_label("State").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label_contains("Quick Load Slot 1 (F9)")
        .click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Replace session").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-one"));
    assert!(h.state().player.disks[1].bytes.is_none());
    open_df0(&mut h);
    h.get_by_label("2 - Disk 2").click();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-two"));
    assert_eq!(
        serde_json::to_value(&common::snapshot(h.state_mut()).floppy.drives[0]).unwrap()
            ["track_data"],
        serde_json::to_value(&expected.machine.floppy.drives[0]).unwrap()["track_data"]
    );

    open_df0(&mut h);
    h.get_by_label("Eject / Empty").click();
    common::settle_harness(&mut h);
    std::fs::write(source, vec![0x44; 901120]).unwrap();
    h.state_mut().start_library_game(game.clone(), None, false);
    common::settle_harness(&mut h);
    open_df0(&mut h);
    h.get_by_label("2 - Disk 2").click();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-two"));
    assert!(expected.machine.floppy.drives[0].insert_disk(&vec![0x44; 901120]));
    assert_eq!(
        serde_json::to_value(&common::snapshot(h.state_mut()).floppy.drives[0]).unwrap()
            ["track_data"],
        serde_json::to_value(&expected.machine.floppy.drives[0]).unwrap()["track_data"]
    );
}

#[test]
fn manual_start_requires_valid_rom_media_attachment_and_available_sources() {
    use egui_kittest::{kittest::Queryable, Harness};
    use gui::player::{PlayerScreen, SessionDisk};
    let root = root();
    let rom_path = root.join("kick.rom");
    let mut app = EmulatorApp::default();
    app.navigate_player(PlayerScreen::Setup);
    app.player.catalog.configurations[0].launch.rom = rom_path.to_string_lossy().into_owned();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app);
    common::settle_harness(&mut h);
    assert!(h.get_by_label("Start").accesskit_node().is_disabled());
    std::fs::write(&rom_path, b"invalid ROM").unwrap();
    h.run_steps(2);
    assert!(h.get_by_label("Start").accesskit_node().is_disabled());
    std::fs::write(&rom_path, rom()).unwrap();
    h.run_steps(2);
    assert!(!h.get_by_label("Start").accesskit_node().is_disabled());
    h.state_mut().player.setup_disks = vec![SessionDisk {
        id: "draft".into(),
        label: "Draft".into(),
        bytes: Some(vec![0; 12]),
    }];
    h.state_mut().player.setup_mounted[0] = Some("draft".into());
    h.run_steps(2);
    assert!(h.get_by_label("Start").accesskit_node().is_disabled());
    h.state_mut().player.setup_disks[0].bytes = Some(vec![0; 901120]);
    h.run_steps(2);
    assert!(!h.get_by_label("Start").accesskit_node().is_disabled());
    h.state_mut().player.setup_mounted = [None, Some("draft".into()), None, None];
    h.run_steps(2);
    assert!(h.get_by_label("Start").accesskit_node().is_disabled());
    h.state_mut().player.catalog.configurations[0].launch.drives[1] = true;
    h.run_steps(2);
    assert!(!h.get_by_label("Start").accesskit_node().is_disabled());
    h.state_mut().player.setup_mounted[1] = Some("missing".into());
    h.run_steps(2);
    assert!(h.get_by_label("Start").accesskit_node().is_disabled());
    h.state_mut().player.setup_mounted = [None, None, None, None];
    h.state_mut().player.catalog.configurations[0]
        .controls
        .ports[1]
        .source = gui::player_config::InputSource::Gamepad {
        identity: "missing".into(),
        mapping: "arrows".into(),
    };
    h.run_steps(2);
    assert!(h.get_by_label("Start").accesskit_node().is_disabled());
    assert_eq!(h.state().host.status.cck, 0);
}

fn game_app(root: &Path) -> EmulatorApp {
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.player.catalog.configurations[0].launch.rom =
        root.join("kick.rom").to_string_lossy().into_owned();
    app.player.catalog.library_root = root.to_string_lossy().into_owned();
    app.player.library = GameLibrary::discover(root).unwrap();
    app
}

fn session_harness(app: EmulatorApp) -> egui_kittest::Harness<'static, EmulatorApp> {
    egui_kittest::Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app)
}

fn screenshot_directory(game: &LibraryGame) -> PathBuf {
    game.directory.join("Screenshots")
}

fn directory_files(directory: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    paths.sort();
    paths
}

fn saved_progress_and_artwork(game: &LibraryGame) -> (Vec<(PathBuf, Vec<u8>)>, Vec<u8>) {
    let saves = gui::game_library::saves(game)
        .unwrap()
        .into_iter()
        .map(|save| {
            let bytes = std::fs::read(&save.path).unwrap();
            (save.path, bytes)
        })
        .collect();
    let artwork =
        std::fs::read(game.directory.join(game.manifest.artwork.as_ref().unwrap())).unwrap();
    (saves, artwork)
}

#[test]
fn screenshot_clicks_keep_the_active_game_destination_and_existing_files() {
    use egui_kittest::kittest::Queryable;

    let (root, library) = library();
    let [one, two] = library.games.as_slice() else {
        panic!("Expected two fixture games");
    };
    for (game, color) in [(one, [12, 34, 56]), (two, [78, 90, 12])] {
        image::save_buffer(
            game.directory.join(game.manifest.artwork.as_ref().unwrap()),
            &color,
            1,
            1,
            image::ColorType::Rgb8,
        )
        .unwrap();
    }
    let mut app = game_app(&root);
    app.start_library_game(one.clone(), None, false);
    let mut h = session_harness(app);
    common::wait_harness(&mut h, |app| {
        app.player.active_game.is_some() && app.host.presented.is_some()
    });
    h.get_by_label("Pause").click_accesskit();
    common::wait_harness(&mut h, |app| !app.host.status.running);
    let save = common::snapshot_json(h.state_mut());
    gui::game_library::persist_save(one, &save, Some(1)).unwrap();
    let mut other_save = debugger::SaveState::from_json(&save).unwrap();
    other_save.header.association.game_id = Some(two.manifest.id.clone());
    gui::game_library::persist_save(two, &other_save.to_json().unwrap(), None).unwrap();
    let before = [one, two].map(saved_progress_and_artwork);

    h.get_by_label("Library").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("two").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.selected_game, Some(1));
    h.get_by_label("Return to session").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(
        h.state().player.active_game.as_ref().unwrap().manifest.id,
        "one"
    );

    let directory = screenshot_directory(one);
    assert!(!directory.exists());
    let screenshot = h.get_by_label("Screenshot");
    let bounds = screenshot.rect();
    assert!(bounds.is_positive());
    assert!(h.ctx.screen_rect().contains_rect(bounds));
    for label in ["Save", "Load"] {
        let control = h.get_by_label(label).rect();
        assert!(bounds.top() < control.bottom() && bounds.bottom() > control.top());
        assert!(bounds.left() >= control.right());
    }
    screenshot.click();
    h.step();
    assert_eq!(
        directory_files(&directory),
        [directory.join("screenshot-1.png")]
    );
    assert!(h.state().toast_message.is_none());
    h.get_by_label("Screenshot").click_accesskit();
    h.step();
    let created = directory_files(&directory);
    assert_eq!(created.len(), 2);
    for path in &created {
        assert_eq!(path.extension().unwrap(), "png");
        image::open(path).unwrap();
    }
    let previous: Vec<_> = created
        .iter()
        .map(|path| std::fs::read(path).unwrap())
        .collect();
    let occupied = directory.join("screenshot-3.png");
    std::fs::write(&occupied, b"existing file").unwrap();
    h.get_by_label("Screenshot").click_accesskit();
    h.step();
    assert_eq!(directory_files(&directory).len(), 4);
    assert_eq!(std::fs::read(occupied).unwrap(), b"existing file");
    for (path, bytes) in created.iter().zip(previous) {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    image::open(directory.join("screenshot-4.png")).unwrap();
    assert!(!screenshot_directory(two).exists());
    assert_eq!([one, two].map(saved_progress_and_artwork), before);
    assert!(h.state().file_error.is_none());
}

fn screenshot_pixel(x: usize, y: usize, width: usize, height: usize) -> [u8; 4] {
    match (x, y) {
        (0, 0) => [255, 0, 0, 255],
        (_, 0) if x == width - 1 => [0, 255, 0, 255],
        (0, _) if y == height - 1 => [0, 0, 255, 255],
        _ if x == width - 1 && y == height - 1 => [255, 255, 0, 255],
        _ => [(x % 223) as u8, (y % 227) as u8, ((x + y) % 229) as u8, 255],
    }
}

#[test]
fn screenshot_uses_every_displayed_pixel_at_a_clipped_scale_during_run_and_reset() {
    use config::VideoStandard;
    use egui_kittest::kittest::Queryable;
    use std::time::{Duration, Instant};

    for (standard, height, command) in [
        (VideoStandard::Pal, 568, None),
        (VideoStandard::Ntsc, 480, Some(HostCommand::Run)),
        (VideoStandard::Pal, 568, Some(HostCommand::Reset)),
    ] {
        let (_, library) = library();
        let game = &library.games[0];
        let mut session = Box::new(gui::DebuggerSession::from_config(
            config::A500Config::standard_1mb(standard),
        ));
        session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
        let frame = &mut session.machine.denise.frame_builder;
        let width = 720;
        let stride = frame.row_stride();
        let mut expected = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let rgba = screenshot_pixel(x, y, width, height);
                let [r, g, b, a] = rgba;
                frame.frame_buffer_mut()[y * stride + x] = u32::from_be_bytes([a, r, g, b]);
                expected.extend_from_slice(&rgba);
            }
        }
        let mut app = EmulatorApp::with_session(session);
        app.set_view_mode(ViewMode::ScreenOnly);
        // Attach the fixture game while the real host presents the prepared machine image.
        app.player.active_game = Some(game.clone());
        app.screen_scale = gui::ScreenScale::Four;
        let mut h = session_harness(app);
        h.ctx.set_zoom_factor(1.25);
        let mut displayed = common::ScreenCapture::new([width, height]);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            displayed.apply(h.output());
            if h.state().host.is_idle() && displayed.id.is_some() && h.ctx.zoom_factor() == 1.25 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Prepared texture did not reach Game view"
            );
            h.step();
            std::thread::sleep(Duration::from_millis(1));
        }
        let texture = displayed.id.unwrap();
        assert!(displayed
            .pixels
            .iter()
            .zip(expected.chunks_exact(4))
            .all(|(pixel, rgba)| pixel.to_srgba_unmultiplied().as_slice() == rgba));
        assert!(h.output().shapes.iter().any(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) if mesh.texture_id == texture => {
                let bounds = mesh.calc_bounds();
                bounds.width() > shape.clip_rect.width()
                    && bounds.height() > shape.clip_rect.height()
            }
            _ => false,
        }));

        let previous = h.state().host.presented.unwrap();
        let pending = if let Some(mut command) = command {
            // Let the worker change the machine without advancing the GUI.
            let runtime = h.state_mut().host.runtime.as_mut().unwrap();
            let running = matches!(&command, HostCommand::Run);
            let generation = if running {
                previous.generation
            } else {
                previous.generation.wrapping_add(1)
            };
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                match runtime.submit(command) {
                    Ok(_) => break,
                    Err(busy) => command = busy.0,
                }
                runtime.poll().unwrap();
                assert!(Instant::now() < deadline, "Could not submit the command");
                std::thread::sleep(Duration::from_millis(1));
            }
            let pending = loop {
                runtime.poll().unwrap();
                let status = runtime.status();
                if status.running == running
                    && status.generation == generation
                    && status.publication > previous.publication
                {
                    break status.publication;
                }
                assert!(
                    Instant::now() < deadline,
                    "The new image did not reach the exchange"
                );
                std::thread::sleep(Duration::from_millis(1));
            };
            assert_eq!(h.state().host.presented, Some(previous));
            Some((pending, generation, running))
        } else {
            assert!(!h.state().host.status.running);
            None
        };

        // Contention may postpone the upload. Keep clicking the old texture until
        // one click and the pending upload complete in the same update.
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut captures = 0;
        loop {
            h.get_by_label("Screenshot").click_accesskit();
            h.step();
            captures += 1;
            let files = directory_files(&screenshot_directory(game));
            assert_eq!(files.len(), captures);
            let path = screenshot_directory(game).join(format!("screenshot-{captures}.png"));
            let decoded = image::open(path).unwrap().into_rgba8();
            assert_eq!(decoded.dimensions(), (width as u32, height as u32));
            assert_eq!(decoded.as_raw(), &expected);
            assert!(h.state().file_error.is_none());
            let Some((pending, generation, running)) = pending else {
                break;
            };
            if let Some(presented) = h.state().host.presented.filter(|info| *info != previous) {
                assert_eq!(h.state().host.status.running, running);
                assert_eq!(presented.generation, generation);
                assert_eq!(presented.publication, pending);
                displayed.apply(h.output());
                assert!(displayed
                    .pixels
                    .iter()
                    .zip(expected.chunks_exact(4))
                    .any(|(pixel, rgba)| pixel.to_srgba_unmultiplied().as_slice() != rgba));
                break;
            }
            assert!(
                Instant::now() < deadline,
                "The click did not coincide with the pending upload"
            );
        }
    }
}

#[test]
fn screenshot_rejects_the_previous_game_image_when_the_new_library_reuses_its_id() {
    use egui_kittest::kittest::Queryable;
    use std::time::{Duration, Instant};

    let (root, first_library) = library();
    let (_, second_library) = library();
    let first = &first_library.games[0];
    let second = &second_library.games[0];
    assert_eq!(first.manifest.id, second.manifest.id);
    let mut app = game_app(&root);
    app.start_library_game(first.clone(), None, false);
    let mut h = session_harness(app);
    common::wait_harness(&mut h, |app| {
        app.player.active_game.is_some() && app.host.presented.is_some()
    });

    h.state_mut()
        .start_library_game(second.clone(), None, false);
    let deadline = Instant::now() + Duration::from_secs(10);
    // Supply a click on the update that accepts the new game, before the GUI
    // has drawn any of its images. Earlier clicks still belong to the first game.
    loop {
        h.get_by_label("Screenshot").click_accesskit();
        h.step();
        let active = h.state().player.active_game.as_ref().unwrap();
        if active.directory == second.directory {
            break;
        }
        assert!(Instant::now() < deadline, "The new game did not start");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        h.state().file_error.as_deref(),
        Some("Could not save screenshot: No game image is available")
    );
    let directory = screenshot_directory(second);
    assert!(!directory.exists());

    h.get_by_label("Close").click_accesskit();
    common::wait_harness(&mut h, |app| {
        app.file_error.is_none() && app.host.presented.is_some()
    });
    h.get_by_label("Screenshot").click_accesskit();
    h.step();
    let files = directory_files(&directory);
    assert_eq!(files, [directory.join("screenshot-1.png")]);
    image::open(&files[0]).unwrap();
}

#[test]
fn screenshot_stays_with_the_active_game_after_editing_the_same_id_in_another_library() {
    use egui_kittest::kittest::Queryable;

    let (first_root, first_library) = library();
    let (second_root, second_library) = library();
    let first = &first_library.games[0];
    let second = &second_library.games[0];
    assert_eq!(first.manifest.id, second.manifest.id);
    let mut app = game_app(&first_root);
    app.start_library_game(first.clone(), None, false);
    let mut h = session_harness(app);
    common::wait_harness(&mut h, |app| {
        app.player.active_game.is_some() && app.host.presented.is_some()
    });

    h.get_by_label("Library").click_accesskit();
    common::settle_harness(&mut h);
    // Supply the path the native folder picker would return.
    h.state_mut().player.catalog.library_root = second_root.to_string_lossy().into_owned();
    h.get_by_label("Refresh").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("one").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Edit game").click_accesskit();
    common::settle_harness(&mut h);
    h.state_mut().player.library_editor.as_mut().unwrap().name = "Renamed".into();
    h.step();
    h.get_by_label("Save game").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.state().file_error.is_none());
    assert!(h.state().player.library_editor.is_none());
    let renamed = second_root.join("Renamed");
    assert!(renamed.join("game.json").is_file());
    assert!(!second.directory.exists());

    h.get_by_label("Return to session").click_accesskit();
    common::settle_harness(&mut h);
    let directory = screenshot_directory(first);
    assert!(!directory.exists());
    h.get_by_label("Screenshot").click_accesskit();
    h.step();
    let files = directory_files(&directory);
    assert_eq!(files, [directory.join("screenshot-1.png")]);
    image::open(&files[0]).unwrap();
    assert!(!renamed.join("Screenshots").exists());
    assert_eq!(
        h.state().player.active_game.as_ref().unwrap().directory,
        first.directory
    );
}

#[test]
fn screenshot_button_is_available_in_library_game_view_only() {
    use egui_kittest::kittest::Queryable;

    let (root, library) = library();
    let mut manual = game_app(&root);
    manual.start_manual();
    let mut h = session_harness(manual);
    common::wait_harness(&mut h, |app| app.player.active_configuration.is_some());
    assert_eq!(h.state().player.screen, PlayerScreen::Session);
    assert!(h.query_by_label("Screenshot").is_none());

    let mut app = game_app(&root);
    app.start_library_game(library.games[0].clone(), None, false);
    let mut h = session_harness(app);
    common::wait_harness(&mut h, |app| app.player.active_game.is_some());
    assert!(!h.get_by_label("Screenshot").accesskit_node().is_disabled());
    h.get_by_label("Debug View (F12)").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(h.state().view_mode, ViewMode::Developer);
    assert!(h.query_by_label("Screenshot").is_none());
}

#[test]
fn screenshot_directory_failure_is_visible_and_the_game_can_resume() {
    use egui_kittest::kittest::Queryable;

    let (root, library) = library();
    let game = &library.games[0];
    let blocked = screenshot_directory(game);
    std::fs::write(&blocked, b"keep this file").unwrap();
    let mut app = game_app(&root);
    app.start_library_game(game.clone(), None, false);
    let mut h = session_harness(app);
    common::wait_harness(&mut h, |app| {
        app.player.active_game.is_some() && app.host.presented.is_some() && app.host.status.running
    });
    h.get_by_label("Pause").click_accesskit();
    common::wait_harness(&mut h, |app| !app.host.status.running);
    let before = h.state().host.status;
    h.get_by_label("Screenshot").click_accesskit();
    h.run_steps(2);
    let error = h.state().file_error.as_ref().unwrap();
    assert!(error.starts_with("Could not save screenshot:"));
    assert!(h.get_all_by_value(error.as_str()).next().is_some());
    assert!(h.state().toast_message.is_none());
    assert_eq!(std::fs::read(blocked).unwrap(), b"keep this file");
    assert_eq!(h.state().host.status.generation, before.generation);
    assert_eq!(h.state().host.status.cck, before.cck);
    h.get_by_label("Close").click_accesskit();
    h.step();
    assert!(h.state().file_error.is_none());
    h.get_by_label("Resume").click_accesskit();
    common::wait_harness(&mut h, |app| {
        app.host.status.running && app.host.status.cck > before.cck
    });
}

#[test]
fn screenshot_write_error_preserves_existing_images_and_hides_partial_output() {
    use std::io::{Error, Write};

    let (_, library) = library();
    let game = &library.games[0];
    let directory = screenshot_directory(game);
    std::fs::create_dir(&directory).unwrap();
    let prior = directory.join("screenshot-1.png");
    image::save_buffer(&prior, &[10, 20, 30], 1, 1, image::ColorType::Rgb8).unwrap();
    let before = std::fs::read(&prior).unwrap();
    let error = gui::game_library::persist_screenshot(game, |file| {
        file.write_all(b"\x89PNG\r\n\x1a\n")?;
        assert!(!directory.join("screenshot-2.png").exists());
        Err(Error::other("simulated full disk"))
    })
    .unwrap_err();
    assert_eq!(error.to_string(), "simulated full disk");
    assert_eq!(std::fs::read(&prior).unwrap(), before);
    assert_eq!(directory_files(&directory), [prior]);
}

#[cfg(windows)]
#[test]
fn screenshot_publishes_a_png_beyond_the_legacy_windows_path_limit() {
    use std::os::windows::ffi::OsStrExt;

    let (root, library) = library();
    let root = std::path::absolute(root).unwrap();
    assert!(!root.as_os_str().as_encoded_bytes().starts_with(br"\\?\"));
    // Keep the game readable through its ordinary path and put the PNG beyond MAX_PATH.
    let padding = 235_usize.saturating_sub(root.as_os_str().encode_wide().count() + 1);
    let game_directory = root.join("g".repeat(padding.max(1)));
    std::fs::rename(&library.games[0].directory, &game_directory).unwrap();
    let game = GameLibrary::discover(&root).unwrap().games.remove(0);
    assert_eq!(game.directory, game_directory);
    let directory = screenshot_directory(&game);
    let path = directory.join("screenshot-1.png");
    assert!(path.as_os_str().encode_wide().count() >= 260);

    let expected = image::RgbImage::from_pixel(1, 1, image::Rgb([10, 20, 30]));
    gui::game_library::persist_screenshot(&game, |file| {
        image::DynamicImage::ImageRgb8(expected.clone())
            .write_to(file, image::ImageFormat::Png)
            .map_err(std::io::Error::other)
    })
    .unwrap();
    let files = directory_files(&directory);
    assert_eq!(files, [path]);
    assert_eq!(image::open(&files[0]).unwrap().to_rgb8(), expected);
}
