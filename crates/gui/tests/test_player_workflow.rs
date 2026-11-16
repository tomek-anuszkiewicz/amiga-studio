#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]
mod common;
use gui::game_library::{GameDisk, GameLibrary, GameManifest};
use gui::host_commands::HostCommand;
use gui::player::PlayerScreen;
use gui::{EmulatorApp, ViewMode};
use std::path::PathBuf;

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
    app.player.catalog.configurations[0].rom = root.join("kick.rom").to_string_lossy().into_owned();
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
    reopened.player.catalog.configurations[0].preset = config::A500Preset::ExpandedPowerUser;
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
