#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use gui::game_library::{GameLibrary, GameManifest};
use gui::host_commands::HostCommand;
use gui::library_refresh::{LibraryRefreshSchedule, REFRESH_DELAY};
use gui::player::PlayerScreen;
use gui::{EmulatorApp, ViewMode};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

#[test]
fn bursts_coalesce_and_overflow_keeps_its_reconciliation_request() {
    let start = Instant::now();
    let mut schedule = LibraryRefreshSchedule::default();
    let callback = schedule.notifications();
    assert!(callback.changed());
    for _ in 0..500 {
        assert!(!callback.changed());
    }
    schedule.receive(start);
    assert_eq!(schedule.deadline(), Some(start + REFRESH_DELAY));
    assert!(callback.changed());
    assert!(callback.rescan());
    let before_due = start + REFRESH_DELAY - Duration::from_millis(1);
    schedule.receive(before_due);
    assert!(schedule.needs_rescan());
    assert_eq!(schedule.take_ready(before_due), None);
    assert_eq!(schedule.take_ready(start + REFRESH_DELAY), Some(true));
    assert_eq!(schedule.take_ready(start + REFRESH_DELAY), None);

    assert!(callback.failed("backend stopped".into()));
    schedule.receive(start + REFRESH_DELAY);
    assert_eq!(schedule.take_error().as_deref(), Some("backend stopped"));
    assert_eq!(schedule.take_ready(start + 2 * REFRESH_DELAY), Some(true));
    assert_eq!(schedule.take_ready(start + 3 * REFRESH_DELAY), None);
}

#[test]
fn old_root_callbacks_cannot_schedule_work_after_rebinding() {
    let start = Instant::now();
    let old = LibraryRefreshSchedule::default();
    let old_callback = old.notifications();
    assert!(old_callback.changed());
    assert!(old_callback.rescan());
    drop(old);

    let mut current = LibraryRefreshSchedule::default();
    assert!(!old_callback.changed());
    assert!(!old_callback.rescan());
    assert!(!old_callback.failed("old root".into()));
    current.receive(start);
    assert_eq!(current.take_error(), None);
    assert_eq!(current.take_ready(start + REFRESH_DELAY), None);
    assert!(current.notifications().changed());
    current.receive(start);
    assert_eq!(current.take_ready(start + REFRESH_DELAY), Some(false));
}

fn write_game(directory: &Path, id: &str, title: &str) -> GameManifest {
    fs::create_dir_all(directory.join("assets")).unwrap();
    let manifest = GameManifest {
        id: id.into(),
        title: title.into(),
        configuration: "standard".into(),
        disks: Vec::new(),
        mounted: [None, None, None, None],
        initial_state: None,
        artwork: None,
        description: Some("assets/description.md".into()),
    };
    fs::write(
        directory.join("game.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    manifest
}

fn harness(app: EmulatorApp) -> Harness<'static, EmulatorApp> {
    Harness::builder()
        .with_size(egui::vec2(1024.0, 760.0))
        .build_state(|ctx, app: &mut EmulatorApp| app.update_ui(ctx), app)
}

fn browse(root: &Path) -> EmulatorApp {
    let mut app = EmulatorApp::default();
    app.player.catalog.library_root = root.to_string_lossy().into_owned();
    app.navigate_player(PlayerScreen::Library);
    app
}

fn filter<'a>(h: &'a Harness<'_, EmulatorApp>) -> egui_kittest::Node<'a> {
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Filter games")
}

#[test]
fn native_library_refreshes_while_paused_and_preserves_filter_selection_and_drafts() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    write_game(&alpha, "alpha", "Alpha Mission");
    write_game(&beta, "beta", "BETA Patrol");
    let mut rom = vec![0; 262144];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    let rom_path = root.join("kick.rom");
    fs::write(&rom_path, rom).unwrap();
    let mut app = browse(root);
    let ctx = egui::Context::default();
    common::wait(&mut app, &ctx, |app| app.player.library.games.len() == 2);
    app.player.catalog.configurations[0].launch.rom = rom_path.to_string_lossy().into_owned();
    let active = app.player.library.games[0].clone();
    app.start_library_game(active, None, true);
    common::wait(&mut app, &ctx, |app| app.host.status.running);
    fs::write(alpha.join("assets/description.md"), "Changed during run").unwrap();
    common::wait(&mut app, &ctx, |app| {
        app.player.library.games[0].description == "Changed during run"
    });
    common::command(&mut app, HostCommand::Pause);
    common::wait(&mut app, &ctx, |app| !app.host.status.running);
    let save_json = common::snapshot_json(&mut app);
    let mut h = harness(app);
    h.get_by_label("Save").click_accesskit();
    common::wait_harness(&mut h, |app| app.player.saves.len() == 1);
    assert!(h.state().file_error.is_none());

    // The external copy becomes visible only when the complete file is published.
    let pending = alpha.join("saves/external.json.pending");
    fs::write(&pending, "{").unwrap();
    fs::write(alpha.join("assets/description.md"), "Pending save ignored").unwrap();
    common::wait_harness(&mut h, |app| {
        app.player.library.games[0].description == "Pending save ignored"
    });
    assert_eq!(h.state().player.saves.len(), 1);
    assert!(h.state().file_error.is_none());
    fs::write(&pending, save_json).unwrap();
    let published = alpha.join("saves/external.json");
    fs::rename(pending, &published).unwrap();
    common::wait_harness(&mut h, |app| app.player.saves.len() == 2);
    fs::remove_file(&published).unwrap();
    common::wait_harness(&mut h, |app| app.player.saves.len() == 1);

    h.get_by_label("Library").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.query_by_label("Choose library directory...").is_none());
    assert!(h.query_by_label("Refresh").is_none());
    h.get_by_label("Alpha Mission").click_accesskit();
    common::settle_harness(&mut h);
    let before = h.state().host.status;
    filter(&h).focus();
    filter(&h).type_text("pAtRoL");
    h.step();
    assert_eq!(h.state().player.library_filter, "pAtRoL");
    assert_eq!(h.state().player.selected_game.as_deref(), Some("alpha"));
    assert!(h.query_by_label("Alpha Mission").is_none());
    assert!(h.query_by_label("Start game").is_none());
    assert!(h.query_by_label("BETA Patrol").is_some());
    h.set_size(egui::vec2(350.0, 650.0));
    h.step();
    assert!(h.ctx.screen_rect().contains_rect(filter(&h).rect()));
    assert!(filter(&h).is_focused());
    filter(&h).type_text("xyz");
    h.step();
    assert!(h
        .get_all_by_value("No games match the filter.")
        .next()
        .is_some());

    // A fully written manifest under a temporary container remains invisible.
    let unpublished = root.join(".external-import/game");
    write_game(&unpublished, "gamma", "00 Gamma");
    let beta_renamed = root.join("beta-renamed");
    fs::rename(&beta, &beta_renamed).unwrap();
    common::wait_harness(&mut h, |app| {
        app.player.library.games[1].directory == beta_renamed
    });
    assert_eq!(h.state().player.library.games.len(), 2);
    fs::rename(&unpublished, root.join("gamma")).unwrap();
    common::wait_harness(&mut h, |app| app.player.library.games.len() == 3);
    assert_eq!(h.state().player.library_filter, "pAtRoLxyz");
    assert_eq!(h.state().player.selected_game.as_deref(), Some("alpha"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.key_press(egui::Key::Backspace);
    h.step();
    assert!(h.state().player.library_filter.is_empty());
    assert_eq!(
        h.state()
            .player
            .library
            .games
            .iter()
            .map(|game| game.manifest.title.as_str())
            .collect::<Vec<_>>(),
        ["00 Gamma", "Alpha Mission", "BETA Patrol"]
    );
    assert!(h.query_by_label("Start game").is_some());
    h.set_size(egui::vec2(1024.0, 760.0));
    h.step();

    h.get_by_label("Edit game").click_accesskit();
    common::settle_harness(&mut h);
    h.state_mut().player.library_editor.as_mut().unwrap().name = "Unsaved title".into();
    fs::write(alpha.join("assets/description.md"), "External description").unwrap();
    common::wait_harness(&mut h, |app| {
        app.player.library.games[1].description == "External description"
    });
    assert_eq!(
        h.state().player.library_editor.as_ref().unwrap().name,
        "Unsaved title"
    );
    h.get_by_label("Save game").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h
        .state()
        .file_error
        .as_ref()
        .unwrap()
        .contains("Game files changed on disk"));
    assert!(!root.join("Unsaved title").exists());
    assert_eq!(h.state().host.status.generation, before.generation);
    assert_eq!(h.state().host.status.cck, before.cck);
    assert!(!h.state().host.status.running);
    assert_eq!(h.state().view_mode, ViewMode::ScreenOnly);
    assert_eq!(
        h.state().player.active_game.as_ref().unwrap().manifest.id,
        "alpha"
    );
}

#[test]
fn native_root_replacement_recovers_and_committed_switch_ignores_old_changes() {
    let temporary = tempfile::tempdir().unwrap();
    let old = temporary.path().join("old");
    let new = temporary.path().join("new");
    let displaced = temporary.path().join("displaced");
    write_game(&old.join("game"), "shared", "First library");
    write_game(&new.join("game"), "shared", "Second library");
    let mut h = harness(browse(&old));
    common::wait_harness(&mut h, |app| app.player.library.games.len() == 1);
    fs::rename(&old, &displaced).unwrap();
    common::wait_harness(&mut h, |app| app.player.library.games.is_empty());
    assert!(h
        .query_by(|node| {
            node.role() == egui::accesskit::Role::Label
                && !node.is_hidden()
                && node.value().is_some_and(|value| {
                    value.starts_with("Could not read library:")
                        && value.contains("in Configurations")
                })
        })
        .is_some());
    assert!(h
        .query_by_label("Add game")
        .unwrap()
        .accesskit_node()
        .is_disabled());
    write_game(&old.join("game"), "shared", "Restored directory");
    common::wait_harness(&mut h, |app| {
        app.player
            .library
            .games
            .first()
            .is_some_and(|game| game.manifest.title == "Restored directory")
    });

    h.get_by_label("Restored directory").click_accesskit();
    common::settle_harness(&mut h);
    common::commit_library_directory(&mut h, &new);
    common::wait_harness(&mut h, |app| {
        app.player
            .library
            .games
            .first()
            .is_some_and(|game| game.manifest.title == "Second library")
    });
    assert!(h.state().player.selected_game.is_none());
    fs::write(old.join("game/assets/description.md"), "Old root edit").unwrap();
    fs::write(new.join("game/assets/description.md"), "Current root edit").unwrap();
    common::wait_harness(&mut h, |app| {
        app.player.library.games[0].description == "Current root edit"
    });
    assert_eq!(h.state().player.library.games.len(), 1);
    assert_eq!(
        h.state().player.library.games[0].manifest.title,
        "Second library"
    );
    assert!(GameLibrary::discover(&displaced).is_ok());
}
