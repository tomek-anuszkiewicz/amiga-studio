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
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join(".agent")
        .join("tmp")
        .join("player-workflow")
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
        std::fs::create_dir_all(directory.join("disks")).unwrap();
        std::fs::write(
            directory.join("disks/Disk 1.adf"),
            vec![if id == "one" { 0x11 } else { 0x22 }; 901120],
        )
        .unwrap();
        let manifest = GameManifest {
            id: id.into(),
            title: id.into(),
            configuration: "standard".into(),
            disks: vec![GameDisk {
                id: "disk-one".into(),
                filename: "Disk 1.adf".into(),
            }],
            mounted: [Some("disk-one".into()), None, None, None],
            initial_state: None,
            start_states: Vec::new(),
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
fn launch_save_reopen_relocate_and_restore_verified_media() {
    use egui_kittest::kittest::Queryable;

    let (root, library) = library();
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.player.library_root = Some(root.clone());
    app.player.catalog.configurations[0].launch.rom = "kick.rom".into();
    app.player.catalog.configurations[0].launch.drives[1] = true;
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
    // Relocate the library and media and keep the stable game/disk locators.
    let moved_root = root.join("relocated");
    std::fs::create_dir(&moved_root).unwrap();
    let moved = moved_root.join("one");
    std::fs::rename(&library.games[0].directory, &moved).unwrap();
    std::fs::rename(&library.games[1].directory, moved_root.join("two")).unwrap();
    std::fs::remove_file(moved.join("disks/Disk 1.adf")).unwrap();
    std::fs::remove_file(root.join("kick.rom")).unwrap();
    let library = GameLibrary::discover(&moved_root).unwrap();
    let mut game = library
        .games
        .iter()
        .find(|g| g.manifest.id == "one")
        .unwrap()
        .clone();
    game.manifest.disks[0].filename = "Renamed disk.adf".into();
    // This unmounted disk has no source during restore.
    game.manifest.disks.push(GameDisk {
        id: "optional".into(),
        filename: "Unneeded.adf".into(),
    });
    game.manifest.mounted = [None, Some("disk-one".into()), None, None];
    game.manifest.initial_state = Some("unavailable-initial-state.json".into());
    std::fs::write(
        moved.join("game.json"),
        serde_json::to_vec(&game.manifest).unwrap(),
    )
    .unwrap();
    let saves = gui::game_library::saves(&game).unwrap();
    assert_eq!(saves.len(), 2);
    assert!(saves
        .iter()
        .all(|save| save.header.screenshot_png == state.header.screenshot_png));
    assert!(saves
        .windows(2)
        .all(|pair| (pair[0].header.timestamp, &pair[0].path)
            >= (pair[1].header.timestamp, &pair[1].path)));
    let unavailable = [
        ("catalog-old.json", br#"{"state":{"physical_memory":[1,2]},"header":{"version":15}}"#.as_slice(),
            "found 15, supported 17"),
        ("catalog-v16.json", br#"{"state":{"floppy":{"drives":[{"track_data":{"modified_tracks":{}}}]}},"header":{"version":16}}"#.as_slice(),
            "found 16, supported 17"),
        ("catalog-corrupt.json", br#"{"header":{"version":17,"timestamp":0,"association":{"mounted":[null,null,null,null]},"screenshot_png":"%%%"},"state":{}}"#.as_slice(),
            "serialization failed"),
    ];
    let original_files: Vec<_> = saves
        .iter()
        .map(|save| std::fs::read(&save.path).unwrap())
        .collect();
    for (name, contents, _) in unavailable {
        std::fs::write(moved.join("saves").join(name), contents).unwrap();
    }
    {
        let mut app = game_app(&moved_root);
        app.navigate_player(PlayerScreen::Library);
        let mut catalog = session_harness(app);
        common::settle_harness(&mut catalog);
        catalog.get_by_label("one").click_accesskit();
        common::wait_harness(&mut catalog, |app| {
            app.player.selected_saves.len() == saves.len()
        });
        for (actual, expected) in catalog.state().player.selected_saves.iter().zip(&saves) {
            assert_eq!(actual.path, expected.path);
            assert_eq!(actual.header, expected.header);
        }
        for (name, contents, expected) in unavailable {
            let path = moved.join("saves").join(name);
            let path_text = path.display().to_string();
            catalog.get_by(|node| {
                node.role() == egui::accesskit::Role::Label
                    && !node.is_hidden()
                    && node.value().is_some_and(|text| {
                        text.lines()
                            .any(|line| line.contains(&path_text) && line.contains(expected))
                    })
            });
            assert_eq!(std::fs::read(&path).unwrap(), contents);
        }
    }
    for (save, bytes) in saves.iter().zip(original_files) {
        assert_eq!(std::fs::read(&save.path).unwrap(), bytes);
    }
    for (name, _, _) in unavailable {
        std::fs::remove_file(moved.join("saves").join(name)).unwrap();
    }
    let save_path = saves[0].path.clone();
    let save_bytes = std::fs::read(&save_path).unwrap();
    let mut reopened = EmulatorApp::default();
    reopened.player.library_root = Some(moved_root.clone());
    reopened.player.catalog = app.player.catalog.clone();
    reopened.player.catalog.configurations[0].name = "Renamed configuration".into();
    reopened.player.catalog.configurations[0].launch.rom = "Renamed Kickstart.rom".into();
    reopened.player.catalog.configurations[0].launch.preset = config::A500Preset::ExpandedPowerUser;
    reopened.player.catalog.configurations[0]
        .controls
        .ports
        .swap(0, 1);

    let unchanged = common::snapshot(&mut reopened);
    let status = reopened.host.status;
    let renamed_rom = moved_root.join("Renamed Kickstart.rom");
    let renamed_disk = moved.join("disks/Renamed disk.adf");
    for (rom_bytes, disk_bytes, expected_error) in [
        (None, None, "Kickstart ROM"),
        (Some(vec![0x55; 262144]), None, "content does not match"),
        (Some(rom()), None, "DF0"),
        (Some(rom()), Some(vec![0x22; ADF_BYTES]), "ADF for DF0"),
    ] {
        if let Some(bytes) = rom_bytes {
            std::fs::write(&renamed_rom, bytes).unwrap();
        }
        if let Some(bytes) = disk_bytes {
            std::fs::write(&renamed_disk, bytes).unwrap();
        }
        reopened.start_library_game(game.clone(), Some(save_path.clone()), false);
        common::settle(&mut reopened, &ctx);
        let error = reopened.file_error.take().unwrap();
        assert!(error.contains(expected_error), "{error}");
        assert_eq!(reopened.host.status.generation, status.generation);
        assert_eq!(reopened.host.status.running, status.running);
        assert_eq!(reopened.host.status.drives, status.drives);
        assert_eq!(common::snapshot(&mut reopened), unchanged);
        assert_eq!(std::fs::read(&save_path).unwrap(), save_bytes);
    }
    let disk = vec![0x11; ADF_BYTES];
    std::fs::write(&renamed_disk, &disk).unwrap();
    reopened.start_library_game(game, Some(save_path.clone()), false);
    common::settle(&mut reopened, &ctx);
    assert!(reopened.host.status.running);
    assert!(reopened.file_error.is_none());
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
    assert_eq!(std::fs::read(&save_path).unwrap(), save_bytes);
    assert_eq!(std::fs::read(&renamed_disk).unwrap(), disk);
    // A save from the other game fails before replacing the healthy paused machine.
    let other = library
        .games
        .iter()
        .find(|g| g.manifest.id == "two")
        .unwrap()
        .clone();
    reopened.start_library_game(other, Some(saves[0].path.clone()), false);
    common::settle(&mut reopened, &ctx);
    assert!(reopened.file_error.is_some());
    assert_eq!(common::snapshot(&mut reopened), restored);

    reopened.file_error = None;
    let generation = reopened.host.status.generation;
    let mut h = session_harness(reopened);
    common::settle_harness(&mut h);
    std::fs::remove_file(&renamed_disk).unwrap();
    h.state_mut().player.disks[0].bytes = None;
    h.get_by_label("Reset").click_accesskit();
    common::wait_harness(&mut h, |app| app.host.status.generation > generation);
    assert!(h.state().host.status.running);
    assert_eq!(
        h.state().player.mounted,
        [None, Some("disk-one".into()), None, None]
    );
    let reset = common::snapshot(h.state_mut());
    assert!(!reset.floppy.drives[0].disk_inserted);
    assert_eq!(reset.config, restored.config);
    assert_eq!(
        reset.physical_memory.kickstart_rom(),
        restored.physical_memory.kickstart_rom()
    );
    assert_eq!(
        serde_json::to_value(&reset.floppy.drives[1]).unwrap()["track_data"],
        serde_json::to_value(&restored.floppy.drives[0]).unwrap()["track_data"]
    );
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
            rom_name: None,
            disk_names: [None, None, None, None],
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
            name: None,
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
    app.player.library_root = Some(root.clone());
    app.player.catalog.configurations[0].launch.rom = "kick.rom".into();
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
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    common::settle_harness(&mut h);
    h.get_by_label("one").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.selected_saves.len(), 1);
    h.get_by_label_contains("Load state - ").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Keep current session").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(common::snapshot(h.state_mut()), before);
    h.get_by_label_contains("Load state - ").click_accesskit();
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
    assert_eq!(restored.cpu().d_long(7), 0x12345678);
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
    let second = draft.save(&second_root, [true; 4]).unwrap().game;
    assert_eq!(first.manifest.id, second.manifest.id);

    let mut other_app = game_app(&second_root);
    let mut configuration = other_app.player.catalog.configurations[0].clone();
    configuration.id = "swapped".into();
    configuration.name = "Swapped ports".into();
    configuration.controls.ports.swap(0, 1);
    other_app.navigate_player(PlayerScreen::Configuration);
    common::settle(&mut other_app, &egui::Context::default());
    other_app
        .player
        .editor
        .as_mut()
        .unwrap()
        .catalog
        .configurations
        .push(configuration);
    other_app.save_configuration_edit();
    common::settle(&mut other_app, &egui::Context::default());
    assert!(other_app.player.editor.is_none());
    other_app.start_library_game(second.clone(), None, false);
    common::command(&mut other_app, HostCommand::Pause);
    let json = common::snapshot_json(&mut other_app);
    gui::game_library::persist_save(&second, &json, None).unwrap();

    let mut app = game_app(&first_root);
    app.start_library_game(first.clone(), None, false);
    let mut h = session_harness(app);
    common::settle_harness(&mut h);
    assert_eq!(
        h.state().host.status.ports,
        [DeviceKind::Mouse, DeviceKind::Joystick]
    );
    h.get_by_label("Library").click_accesskit();
    common::settle_harness(&mut h);
    common::select_library(&mut h, &second_root);
    common::wait_harness(&mut h, |app| app.player.library.games.len() == 2);
    h.get_by_label("one").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label_contains("Load state - ").click_accesskit();
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

fn player_drive_dropdown<'a>(
    h: &'a egui_kittest::Harness<'_, EmulatorApp>,
    drive_label: &str,
) -> egui_kittest::Node<'a> {
    use egui_kittest::kittest::Queryable;
    let row = h
        .get_all_by_value(drive_label)
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
}

fn assert_player_drive_text(
    h: &egui_kittest::Harness<'_, EmulatorApp>,
    drive_label: &str,
    expected: &str,
) {
    let dropdown = player_drive_dropdown(h, drive_label).rect();
    // egui paints the current choice without exposing it on the ComboBox node.
    assert!(
        h.output().shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Text(text)
                if text.galley.text() == expected
                    && dropdown.contains(text.pos)
                    && shape.clip_rect.contains(text.pos)
        )),
        "{drive_label} dropdown should display {expected:?}"
    );
}

fn open_player_drive(h: &mut egui_kittest::Harness<'_, EmulatorApp>, drive_label: &str) {
    use egui_kittest::kittest::Queryable;
    player_drive_dropdown(h, drive_label).click();
    h.step();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while h.query_by_label("Eject / Empty").is_none() {
        h.step();
        assert!(
            std::time::Instant::now() < deadline,
            "{drive_label} menu did not open"
        );
    }
}

const ADF_BYTES: usize = 901_120;
type DiskLayout<'a> = [Option<(&'a str, &'a str, u8)>; 4];

fn assert_player_disk_labels(h: &egui_kittest::Harness<'_, EmulatorApp>, layout: DiskLayout<'_>) {
    assert_eq!(
        h.state().player.mounted,
        layout.map(|disk| disk.map(|(id, _, _)| id.to_owned()))
    );
    for (drive, disk) in layout.into_iter().enumerate() {
        let status = h.state().host.status.drives[drive];
        assert!(status.connected, "DF{drive} should be attached");
        assert_eq!(status.inserted, disk.is_some(), "DF{drive} owner status");
        assert_player_drive_text(
            h,
            &format!("DF{drive}:"),
            disk.map_or("Empty", |(_, label, _)| label),
        );
    }
}

fn assert_player_disk_layout(
    h: &mut egui_kittest::Harness<'_, EmulatorApp>,
    layout: DiskLayout<'_>,
) -> common::ObservedSave {
    assert_player_disk_labels(h, layout);
    let snapshot = common::observe_save(h.state_mut());
    assert_eq!(
        snapshot.header.association.mounted,
        h.state().player.mounted
    );
    for (drive, expected) in snapshot.state.floppy.drives.iter().zip(layout) {
        assert!(drive.connected);
        assert_eq!(drive.disk_inserted, expected.is_some());
        if let Some((id, _, fill)) = expected {
            assert_eq!(
                drive.tracks_match_adf(&vec![fill; ADF_BYTES]),
                Some(true),
                "Full tracks for {id}"
            );
        }
    }
    snapshot
}

#[test]
fn unassigned_imports_stay_empty_on_boot_and_reset_and_remain_available() {
    use egui_kittest::kittest::Queryable;
    use gui::library_edit::GameDraft;

    let root = root();
    std::fs::write(root.join("kick.rom"), rom()).unwrap();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Unassigned imports".into();
    let imports = [("first.adf", 0x12), ("second.adf", 0x34)];
    for (name, fill) in imports {
        let source = root.join(name);
        std::fs::write(&source, vec![fill; ADF_BYTES]).unwrap();
        assert!(draft.add_disks(&[source]).is_empty());
    }
    assert_eq!(
        std::array::from_fn(|drive| draft.startup_disk(drive)),
        [const { None }; 4]
    );
    let game = draft.save(&root, [true; 4]).unwrap().game;
    assert_eq!(game.manifest.mounted, [const { None }; 4]);

    for (name, _) in imports {
        std::fs::remove_file(root.join(name)).unwrap();
    }
    let reopened = GameLibrary::discover(&root).unwrap().games.remove(0);
    let edit = GameDraft::edit(reopened.clone());
    assert_eq!(
        std::array::from_fn(|drive| edit.startup_disk(drive)),
        [const { None }; 4]
    );
    assert_eq!(reopened.manifest.disks.len(), imports.len());
    for (disk, (name, fill)) in reopened.manifest.disks.iter().zip(imports) {
        assert_eq!(disk.filename, name);
        let relative = Path::new("disks").join(name);
        assert_eq!(
            std::fs::read(reopened.directory.join(relative)).unwrap(),
            vec![fill; ADF_BYTES]
        );
    }

    let mut app = game_app(&root);
    app.player.catalog.configurations[0].launch.drives = [true; 4];
    app.start_library_game(reopened.clone(), None, true);
    let mut h = session_harness(app);
    common::wait_harness(&mut h, |app| app.host.status.running);
    assert_player_disk_layout(&mut h, [None; 4]);

    open_player_drive(&mut h, "DF2:");
    h.get_by_label("2 - second.adf").click();
    common::settle_harness(&mut h);
    assert_player_disk_layout(
        &mut h,
        [None, None, Some(("disk-2", "second.adf", 0x34)), None],
    );

    let generation = h.state().host.status.generation;
    h.get_by_label("Reset").click_accesskit();
    common::wait_harness(&mut h, |app| app.host.status.generation > generation);
    assert_player_disk_layout(&mut h, [None; 4]);
    assert_eq!(
        std::fs::read(
            reopened
                .directory
                .join("disks")
                .join(&reopened.manifest.disks[1].filename)
        )
        .unwrap(),
        vec![0x34; ADF_BYTES]
    );
}

#[test]
fn player_disk_selection_and_ejection_publish_only_acknowledged_mounts() {
    use egui_kittest::{kittest::Queryable, Harness};

    let (root, mut library) = library();
    let game = &mut library.games[0];
    let second_adf = vec![0x33; ADF_BYTES];
    std::fs::write(game.directory.join("disks/Disk 2.adf"), &second_adf).unwrap();
    game.manifest.disks.push(GameDisk {
        id: "disk-two".into(),
        filename: "Disk 2.adf".into(),
    });
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.player.library_root = Some(root.clone());
    app.player.catalog.configurations[0].launch.rom = "kick.rom".into();
    app.start_library_game(game.clone(), None, false);
    common::command(&mut app, HostCommand::Pause);
    gui::game_library::persist_save(game, &common::snapshot_json(&mut app), Some(1)).unwrap();
    app.navigate_player(PlayerScreen::Session);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
    common::settle_harness(&mut h);
    open_player_drive(&mut h, "DF0:");
    h.get_by_label("2 - Disk 2.adf").click();
    h.step();
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-one"));
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-two"));
    assert!(h.state().host.status.drives[0].inserted);
    assert_eq!(
        observe_drive(h.state_mut(), 0, Some(second_adf.clone())),
        (true, Some(true)),
        "DF0 did not receive the selected disk's complete tracks"
    );
    let mut different_adf = second_adf.clone();
    different_adf[ADF_BYTES - 1] ^= 1;
    assert_eq!(
        observe_drive(h.state_mut(), 0, Some(different_adf)),
        (true, Some(false)),
        "DF0 matched an ADF with a different byte on the final track"
    );
    common::settle_harness(&mut h);
    open_player_drive(&mut h, "DF0:");
    h.get_by_label("Eject / Empty").click();
    h.step();
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-two"));
    common::settle_harness(&mut h);
    assert!(h.state().player.mounted[0].is_none());
    assert!(!h.state().host.status.drives[0].inserted);
    assert_eq!(observe_drive(h.state_mut(), 0, None), (false, None));

    let source = game.directory.join("disks/Disk 2.adf");
    assert_eq!(std::fs::read(&source).unwrap(), second_adf);
    std::fs::remove_file(&source).unwrap();
    h.state_mut().set_view_mode(ViewMode::Developer);
    common::settle_harness(&mut h);
    h.get_by_label("State").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label_contains("Quick Load Slot 1")
        .click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Replace session").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-one"));
    assert!(h.state().player.disks[1].bytes.is_none());
    open_player_drive(&mut h, "DF0:");
    h.get_by_label("2 - Disk 2.adf").click();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-two"));
    assert_eq!(
        observe_drive(h.state_mut(), 0, Some(second_adf)),
        (true, Some(true)),
        "DF0 did not reinsert the detached disk's complete tracks"
    );

    open_player_drive(&mut h, "DF0:");
    h.get_by_label("Eject / Empty").click();
    common::settle_harness(&mut h);
    let replacement_adf = vec![0x44; ADF_BYTES];
    std::fs::write(source, &replacement_adf).unwrap();
    h.state_mut().start_library_game(game.clone(), None, false);
    common::settle_harness(&mut h);
    open_player_drive(&mut h, "DF0:");
    h.get_by_label("2 - Disk 2.adf").click();
    common::settle_harness(&mut h);
    assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-two"));
    assert_eq!(
        observe_drive(h.state_mut(), 0, Some(replacement_adf)),
        (true, Some(true)),
        "DF0 reused detached tracks instead of opening the new source"
    );
}

fn observe_drive(
    app: &mut EmulatorApp,
    drive: usize,
    expected_adf: Option<Vec<u8>>,
) -> (bool, Option<bool>) {
    use gui::host_commands::{HostObservation, HostValue};

    let reply = common::idle_command(
        app,
        HostCommand::Observe(HostObservation::Drive {
            drive,
            expected_adf,
        }),
    );
    let Ok(HostValue::Drive {
        inserted,
        tracks_match,
    }) = reply.value
    else {
        panic!("Missing DF{drive} observation: {:?}", reply.value);
    };
    (inserted, tracks_match)
}

#[test]
fn manual_start_requires_valid_rom_media_attachment_and_available_sources() {
    use egui_kittest::{kittest::Queryable, Harness};
    use gui::player::{PlayerScreen, SessionDisk};
    let root = root();
    let rom_path = root.join("kick.rom");
    let mut app = EmulatorApp::default();
    app.navigate_player(PlayerScreen::Setup);
    app.player.library_root = Some(root);
    app.player.catalog.configurations[0].launch.rom = "kick.rom".into();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(|ui, app: &mut EmulatorApp| app.update_ui(ui), app);
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
    app.player.catalog.configurations[0].launch.rom = "kick.rom".into();
    app.player.library_root = Some(root.to_path_buf());
    app.player.library = GameLibrary::discover(root).unwrap();
    app
}

fn select_library_with_current_profiles(
    h: &mut egui_kittest::Harness<'_, EmulatorApp>,
    root: &Path,
) {
    use egui_kittest::kittest::Queryable;

    let catalog = &h.state().player.catalog;
    std::fs::write(
        root.join("configurations.json"),
        serde_json::json!({
            "configurations": catalog.configurations,
            "mappings": catalog.mappings,
        })
        .to_string(),
    )
    .unwrap();
    h.get_by_label("Library").click_accesskit();
    common::settle_harness(h);
    common::select_library(h, root);
    h.get_by_label("Return to session").click_accesskit();
    common::settle_harness(h);
}

fn session_harness_builder() -> egui_kittest::HarnessBuilder<EmulatorApp> {
    egui_kittest::Harness::builder().with_size(egui::vec2(1280.0, 900.0))
}

fn session_harness(app: EmulatorApp) -> egui_kittest::Harness<'static, EmulatorApp> {
    session_harness_builder().build_ui_state(|ui, app| app.update_ui(ui), app)
}

fn caption_commands(output: &egui::FullOutput) -> impl Iterator<Item = &str> {
    output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .iter()
        .filter_map(|command| match command {
            egui::ViewportCommand::Title(title) => Some(title.as_str()),
            _ => None,
        })
}

#[track_caller]
fn settle_caption(h: &mut egui_kittest::Harness<'_, EmulatorApp>, expected: &[&str]) {
    let commands = collect_caption_commands(h, |_| true);
    assert_eq!(commands, expected, "GUI error: {:?}", h.state().file_error);
}

#[track_caller]
fn collect_caption_commands(
    h: &mut egui_kittest::Harness<'_, EmulatorApp>,
    ready: impl Fn(&egui_kittest::Harness<'_, EmulatorApp>) -> bool,
) -> Vec<String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut commands = Vec::new();
    let mut settled = false;
    loop {
        h.step();
        commands.extend(caption_commands(h.output()).map(str::to_owned));
        let app = h.state();
        let idle = app.host.is_idle() && !app.player.busy && ready(h);
        if idle && settled {
            break;
        }
        settled = idle;
        assert!(
            std::time::Instant::now() < deadline,
            "Caption did not settle. GUI error: {:?}",
            app.file_error
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    commands
}

#[test]
fn desktop_startup_requests_the_default_root_viewport_caption() {
    let context = egui::Context::default();
    let creation = eframe::CreationContext::_new_kittest(context.clone());
    let mut app = EmulatorApp::new(&creation);
    let output = common::settle(&mut app, &context);
    assert_eq!(
        caption_commands(&output).collect::<Vec<_>>(),
        ["Amiga 500 Developer Studio"]
    );
}

#[test]
fn caption_identifies_the_acknowledged_session_across_browse_rename_and_replacement() {
    use egui_kittest::kittest::Queryable;

    const FIRST: &str = "Arctic Trading Expedition";
    const RENAMED: &str = "Arctic Trading Expedition Deluxe";
    const SECOND: &str = "Workbench Tools Collection";
    const SECOND_EDIT: &str = "Workbench Tools Collection Updated";

    let (root, mut library) = library();
    for (game, title) in library.games.iter_mut().zip([FIRST, SECOND]) {
        game.manifest.title = title.into();
        std::fs::write(
            game.directory.join("game.json"),
            serde_json::to_vec(&game.manifest).unwrap(),
        )
        .unwrap();
    }
    let mut app = game_app(&root);
    app.player.library_root = None;
    let mut h = session_harness(app);
    common::settle_harness(&mut h);
    select_library_with_current_profiles(&mut h, &root);
    h.get_by_label("Library").click_accesskit();
    let commands = collect_caption_commands(&mut h, |h| {
        h.query_by_role_and_label(egui::accesskit::Role::Button, FIRST)
            .is_some()
    });
    assert!(
        commands.is_empty(),
        "Unexpected caption commands while opening the library: {commands:?}"
    );

    let choose_game = |h: &mut egui_kittest::Harness<'_, EmulatorApp>, name| {
        h.get_by_role_and_label(egui::accesskit::Role::Button, name)
            .click_accesskit();
        settle_caption(h, &[]);
    };
    let rename = |h: &mut egui_kittest::Harness<'_, EmulatorApp>, name, expected: &[&str]| {
        h.get_by_label("Edit game").click_accesskit();
        settle_caption(h, &[]);
        h.state_mut().player.library_editor.as_mut().unwrap().name = name;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            h.step();
            let save = h.get_by_label("Save game");
            if !save.accesskit_node().is_disabled()
                && h.ctx.content_rect().contains_rect(save.rect())
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Save game did not become usable"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        h.get_by_label("Save game").click_accesskit();
        let commands = collect_caption_commands(h, |h| {
            let app = h.state();
            app.player.library_editor.is_none() || app.file_error.is_some()
        });
        assert!(
            h.state().file_error.is_none(),
            "Rename failed: {:?}",
            h.state().file_error
        );
        assert!(
            h.state().player.library_editor.is_none(),
            "The rename did not commit"
        );
        assert_eq!(commands, expected);
    };

    choose_game(&mut h, FIRST);
    h.get_by_label("Start game").click_accesskit();
    settle_caption(&mut h, &[FIRST]);
    assert!(h.state().host.status.running);
    h.get_by_label("Pause").click_accesskit();
    settle_caption(&mut h, &[]);
    assert!(!h.state().host.status.running);
    let first_generation = h.state().host.status.generation;
    let first_snapshot = common::snapshot(h.state_mut());

    h.get_by_label("Library").click_accesskit();
    settle_caption(&mut h, &[]);
    choose_game(&mut h, SECOND);
    h.get_by_label("Return to session").click_accesskit();
    settle_caption(&mut h, &[]);
    assert_eq!(h.state().host.status.generation, first_generation);
    assert_eq!(common::snapshot(h.state_mut()), first_snapshot);

    h.get_by_label("Library").click_accesskit();
    settle_caption(&mut h, &[]);
    rename(&mut h, SECOND_EDIT.into(), &[]);
    choose_game(&mut h, FIRST);
    rename(&mut h, RENAMED.into(), &[RENAMED]);

    h.get_by_label("Return to session").click_accesskit();
    settle_caption(&mut h, &[]);
    assert_eq!(h.state().host.status.generation, first_generation);
    assert!(!h.state().host.status.running);
    assert_eq!(common::snapshot(h.state_mut()), first_snapshot);
    h.get_by_label("Save").click_accesskit();
    settle_caption(&mut h, &[]);
    assert_eq!(h.state().player.saves.len(), 1);
    let save_path = &h.state().player.saves[0].path;
    let saved =
        debugger::SaveState::from_json(&std::fs::read_to_string(save_path).unwrap()).unwrap();
    assert_eq!(saved.header.association.game_id.as_deref(), Some("one"));
    let sources = debugger::MediaSources {
        rom: Some(debugger::SourceImage {
            name: None,
            bytes: std::fs::read(root.join("kick.rom")).unwrap(),
        }),
        disks: [
            Some(debugger::SourceImage {
                name: None,
                bytes: std::fs::read(root.join(RENAMED).join("disks/Disk 1.adf")).unwrap(),
            }),
            None,
            None,
            None,
        ],
    };
    assert_eq!(*saved.restore(&sources).unwrap(), first_snapshot);

    h.get_by_label("Library").click_accesskit();
    settle_caption(&mut h, &[]);
    choose_game(&mut h, SECOND_EDIT);

    h.get_by_label("Start game").click_accesskit();
    settle_caption(&mut h, &[]);
    h.get_by_label("Keep current session").click_accesskit();
    settle_caption(&mut h, &[]);
    assert_eq!(h.state().host.status.generation, first_generation);

    let disk = root.join(SECOND_EDIT).join("disks/Disk 1.adf");
    std::fs::write(&disk, [0; 1]).unwrap();
    h.get_by_label("Start game").click_accesskit();
    settle_caption(&mut h, &[]);
    h.get_by_label("Replace session").click_accesskit();
    settle_caption(&mut h, &[]);
    assert!(h.state().file_error.is_some());
    assert_eq!(h.state().host.status.generation, first_generation);
    assert_eq!(common::snapshot(h.state_mut()), first_snapshot);
    h.get_by_label("Close").click_accesskit();
    settle_caption(&mut h, &[]);

    std::fs::write(&disk, vec![0x22; ADF_BYTES]).unwrap();
    h.get_by_label("Start game").click_accesskit();
    settle_caption(&mut h, &[]);
    // Observe the submitted command before another pass can consume the reply.
    let passes = h.ctx.options(|options| options.max_passes);
    h.ctx
        .options_mut(|options| options.max_passes = std::num::NonZeroUsize::MIN);
    h.get_by_label("Replace session").click_accesskit();
    h.step();
    h.ctx.options_mut(|options| options.max_passes = passes);
    assert!(h.state().player.busy);
    assert_eq!(
        h.state()
            .player
            .active_game
            .as_ref()
            .unwrap()
            .manifest
            .title,
        RENAMED
    );
    assert!(caption_commands(h.output()).next().is_none());
    settle_caption(&mut h, &[SECOND_EDIT]);
    assert!(h.state().host.status.generation > first_generation);

    h.get_by_label("Library").click_accesskit();
    settle_caption(&mut h, &[]);
    choose_game(&mut h, RENAMED);
    h.get_by_label_contains("Load state - ").click_accesskit();
    settle_caption(&mut h, &[]);
    h.get_by_label("Replace session").click_accesskit();
    settle_caption(&mut h, &[RENAMED]);
    assert_eq!(
        h.state().player.active_game.as_ref().unwrap().manifest.id,
        "one"
    );

    h.get_by_label("Manual setup").click_accesskit();
    settle_caption(&mut h, &[]);
    h.get_by_label("Start").click_accesskit();
    settle_caption(&mut h, &[]);
    h.get_by_label("Replace session").click_accesskit();
    settle_caption(&mut h, &["Amiga 500 Developer Studio"]);
    assert!(h.state().player.active_game.is_none());
    assert_eq!(h.state().player.screen, PlayerScreen::Session);
}

fn add_library_disk(game: &mut LibraryGame, id: &str, label: &str, fill: u8) -> PathBuf {
    let source = game.directory.join("disks").join(label);
    std::fs::write(&source, vec![fill; ADF_BYTES]).unwrap();
    game.manifest.disks.push(GameDisk {
        id: id.into(),
        filename: label.into(),
    });
    source
}

#[test]
fn side_panel_stretches_short_long_and_empty_disk_selectors_equally() {
    use egui::accesskit::Role;
    use egui_kittest::kittest::Queryable;

    let (root, mut library) = library();
    let game = &mut library.games[0];
    let long = "The very long name of the alternate first disk from the Amiga collection.adf";
    add_library_disk(game, "long-disk", long, 0x41);
    game.manifest.mounted = [
        Some("long-disk".into()),
        Some("disk-one".into()),
        None,
        None,
    ];
    let mut app = game_app(&root);
    app.player.catalog.configurations[0].launch.drives = [true, true, true, false];
    app.start_library_game(game.clone(), None, false);
    let mut h = session_harness(app);
    common::settle_harness(&mut h);
    h.get_by_label("Pause").click_accesskit();
    common::settle_harness(&mut h);
    let before = common::snapshot(h.state_mut());

    let selectors = |h: &egui_kittest::Harness<'_, EmulatorApp>| {
        let controls = ["DF0:", "DF1:", "DF2:"].map(|drive| player_drive_dropdown(h, drive).rect());
        let first = controls[0];
        for rect in controls {
            assert!((rect.left() - first.left()).abs() <= 1.0);
            assert!((rect.right() - first.right()).abs() <= 1.0);
            assert!(h.ctx.content_rect().contains_rect(rect));
        }
        assert!((h.get_by_label("Disconnected").rect().left() - first.left()).abs() <= 1.0);
        assert_player_drive_text(h, "DF2:", "Empty");
        first.width()
    };
    let before_width = selectors(&h);
    let edge = h.get_by_role(Role::Splitter).rect().center();
    let target = edge - egui::vec2(175.0, 0.0);
    h.hover_at(edge);
    h.step();
    h.drag_at(edge);
    h.step();
    h.hover_at(target);
    h.step();
    h.drop_at(target);
    common::settle_harness(&mut h);
    assert!(selectors(&h) >= before_width + 160.0);
    h.ctx
        .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
    player_drive_dropdown(&h, "DF0:").hover();
    common::settle_harness(&mut h);
    h.get_by_label(long);
    assert_eq!(common::snapshot(h.state_mut()), before);
}

#[test]
fn initial_state_and_normal_boot_use_their_own_disk_layouts() {
    use debugger::launch::{DeviceKind, MachineSetup};
    use egui_kittest::kittest::Queryable;

    const INITIAL_D7: u32 = 0x1234_5678;
    const LIVE_D7: u32 = 0x8765_4321;

    let (root, library) = library();
    let mut game = library.games[0].clone();
    add_library_disk(&mut game, "disk-two", "Disk 2.adf", 0x22);
    add_library_disk(&mut game, "disk-three", "Disk 3.adf", 0x33);
    let startup = [
        Some(("disk-one", "Disk 1.adf", 0x11)),
        None,
        Some(("disk-three", "Disk 3.adf", 0x33)),
        None,
    ];
    let initial = [
        None,
        Some(("disk-two", "Disk 2.adf", 0x22)),
        None,
        Some(("disk-three", "Disk 3.adf", 0x33)),
    ];
    game.manifest.mounted = startup.map(|disk| disk.map(|(id, _, _)| id.to_owned()));
    game.manifest.initial_state = Some("initial.json".into());
    std::fs::write(
        game.directory.join("game.json"),
        serde_json::to_vec(&game.manifest).unwrap(),
    )
    .unwrap();
    let mut captured_disks = initial.map(|disk| disk.map(|(_, _, fill)| vec![fill; ADF_BYTES]));
    // DF3 at capture has different tracks from the library's ADF.
    captured_disks[3].as_mut().unwrap().fill(0x43);
    let mut machine = MachineSetup {
        config: config::A500Config::default(),
        ports: [DeviceKind::Mouse, DeviceKind::Joystick],
        drives: [true; 4],
        rom: rom(),
        disks: captured_disks,
    }
    .prepare()
    .unwrap();
    machine.machine.cpu.state.set_d_long(7, INITIAL_D7);
    let mut sources = machine.file_sources().clone();
    let source = &mut sources.disks[3].as_mut().unwrap().bytes;
    *source = std::fs::read(game.directory.join("disks/Disk 3.adf")).unwrap();
    assert_eq!(
        machine.machine.floppy.drives[3].tracks_match_adf(source),
        Some(false)
    );
    let mut saved = debugger::SaveState::capture(&machine.machine, &sources).unwrap();
    saved.header.association.game_id = Some(game.manifest.id.clone());
    saved.header.association.mounted = initial.map(|disk| disk.map(|(id, _, _)| id.to_owned()));
    let initial_path = game.directory.join("initial.json");
    let initial_json = saved.to_json().unwrap();
    std::fs::write(&initial_path, &initial_json).unwrap();

    let mut app = game_app(&root);
    app.player.catalog.configurations[0].launch.drives = [true; 4];
    app.navigate_player(PlayerScreen::Library);
    let mut h = session_harness_builder().build_ui_state(
        |ui, app| {
            // The click frame can send a command, and its next poll is in the next frame.
            ui.ctx()
                .options_mut(|options| options.max_passes = std::num::NonZeroUsize::MIN);
            app.update_ui(ui);
        },
        app,
    );
    common::settle_harness(&mut h);
    h.get_by_label("one").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Start game").click_accesskit();
    common::wait_harness(&mut h, |app| app.host.status.running);
    h.get_by_label("Pause").click_accesskit();
    common::wait_harness(&mut h, |app| !app.host.status.running);
    assert_eq!(
        assert_player_disk_layout(&mut h, initial)
            .state
            .cpu
            .d_long(7),
        INITIAL_D7
    );

    let generation = h.state().host.status.generation;
    h.get_by_label("Reset").click_accesskit();
    h.step();
    assert!(
        h.state().player.busy,
        "Reset must be queued before its acknowledgement"
    );
    assert_eq!(h.state().host.status.generation, generation);
    assert!(!h.state().host.status.running);
    h.get_by_label("Resume");
    assert_player_disk_labels(&h, initial);

    common::wait_harness(&mut h, |app| app.host.status.generation > generation);
    assert!(h.state().host.status.running);
    assert!(!h.state().player.busy);
    h.get_by_label("Pause");
    assert_eq!(
        assert_player_disk_layout(&mut h, startup)
            .state
            .cpu
            .d_long(7),
        0
    );
    assert_eq!(
        std::fs::read(&initial_path).unwrap(),
        initial_json.as_bytes()
    );

    std::fs::write(initial_path, b"invalid initial state").unwrap();
    h.get_by_label("Library").click_accesskit();
    common::settle_harness(&mut h);
    common::command(
        h.state_mut(),
        HostCommand::SetRegister {
            register: gui::EditRegister::D(7),
            value: LIVE_D7,
        },
    )
    .value
    .unwrap();
    h.get_by_label("Start game").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Replace session").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.state().file_error.is_some());
    assert!(h.state().player.initial_error);
    let generation = h.state().host.status.generation;
    assert!(!h.state().host.status.running);
    assert_eq!(common::snapshot(h.state_mut()).cpu.d_long(7), LIVE_D7);

    h.get_by_label("Close").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Boot normally without initial state")
        .click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Replace session").click_accesskit();
    common::wait_harness(&mut h, |app| app.host.status.generation > generation);
    assert!(h.state().host.status.running);
    assert!(h.state().file_error.is_none());
    assert_eq!(
        assert_player_disk_layout(&mut h, startup)
            .state
            .cpu
            .d_long(7),
        0
    );
}

#[test]
fn player_reset_runs_after_running_paused_and_loaded_sessions() {
    use egui_kittest::kittest::Queryable;
    use gui::host_commands::{HostObservation, HostValue};

    // Slow RAM remains visible when Reset enables the Kickstart overlay.
    const SLOW_RAM_ADDRESS: u32 = 0xc0_0000;
    const RAM_SENTINEL: u16 = 0xa55a;

    let read_slow_ram = |app: &mut EmulatorApp| {
        let reply = common::idle_command(
            app,
            HostCommand::Observe(HostObservation::Execution {
                data_register: 0,
                word_address: Some(SLOW_RAM_ADDRESS),
            }),
        );
        let Ok(HostValue::Execution { memory_word, .. }) = reply.value else {
            panic!("Missing Slow RAM observation: {:?}", reply.value);
        };
        memory_word
    };

    let (root, library) = library();
    let game = &library.games[0];
    let mut observations = Vec::new();
    let manual_disks = [
        None,
        Some(("disk-two", "Disk 2.adf", 0x22)),
        None,
        Some(("disk-three", "Disk 3.adf", 0x33)),
    ];
    let assert_manual_media = |h: &mut egui_kittest::Harness<'_, EmulatorApp>| {
        assert_player_disk_labels(h, manual_disks);
        for (drive, disk) in manual_disks.into_iter().enumerate() {
            assert_eq!(
                observe_drive(
                    h.state_mut(),
                    drive,
                    disk.map(|(_, _, fill)| vec![fill; ADF_BYTES])
                ),
                (disk.is_some(), disk.map(|_| true)),
                "Manual Reset must preserve the full tracks or empty state of DF{drive}"
            );
        }
    };
    for library_game in [true, false] {
        let mut app = game_app(&root);
        app.player.catalog.configurations[0].launch.preset = config::A500Preset::Standard1Mb;
        if library_game {
            app.start_library_game(game.clone(), None, true);
        } else {
            app.player.catalog.configurations[0].launch.drives = [true; 4];
            for (drive, id, label, fill) in [
                (0, "disk-one", "Disk 1.adf", 0x11),
                (1, "disk-two", "Disk 2.adf", 0x22),
                (3, "disk-three", "Disk 3.adf", 0x33),
            ] {
                app.player.setup_disks.push(gui::player::SessionDisk {
                    id: id.into(),
                    label: label.into(),
                    bytes: Some(vec![fill; 901120]),
                });
                app.player.setup_mounted[drive] = Some(id.into());
            }
            app.start_manual();
        }
        let mut h = session_harness(app);
        common::wait_harness(&mut h, |app| app.host.status.running);
        let progress = common::snapshot_json(h.state_mut());
        for (paused, loaded) in [(false, false), (true, false), (false, true), (true, true)] {
            if loaded {
                let generation = h.state().host.status.generation;
                if library_game {
                    let save = gui::game_library::persist_save(game, &progress, Some(1)).unwrap();
                    h.state_mut()
                        .start_library_game(game.clone(), Some(save), false);
                } else {
                    let save = root.join("manual-progress.json");
                    std::fs::write(&save, &progress).unwrap();
                    h.state_mut().load_state_file(&save);
                }
                common::wait_harness(&mut h, |app| app.host.status.generation > generation);
                assert!(h.state().host.status.drives[0].inserted);
                assert_eq!(h.state().player.mounted[0].as_deref(), Some("disk-one"));
            }
            if !h.state().host.status.running {
                h.get_by_label("Resume").click_accesskit();
                common::wait_harness(&mut h, |app| app.host.status.running);
            }
            if h.state().player.mounted[0].is_some() {
                open_player_drive(&mut h, "DF0:");
                h.get_by_label("Eject / Empty").click();
                common::settle_harness(&mut h);
            }
            if paused {
                h.get_by_label("Pause").click_accesskit();
                common::wait_harness(&mut h, |app| !app.host.status.running);
            }
            h.get_by_label(if paused { "Resume" } else { "Pause" });
            let before = h.state().host.status;
            assert_eq!(before.running, !paused);
            assert!(!before.drives[0].inserted);
            let manual_disk = (!library_game).then(|| "disk-two".into());
            let third_manual_disk = (!library_game).then(|| "disk-three".into());
            assert_eq!(
                h.state().player.mounted,
                [None, manual_disk, None, third_manual_disk]
            );
            if !library_game {
                assert_manual_media(&mut h);
            }
            if paused {
                common::idle_command(
                    h.state_mut(),
                    HostCommand::WriteMemory {
                        address: SLOW_RAM_ADDRESS,
                        bytes: RAM_SENTINEL.to_be_bytes().to_vec(),
                        restart_instruction: false,
                    },
                )
                .value
                .unwrap();
                assert_eq!(
                    read_slow_ram(h.state_mut()),
                    Some(RAM_SENTINEL),
                    "Slow RAM must contain the test word before Player Reset"
                );
            }
            h.get_by_label("Reset").click_accesskit();
            common::wait_harness(&mut h, |app| app.host.status.generation > before.generation);
            let reset = h.state().host.status;
            if paused {
                assert_eq!(
                    read_slow_ram(h.state_mut()),
                    Some(0),
                    "Player Reset must clear Slow RAM (library game {library_game}, loaded {loaded})"
                );
            }
            if reset.running {
                common::wait_harness(&mut h, |app| {
                    app.host.status.generation == reset.generation
                        && app.host.status.cck > reset.cck
                });
            } else {
                std::thread::sleep(std::time::Duration::from_millis(100));
                common::settle_harness(&mut h);
            }
            let status = h.state().host.status;
            let associations = &h.state().player.mounted;
            let pause_visible = h.query_by_label("Pause").is_some();
            let inserted = status.drives.map(|drive| drive.inserted);
            observations.push((
                library_game,
                paused,
                loaded,
                before.cck,
                reset.running,
                reset.cck,
                status.cck,
                inserted,
                associations.clone(),
                pause_visible,
            ));
            if !library_game {
                assert_manual_media(&mut h);
            }
        }
    }
    eprintln!("Reset observations (game, paused, loaded, prior CCK, running after Reset, Reset CCK, later CCK, inserted, IDs, Pause visible): {observations:#?}");
    for (library_game, _, _, _, running, reset_cck, later_cck, inserted, ids, pause_visible) in
        observations
    {
        assert!(
            running && pause_visible && later_cck > reset_cck,
            "Reset must run and advance without Resume"
        );
        assert_eq!(
            inserted,
            [library_game, !library_game, false, !library_game]
        );
        assert_eq!(
            ids,
            [
                library_game.then(|| "disk-one".into()),
                (!library_game).then(|| "disk-two".into()),
                None,
                (!library_game).then(|| "disk-three".into()),
            ]
        );
    }
}

#[test]
fn unavailable_invalid_or_disconnected_startup_disk_rejects_player_reset_without_changes() {
    use egui_kittest::kittest::{by, Queryable};
    let (root, library) = library();
    let mut game = library.games[0].clone();
    let source = game.directory.join("disks/Disk 2.adf");
    std::fs::write(&source, vec![0x22; 901120]).unwrap();
    game.manifest.disks.push(GameDisk {
        id: "disk-two".into(),
        filename: "Disk 2.adf".into(),
    });
    add_library_disk(&mut game, "disk-three", "Disk 3.adf", 0x44);
    let mut app = game_app(&root);
    app.player.catalog.configurations[0].launch.drives = [true, true, true, false];
    app.start_library_game(game.clone(), None, true);
    let progress = common::snapshot_json(&mut app);
    let save = gui::game_library::persist_save(&game, &progress, None).unwrap();
    drop(app);
    std::fs::remove_file(&source).unwrap();

    let invalid_adf = b"readable but invalid ADF";
    for (drive, earlier_disk, invalid, expected_error) in [
        (2, "disk-one", false, "unavailable"),
        (3, "disk-one", false, "disconnected"),
        // DF1 requires a valid, previously unused ADF before preparing the invalid DF2.
        (2, "disk-three", true, "Unsupported ADF length"),
    ] {
        if invalid {
            std::fs::write(&source, invalid_adf).unwrap();
        }
        game.manifest.mounted = [None, Some(earlier_disk.into()), None, None];
        game.manifest.mounted[drive] = Some("disk-two".into());
        let mut app = game_app(&root);
        app.player.catalog.configurations[0].launch.drives = [true, true, true, false];
        app.start_library_game(game.clone(), Some(save.clone()), false);
        let mut h = session_harness(app);
        common::wait_harness(&mut h, |app| app.host.status.running);
        assert_eq!(
            h.state().player.disks[1].bytes.as_deref(),
            invalid.then_some(invalid_adf.as_slice())
        );
        let mounted = [Some("disk-one".into()), None, None, None];
        let before = common::snapshot(h.state_mut());
        assert!(before.floppy.drives[0].disk_inserted);
        assert!(!before.floppy.drives[3].connected);
        for running in [true, false] {
            if !running {
                h.get_by_label("Pause").click_accesskit();
                common::wait_harness(&mut h, |app| !app.host.status.running);
            }
            let prior = h.state().host.status;
            assert_eq!(prior.running, running);
            assert_player_drive_text(&h, "DF0:", "Disk 1.adf");
            h.get_by_label("Reset").click_accesskit();
            common::wait_harness(&mut h, |app| app.file_error.is_some());
            let error = h.state().file_error.as_deref().unwrap();
            assert!(error.contains(expected_error));
            assert!(error.contains(&format!("DF{drive}")));
            h.get(
                by().role(egui::accesskit::Role::Label)
                    .value(error)
                    .predicate(|node| !node.is_hidden()),
            );
            h.get_by_label(if running { "Pause" } else { "Resume" });
            let rejected = h.state().host.status;
            assert_eq!(rejected.running, running);
            assert_eq!(rejected.generation, prior.generation);
            assert_eq!(rejected.drive_reset_sequence, prior.drive_reset_sequence);
            assert_eq!(rejected.drives, prior.drives);
            assert_eq!(h.state().player.mounted, mounted);
            if running {
                assert!(rejected.cck >= prior.cck);
            } else {
                assert_eq!(rejected.cck, prior.cck);
            }
            h.get_by_label("Close").click_accesskit();
            common::settle_harness(&mut h);
            assert_player_drive_text(&h, "DF0:", "Disk 1.adf");
            let snapshot = common::observe_save(h.state_mut());
            assert_eq!(snapshot.header.association.mounted, mounted);
            assert_eq!(snapshot.state.floppy.drives, before.floppy.drives);
        }
    }
}

#[test]
fn normal_boot_with_a_missing_or_invalid_required_adf_keeps_the_current_machine() {
    use egui_kittest::kittest::{by, Queryable};

    let (root, library) = library();
    let mut next = library.games[1].clone();
    let required = add_library_disk(&mut next, "required", "Required disk.adf", 0x33);
    next.manifest.mounted[2] = Some("required".into());
    std::fs::remove_file(&required).unwrap();

    let mut app = game_app(&root);
    app.player.catalog.configurations[0].launch.drives = [true; 4];
    app.start_library_game(library.games[0].clone(), None, true);
    let mut h = session_harness(app);
    common::wait_harness(&mut h, |app| app.host.status.running);
    h.get_by_label("Pause").click_accesskit();
    common::wait_harness(&mut h, |app| !app.host.status.running);
    let layout = [Some(("disk-one", "Disk 1.adf", 0x11)), None, None, None];
    let before = assert_player_disk_layout(&mut h, layout);
    let prior = h.state().host.status;

    for (bytes, expected_error) in [
        (None, "unavailable"),
        (
            Some(b"readable but invalid ADF".as_slice()),
            "unsupported ADF",
        ),
    ] {
        if let Some(bytes) = bytes {
            std::fs::write(&required, bytes).unwrap();
        }
        h.state_mut().start_library_game(next.clone(), None, true);
        common::wait_harness(&mut h, |app| app.file_error.is_some());
        let error = h.state().file_error.as_deref().unwrap();
        assert!(error.contains("Required disk.adf for DF2"), "{error}");
        assert!(error.contains(expected_error), "{error}");
        h.get(
            by().role(egui::accesskit::Role::Label)
                .value(error)
                .predicate(|node| !node.is_hidden()),
        );
        assert_eq!(h.state().host.status.generation, prior.generation);
        assert_eq!(
            h.state().host.status.drive_reset_sequence,
            prior.drive_reset_sequence
        );
        assert!(!h.state().host.status.running);
        assert_eq!(
            h.state().player.active_game.as_ref().unwrap().manifest.id,
            library.games[0].manifest.id
        );
        h.get_by_label("Close").click_accesskit();
        common::settle_harness(&mut h);
        h.get_by_label("Resume");
        let after = assert_player_disk_layout(&mut h, layout);
        assert_eq!(after.header.association, before.header.association);
        assert_eq!(after.state, before.state);
    }
}

#[test]
fn saved_progress_selection_restores_only_after_replacement_is_confirmed() {
    use egui_kittest::kittest::Queryable;

    const SAVED_D7: u32 = 0x1234_5678;
    const LIVE_D7: u32 = 0x8765_4321;

    let (root, library) = library();
    let game = &library.games[0];
    let mut app = game_app(&root);
    app.start_library_game(game.clone(), None, false);
    let mut h = session_harness(app);
    common::settle_harness(&mut h);
    h.get_by_label("Pause").click();
    common::wait_harness(&mut h, |app| !app.host.status.running);
    common::command(
        h.state_mut(),
        HostCommand::SetRegister {
            register: gui::EditRegister::D(7),
            value: SAVED_D7,
        },
    );
    h.get_by_label("Save").click();
    common::settle_harness(&mut h);
    let saves = gui::game_library::saves(game).unwrap();
    assert_eq!(saves.len(), 1);
    let saved =
        debugger::SaveState::from_json(&std::fs::read_to_string(&saves[0].path).unwrap()).unwrap();
    assert_eq!(saved.header.version, debugger::SAVE_STATE_VERSION);
    assert_eq!(saved.cpu().d_long(7), SAVED_D7);

    common::command(
        h.state_mut(),
        HostCommand::SetRegister {
            register: gui::EditRegister::D(7),
            value: LIVE_D7,
        },
    );
    let live = common::snapshot(h.state_mut());
    assert_eq!(live.cpu.d_long(7), LIVE_D7);
    let generation = h.state().host.status.generation;

    let choose_save = |h: &mut egui_kittest::Harness<'_, EmulatorApp>| {
        h.get_by_label("Load").click();
        common::settle_harness(h);
        assert!(h.query_by_label("Saved progress").is_some());
        assert!(h.query_by_label("Replace session").is_none());

        h.get_by_label("Resume").click();
        common::settle_harness(h);
        assert!(!h.state().host.status.running);
        h.get_by_label_contains("Load state - ").click();
        common::settle_harness(h);
        assert!(h.query_by_label("Saved progress").is_none());
        assert!(h.query_by_label("Replace session").is_some());

        h.get_by_label("Resume").click();
        common::settle_harness(h);
        assert!(h.query_by_label("Keep current session").is_some());
        assert!(!h.state().host.status.running);
        assert_eq!(h.state().host.status.generation, generation);
        assert_eq!(common::snapshot(h.state_mut()), live);
    };

    choose_save(&mut h);
    h.get_by_label("Keep current session").click();
    common::settle_harness(&mut h);
    assert!(h.query_by_label("Replace session").is_none());
    assert_eq!(h.state().host.status.generation, generation);
    assert_eq!(common::snapshot(h.state_mut()), live);

    choose_save(&mut h);
    h.get_by_label("Replace session").click();
    common::settle_harness(&mut h);
    assert!(h.query_by_label("Replace session").is_none());
    assert!(h.state().host.status.running);
    assert_eq!(h.state().player.screen, PlayerScreen::Session);
    assert!(h.state().file_error.is_none());
    let restored = debugger::SaveState::from_json(&common::snapshot_json(h.state_mut())).unwrap();
    assert_eq!(restored.cpu().d_long(7), SAVED_D7);
    assert_eq!(
        restored.header.association.game_id.as_deref(),
        Some(game.manifest.id.as_str())
    );
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
    assert_eq!(h.state().player.selected_game.as_deref(), Some("two"));
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
    assert!(h.ctx.content_rect().contains_rect(bounds));
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
    let numbered = directory_files(&directory)
        .into_iter()
        .map(|path| (path.clone(), std::fs::read(path).unwrap()))
        .collect::<Vec<_>>();
    let other_files = directory_files(&two.directory);
    h.get_by_label("Cover").click_accesskit();
    h.step();
    assert!(image::open(one.directory.join("cover.png")).is_ok());
    assert_eq!(directory_files(&two.directory), other_files);
    assert_eq!([one, two].map(saved_progress_and_artwork), before);
    assert_eq!(
        directory_files(&directory),
        numbered
            .iter()
            .map(|(path, _)| path)
            .cloned()
            .collect::<Vec<_>>()
    );
    for (path, bytes) in numbered {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
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

struct ScreenTextureObserver {
    size: [usize; 2],
    capture: common::ScreenCapture,
}

impl egui_kittest::TestRenderer for ScreenTextureObserver {
    fn handle_delta(&mut self, delta: &mut egui::TexturesDelta) {
        self.capture.apply_delta(delta);
        delta.clear();
    }

    fn render(
        &mut self,
        _ctx: &egui::Context,
        output: &egui::FullOutput,
    ) -> Result<image::RgbaImage, String> {
        let texture = self.capture.id.ok_or("The game texture was not uploaded")?;
        let clipped = output.shapes.iter().any(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) if mesh.texture_id == texture => {
                let bounds = mesh.calc_bounds();
                bounds.width() > shape.clip_rect.width()
                    && bounds.height() > shape.clip_rect.height()
            }
            _ => false,
        });
        if !clipped {
            return Err("The game texture must fill a clipped mesh".into());
        }
        let [width, height] = self.size;
        if self.capture.pixels.len() != width * height {
            return Err("The observed texture has an unexpected pixel count".into());
        }
        // Return the full uploaded texture, including parts clipped in the viewport.
        let pixel_at = |x: u32, y: u32| {
            let pixel = self.capture.pixels[y as usize * width + x as usize];
            image::Rgba(pixel.to_srgba_unmultiplied())
        };
        Ok(image::RgbaImage::from_fn(
            width as u32,
            height as u32,
            pixel_at,
        ))
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
        // The paused image can arrive during harness construction. Observe each
        // texture update starting with the first frame.
        let mut h = session_harness_builder()
            .renderer(ScreenTextureObserver {
                size: [width, height],
                capture: common::ScreenCapture::new([width, height]),
            })
            .build_ui_state(|ui, app| app.update_ui(ui), app);
        h.ctx.set_zoom_factor(1.25);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if h.state().host.is_idle()
                && h.state().host.presented.is_some()
                && h.ctx.zoom_factor() == 1.25
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Prepared {standard:?} texture did not reach Game view at 1.25 zoom"
            );
            h.step();
            std::thread::sleep(Duration::from_millis(1));
        }
        let observed_texture = h.render().unwrap();
        assert_eq!(observed_texture.as_raw(), &expected);

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
            if pending.is_none() {
                h.get_by_label("Cover").click_accesskit();
                h.step();
                let cover = image::open(game.directory.join("cover.png"))
                    .unwrap()
                    .into_rgba8();
                assert_eq!(cover.dimensions(), (width as u32, height as u32));
                assert_eq!(cover.as_raw(), &expected);
            }
            let Some((pending, generation, running)) = pending else {
                break;
            };
            if let Some(presented) = h.state().host.presented.filter(|info| *info != previous) {
                assert_eq!(h.state().host.status.running, running);
                assert_eq!(presented.generation, generation);
                assert_eq!(presented.publication, pending);
                let observed_texture = h.render().unwrap();
                assert_ne!(observed_texture.as_raw(), &expected);
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
    let (second_root, second_library) = library();
    let first = &first_library.games[0];
    let second = &second_library.games[0];
    assert_eq!(first.manifest.id, second.manifest.id);
    let mut app = game_app(&root);
    app.start_library_game(first.clone(), None, false);
    let mut h = session_harness(app);
    common::wait_harness(&mut h, |app| {
        app.player.active_game.is_some() && app.host.presented.is_some()
    });

    select_library_with_current_profiles(&mut h, &second_root);
    assert_eq!(
        h.state().player.active_game.as_ref().unwrap().directory,
        first.directory
    );

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

    common::settle_harness(&mut h);
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
    common::select_library(&mut h, &second_root);
    common::wait_harness(&mut h, |app| app.player.library.games.len() == 2);
    h.get_by_label("one").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Edit game").click_accesskit();
    common::settle_harness(&mut h);
    h.state_mut().player.library_editor.as_mut().unwrap().name = "Renamed".into();
    h.step();
    h.get_by_label("Save game").click_accesskit();
    settle_caption(&mut h, &[]);
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
    h.get_by_label("Cover").click_accesskit();
    h.step();
    assert!(image::open(first.directory.join("cover.png")).is_ok());
    assert!(!renamed.join("cover.png").exists());
    assert_eq!(
        h.state().player.active_game.as_ref().unwrap().directory,
        first.directory
    );
}

#[test]
fn capture_buttons_are_adjacent_in_regular_and_compact_library_game_views_only() {
    use egui_kittest::kittest::Queryable;

    let (root, library) = library();
    let mut manual = game_app(&root);
    manual.start_manual();
    let mut h = session_harness(manual);
    common::wait_harness(&mut h, |app| app.player.active_configuration.is_some());
    assert_eq!(h.state().player.screen, PlayerScreen::Session);
    assert!(h.query_by_label("Screenshot").is_none());
    assert!(h.query_by_label("Cover").is_none());

    let mut app = game_app(&root);
    app.start_library_game(library.games[0].clone(), None, false);
    let mut h = session_harness(app);
    common::wait_harness(&mut h, |app| app.player.active_game.is_some());
    let assert_capture_controls = |h: &egui_kittest::Harness<'_, EmulatorApp>| {
        let screenshot = h.get_by_label("Screenshot");
        let cover = h.get_by_label("Cover");
        assert!(!screenshot.accesskit_node().is_disabled());
        assert!(!cover.accesskit_node().is_disabled());
        assert!(h.ctx.content_rect().contains_rect(cover.rect()));
        assert!(cover.rect().left() >= screenshot.rect().right());
        assert!(cover.rect().top() < screenshot.rect().bottom());
        assert!(cover.rect().bottom() > screenshot.rect().top());
    };
    assert_capture_controls(&h);
    h.set_size(egui::vec2(640.0, 720.0));
    common::settle_harness(&mut h);
    h.get_by_label("Game controls").click_accesskit();
    common::settle_harness(&mut h);
    assert_capture_controls(&h);
    let generation = h.state().host.status.generation;
    for pressed in [true, false] {
        assert!(gui::keyboard_input::receive_key(
            &h.ctx,
            common::keyboard::key("F12", pressed),
            Some(generation)
        ));
    }
    common::settle_harness(&mut h);
    assert_eq!(h.state().view_mode, ViewMode::Developer);
    assert!(h.query_by_label("Screenshot").is_none());
    assert!(h.query_by_label("Cover").is_none());
}

#[derive(Default)]
struct CoverTextureObserver {
    covers: std::collections::HashMap<egui::TextureId, egui::ColorImage>,
}

impl egui_kittest::TestRenderer for CoverTextureObserver {
    fn handle_delta(&mut self, delta: &mut egui::TexturesDelta) {
        for (id, update) in common::texture_updates(delta) {
            let egui::ImageData::Color(image) = &update.image;
            if image.size == [720, 568] && update.pos.is_none() {
                self.covers.insert(*id, image.as_ref().clone());
            }
        }
        for id in &delta.free {
            self.covers.remove(id);
        }
        delta.clear();
    }

    fn render(
        &mut self,
        _ctx: &egui::Context,
        output: &egui::FullOutput,
    ) -> Result<image::RgbaImage, String> {
        let image = output
            .shapes
            .iter()
            .find_map(|shape| {
                let egui::Shape::Rect(rect) = &shape.shape else {
                    return None;
                };
                let brush = rect.brush.as_ref()?;
                let bounds = rect.rect;
                (rect.fill.a() > 0
                    && shape.clip_rect.contains_rect(bounds)
                    && (bounds.width() - 110.0).abs() <= 1.0)
                    .then(|| self.covers.get(&brush.fill_texture_id))
                    .flatten()
            })
            .ok_or("The library did not draw the captured cover")?;
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Text(text) if text.galley.text() == "one"
        )));
        let bytes = image
            .pixels
            .iter()
            .flat_map(egui::Color32::to_srgba_unmultiplied)
            .collect();
        image::RgbaImage::from_raw(720, 568, bytes).ok_or_else(|| "Unexpected cover size".into())
    }
}

#[test]
fn cover_replaces_one_png_and_updates_the_drawn_thumbnail_at_each_precedence() {
    use egui_kittest::kittest::Queryable;

    for (declared, higher_priority) in [
        (Some("alternate.png"), Some("alternate.png")),
        (None, Some("cover.jpg")),
        (None, None),
    ] {
        let (root, library) = library();
        let one = &library.games[0];
        let png = one.directory.join("cover.png");
        image::RgbImage::from_pixel(5, 3, image::Rgb([12, 34, 56]))
            .save(&png)
            .unwrap();
        let competing = higher_priority.map(|name| {
            let path = one.directory.join(name);
            image::RgbImage::from_pixel(8, 3, image::Rgb([240, 16, 63]))
                .save(&path)
                .unwrap();
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        });
        let manifest_path = one.directory.join("game.json");
        let mut manifest = serde_json::to_value(&one.manifest).unwrap();
        manifest["artwork"] = serde_json::json!(declared);
        manifest["custom"] = serde_json::json!({"preserve": [1, 2]});
        let before = serde_json::to_vec(&manifest).unwrap();
        std::fs::write(&manifest_path, &before).unwrap();
        let files_before = directory_files(&one.directory);

        let mut session = Box::new(gui::DebuggerSession::new());
        session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
        let frame = &mut session.machine.denise.frame_builder;
        let stride = frame.row_stride();
        let first = image::RgbaImage::from_fn(720, 568, |x, y| {
            let rgba = screenshot_pixel(x as usize, y as usize, 720, 568);
            let [r, g, b, a] = rgba;
            frame.frame_buffer_mut()[y as usize * stride + x as usize] =
                u32::from_be_bytes([a, r, g, b]);
            image::Rgba(rgba)
        });
        let mut app = EmulatorApp::with_session(session);
        app.set_view_mode(ViewMode::ScreenOnly);
        app.player.library_root = Some(root.clone());
        app.player.library = GameLibrary::discover(&root).unwrap();
        app.player.active_game = Some(app.player.library.games[0].clone());
        app.screen_scale = gui::ScreenScale::Four;
        let mut h = session_harness_builder()
            .renderer(CoverTextureObserver::default())
            .build_ui_state(|ui, app| app.update_ui(ui), app);
        common::wait_harness(&mut h, |app| app.host.presented.is_some());
        h.get_by_label("Library").click_accesskit();
        common::settle_harness(&mut h);
        h.get_by_label("two").click_accesskit();
        common::settle_harness(&mut h);
        h.get_by_label("Return to session").click_accesskit();
        common::settle_harness(&mut h);
        let capture = |h: &mut egui_kittest::Harness<'_, EmulatorApp>| {
            h.get_by_label("Cover").click_accesskit();
            h.step();
            assert!(h.state().file_error.is_none());
            let captured = image::open(&png).unwrap().into_rgba8();
            h.get_by_label("Library").click_accesskit();
            common::settle_harness(h);
            assert_eq!(h.state().player.selected_game.as_deref(), Some("two"));
            assert_eq!(h.render().unwrap(), captured);
            captured
        };
        assert_eq!(capture(&mut h), first);
        let manifest_after_first = std::fs::read(&manifest_path).unwrap();
        if let Some((path, bytes)) = &competing {
            manifest["artwork"] = serde_json::json!("cover.png");
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&manifest_after_first).unwrap(),
                manifest
            );
            assert_eq!(std::fs::read(path).unwrap(), *bytes);
        } else {
            assert_eq!(manifest_after_first, before);
        }
        h.get_by_label("Return to session").click_accesskit();
        common::settle_harness(&mut h);
        let first_generation = h.state().host.presented.unwrap().generation;
        h.state_mut().host.queue(HostCommand::Reset);
        common::wait_harness(&mut h, |app| {
            app.host
                .presented
                .is_some_and(|image| image.generation != first_generation)
        });
        let second = capture(&mut h);
        assert_ne!(second, first);
        assert_eq!(std::fs::read(&manifest_path).unwrap(), manifest_after_first);
        assert_eq!(directory_files(&one.directory), files_before);
        if let Some((path, bytes)) = &competing {
            assert_eq!(std::fs::read(path).unwrap(), *bytes);
        }

        let mut reopened = game_app(&root);
        reopened.navigate_player(PlayerScreen::Library);
        let mut reopened = session_harness_builder()
            .renderer(CoverTextureObserver::default())
            .build_ui_state(|ui, app| app.update_ui(ui), reopened);
        common::settle_harness(&mut reopened);
        assert_eq!(reopened.render().unwrap(), second);
    }
}

#[test]
fn cover_write_failure_keeps_previous_png_and_manifest_and_manifest_failure_allows_retry() {
    use egui_kittest::kittest::Queryable;
    use std::io::{Error, Write};

    let (root, library) = library();
    let game = &library.games[0];
    let previous = image::RgbImage::from_pixel(2, 1, image::Rgb([200, 30, 50]));
    previous
        .save(game.directory.join("missing-cover.png"))
        .unwrap();
    let cover = game.directory.join("cover.png");
    previous.save(&cover).unwrap();
    let previous_cover = std::fs::read(&cover).unwrap();
    let manifest = game.directory.join("game.json");
    let before = std::fs::read(&manifest).unwrap();
    let files_before = directory_files(&game.directory);
    let error = gui::game_library::persist_cover(game, |file| {
        file.write_all(b"\x89PNG\r\n\x1a\n")?;
        assert_eq!(std::fs::read(&cover)?, previous_cover);
        Err(Error::other("simulated full disk"))
    })
    .unwrap_err();
    assert_eq!(error, "simulated full disk");
    assert_eq!(std::fs::read(&cover).unwrap(), previous_cover);
    assert_eq!(std::fs::read(&manifest).unwrap(), before);
    assert_eq!(directory_files(&game.directory), files_before);

    let blocker = game.directory.join("game.json.pending");
    std::fs::create_dir(&blocker).unwrap();
    let mut app = game_app(&root);
    app.start_library_game(game.clone(), None, false);
    let mut h = session_harness(app);
    common::wait_harness(&mut h, |app| {
        app.player.active_game.is_some() && app.host.presented.is_some()
    });
    h.get_by_label("Pause").click_accesskit();
    common::wait_harness(&mut h, |app| !app.host.status.running);
    h.get_by_label("Cover").click_accesskit();
    h.step();
    let error = h.state().file_error.clone().unwrap();
    assert!(error.contains("Saved cover.png but could not select it in game.json"));
    common::settle_harness(&mut h);
    assert!(h.get_all_by_value(&error).next().is_some());
    assert_eq!(std::fs::read(&manifest).unwrap(), before);
    assert_eq!(
        image::open(&cover).unwrap().to_rgba8().dimensions(),
        (720, 568)
    );
    assert_ne!(std::fs::read(&cover).unwrap(), previous_cover);
    let discovered = GameLibrary::discover(&root).unwrap();
    assert_eq!(
        image::load_from_memory(discovered.games[0].artwork.as_deref().unwrap())
            .unwrap()
            .to_rgb8(),
        previous
    );

    std::fs::remove_dir(&blocker).unwrap();
    h.get_by_label("Close").click_accesskit();
    h.step();
    h.get_by_label("Cover").click_accesskit();
    h.step();
    assert!(h.state().file_error.is_none());
    let discovered = GameLibrary::discover(&root).unwrap();
    assert_eq!(
        discovered.games[0].manifest.artwork.as_deref(),
        Some("cover.png")
    );
    assert_eq!(
        discovered.games[0].artwork.as_deref().unwrap(),
        std::fs::read(&cover).unwrap()
    );
    h.get_by_label("Resume").click_accesskit();
    common::wait_harness(&mut h, |app| app.host.status.running);
}

#[test]
fn cover_before_the_new_game_displays_a_frame_preserves_that_games_existing_artwork() {
    use egui_kittest::kittest::Queryable;
    use std::time::{Duration, Instant};

    let (root, first_library) = library();
    let (second_root, second_library) = library();
    let first = &first_library.games[0];
    let second = &second_library.games[0];
    assert_eq!(first.manifest.id, second.manifest.id);
    let cover = second.directory.join("cover.png");
    image::RgbImage::from_pixel(2, 1, image::Rgb([20, 60, 100]))
        .save(&cover)
        .unwrap();
    let png = std::fs::read(&cover).unwrap();
    let manifest_path = second.directory.join("game.json");
    let manifest = std::fs::read(&manifest_path).unwrap();
    let mut app = game_app(&root);
    app.start_library_game(first.clone(), None, false);
    let mut h = session_harness(app);
    common::wait_harness(&mut h, |app| {
        app.player.active_game.is_some() && app.host.presented.is_some()
    });
    select_library_with_current_profiles(&mut h, &second_root);
    assert_eq!(
        h.state().player.active_game.as_ref().unwrap().directory,
        first.directory
    );
    h.state_mut()
        .start_library_game(second.clone(), None, false);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        h.get_by_label("Cover").click_accesskit();
        h.step();
        if h.state().player.active_game.as_ref().unwrap().directory == second.directory {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "The second library game did not start"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        h.state().file_error.as_deref(),
        Some("Could not save cover: No game image is available")
    );
    assert_eq!(std::fs::read(&cover).unwrap(), png);
    assert_eq!(std::fs::read(&manifest_path).unwrap(), manifest);

    common::settle_harness(&mut h);
    h.get_by_label("Close").click_accesskit();
    common::wait_harness(&mut h, |app| {
        app.file_error.is_none() && app.host.presented.is_some()
    });
    h.get_by_label("Cover").click_accesskit();
    h.step();
    assert!(h.state().file_error.is_none());
    assert_ne!(std::fs::read(cover).unwrap(), png);
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
