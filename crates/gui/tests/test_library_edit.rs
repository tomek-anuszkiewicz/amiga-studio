#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;

use gui::game_library::GameLibrary;
use gui::library_edit::{validate_game_name, GameDraft};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.agent/tmp/library-editor")
        .join(format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn add_disk(draft: &mut GameDraft, directory: &Path, value: u8) -> PathBuf {
    let source = directory.join(format!("source-{value}.adf"));
    std::fs::write(&source, vec![value; 901120]).unwrap();
    draft.add_disk(&source).unwrap();
    source
}

#[test]
fn create_import_and_rename_preserve_media_ids_saved_progress_and_other_files() {
    let root = root();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "A new game".into();
    draft.description = "A short description".into();
    let first = add_disk(&mut draft, &root, 0x11);
    let second = add_disk(&mut draft, &root, 0x22);
    assert!(
        !root.join(&draft.name).exists(),
        "Picking files must not publish a game"
    );
    let mut game = draft.save(&root).unwrap();
    assert_eq!(game.directory.file_name().unwrap(), "A new game");
    assert_eq!(game.description, draft.description);
    assert_eq!(
        game.manifest.mounted,
        [Some("disk-1".into()), None, None, None]
    );
    assert_eq!(
        std::fs::read(game.directory.join(&game.manifest.disks[0].path)).unwrap(),
        std::fs::read(first).unwrap()
    );
    assert_eq!(
        std::fs::read(game.directory.join(&game.manifest.disks[1].path)).unwrap(),
        std::fs::read(second).unwrap()
    );
    std::fs::create_dir(game.directory.join("saves")).unwrap();
    std::fs::write(game.directory.join("saves/retained.txt"), b"saved progress").unwrap();
    std::fs::write(game.directory.join("initial.json"), b"initial state").unwrap();
    std::fs::write(game.directory.join("notes.txt"), b"user notes").unwrap();
    game.manifest.initial_state = Some("initial.json".into());
    std::fs::write(
        game.directory.join("game.json"),
        serde_json::to_vec(&game.manifest).unwrap(),
    )
    .unwrap();
    let id = game.manifest.id.clone();
    let before_disks = game.manifest.disks.clone();
    let mut edit = GameDraft::edit(game.clone());
    edit.name = "Renamed game".into();
    edit.description = "Edited description".into();
    let third = add_disk(&mut edit, &root, 0x33);
    let updated = edit.save(&root).unwrap();
    assert!(!game.directory.exists());
    assert_eq!(updated.manifest.id, id);
    assert_eq!(&updated.manifest.disks[..2], before_disks);
    assert_eq!(updated.manifest.mounted, game.manifest.mounted);
    assert_eq!(updated.manifest.initial_state, game.manifest.initial_state);
    assert_eq!(updated.description, "Edited description");
    for (path, content) in [
        ("saves/retained.txt", b"saved progress".as_slice()),
        ("initial.json", b"initial state"),
        ("notes.txt", b"user notes"),
    ] {
        assert_eq!(
            std::fs::read(updated.directory.join(path)).unwrap(),
            content
        );
    }
    assert_eq!(
        std::fs::read(updated.directory.join(&updated.manifest.disks[2].path)).unwrap(),
        std::fs::read(third).unwrap()
    );
    let rediscovered = GameLibrary::discover(&root).unwrap();
    assert_eq!(rediscovered.games.len(), 1);
    assert!(rediscovered.errors.is_empty());
}

#[test]
fn invalid_names_images_and_destination_collisions_leave_existing_game_untouched() {
    let root = root();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Original".into();
    let game = draft.save(&root).unwrap();
    let original = std::fs::read(game.directory.join("game.json")).unwrap();
    for invalid in [
        "",
        "../escape",
        "a/b",
        "a\\b",
        "CON",
        "aux.txt",
        "LPT1",
        "trailing.",
        " name",
        "x:y",
    ] {
        assert!(validate_game_name(invalid).is_err(), "{invalid}");
        let mut edit = GameDraft::edit(game.clone());
        edit.name = invalid.into();
        assert!(edit.save(&root).is_err());
        assert_eq!(
            std::fs::read(game.directory.join("game.json")).unwrap(),
            original
        );
    }
    let bad = root.join("bad.adf");
    std::fs::write(&bad, b"unsupported disk").unwrap();
    assert!(draft.add_disk(&bad).is_err());
    assert!(draft.imports.is_empty());
    std::fs::create_dir(root.join("Other")).unwrap();
    std::fs::write(root.join("Other/preserve.txt"), b"unrelated").unwrap();
    let mut edit = GameDraft::edit(game.clone());
    edit.name = "Other".into();
    edit.description = "Must not be written".into();
    assert!(edit.save(&root).is_err());
    assert_eq!(
        std::fs::read(game.directory.join("game.json")).unwrap(),
        original
    );
    assert!(!game.directory.join("description.md").exists());
    assert_eq!(
        std::fs::read(root.join("Other/preserve.txt")).unwrap(),
        b"unrelated"
    );
}

#[test]
fn jpeg_and_png_covers_are_discovered_and_refresh_reads_replacements() {
    let root = root();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Covered game".into();
    let game = draft.save(&root).unwrap();
    let cover = game.directory.join("cover.jpg");
    let image = image::RgbImage::from_pixel(2, 2, image::Rgb([240, 20, 10]));
    image.save(&cover).unwrap();
    let first = GameLibrary::discover(&root).unwrap().games.remove(0);
    assert!(
        first.manifest.artwork.is_none(),
        "The filename convention needs no manifest edit"
    );
    assert_eq!(
        image::load_from_memory(first.artwork.as_ref().unwrap())
            .unwrap()
            .width(),
        2
    );
    let mut app = gui::EmulatorApp::default();
    app.player.catalog.library_root = root.to_string_lossy().into_owned();
    app.player.library = GameLibrary::discover(&root).unwrap();
    app.navigate_player(gui::player::PlayerScreen::Library);
    let ctx = egui::Context::default();
    let uploaded_color = |output: egui::FullOutput| {
        output
            .textures_delta
            .set
            .into_iter()
            .find_map(|(_, delta)| match delta.image {
                egui::ImageData::Color(image) if image.size == [2, 2] => Some(image.pixels[0]),
                _ => None,
            })
            .expect("The displayed cover must be uploaded")
    };
    let red = uploaded_color(common::settle(&mut app, &ctx));
    assert!(red.r() > red.b());
    image::RgbImage::from_pixel(2, 2, image::Rgb([10, 20, 240]))
        .save(&cover)
        .unwrap();
    let second = GameLibrary::discover(&root).unwrap().games.remove(0);
    assert_ne!(first.artwork, second.artwork);
    app.player.library = GameLibrary::discover(&root).unwrap();
    let blue = uploaded_color(common::settle(&mut app, &ctx));
    assert!(
        blue.b() > blue.r(),
        "Refresh retained the obsolete cover texture"
    );
    std::fs::rename(&cover, game.directory.join("saved-cover.jpg")).unwrap();
    image.save(game.directory.join("cover.png")).unwrap();
    let png = GameLibrary::discover(&root).unwrap().games.remove(0);
    assert!(image::load_from_memory(png.artwork.as_ref().unwrap()).is_ok());
}

#[test]
fn editor_cancel_does_not_write_and_save_updates_the_active_games_path_without_restarting() {
    use egui_kittest::{kittest::Queryable, Harness};
    let root = root();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Before".into();
    add_disk(&mut draft, &root, 0x44);
    let game = draft.save(&root).unwrap();
    let rom_path = root.join("kick.rom");
    let mut rom = vec![0; 262144];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    std::fs::write(&rom_path, rom).unwrap();
    let mut app = gui::EmulatorApp::default();
    app.player.catalog.configurations[0].launch.rom = rom_path.to_string_lossy().into_owned();
    app.start_library_game(game.clone(), None, false);
    common::settle(&mut app, &egui::Context::default());
    app.player.catalog.library_root = root.to_string_lossy().into_owned();
    app.player.library = GameLibrary::discover(&root).unwrap();
    app.player.selected_game = Some(0);
    app.navigate_player(gui::player::PlayerScreen::Library);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 800.0))
        .build_state(|ctx, app: &mut gui::EmulatorApp| app.update_ui(ctx), app);
    common::settle_harness(&mut harness);
    harness.get_by_label("Add game").click_accesskit();
    common::settle_harness(&mut harness);
    harness
        .state_mut()
        .player
        .library_editor
        .as_mut()
        .unwrap()
        .name = "Cancelled".into();
    harness.step();
    harness.get_by_label("Cancel").click_accesskit();
    common::settle_harness(&mut harness);
    assert!(!root.join("Cancelled").exists());
    harness.get_by_label("Edit game").click_accesskit();
    common::settle_harness(&mut harness);
    harness
        .state_mut()
        .player
        .library_editor
        .as_mut()
        .unwrap()
        .name = "After".into();
    harness.step();
    add_disk(
        harness.state_mut().player.library_editor.as_mut().unwrap(),
        &root,
        0x55,
    );
    common::settle_harness(&mut harness);
    let generation = harness.state().host.status.generation;
    let before = common::snapshot(harness.state_mut());
    harness.get_by_label("Save game").click_accesskit();
    common::settle_harness(&mut harness);
    assert!(
        harness.state().file_error.is_none(),
        "{:?}",
        harness.state().file_error
    );
    assert!(harness.state().player.library_editor.is_none());
    assert_eq!(harness.state().host.status.generation, generation);
    assert_eq!(common::snapshot(harness.state_mut()), before);
    let active = harness.state().player.active_game.as_ref().unwrap();
    assert_eq!(active.directory.file_name().unwrap(), "After");
    assert_eq!(active.manifest.id, game.manifest.id);
    assert_eq!(harness.state().player.disks.len(), 2);
    assert_eq!(harness.state().player.mounted[0].as_deref(), Some("disk-1"));
    assert_eq!(
        harness.state().player.disks[1].bytes.as_deref().unwrap(),
        vec![0x55; 901120]
    );
}

#[test]
fn invalid_manifests_and_duplicate_ids_are_reported_without_hiding_valid_games() {
    use gui::game_library::relative_asset;
    let root = root();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Valid".into();
    add_disk(&mut draft, &root, 0x11);
    let game = draft.save(&root).unwrap();
    let valid = game.manifest.clone();
    for (manifest, expected) in [
        (
            {
                let mut m = valid.clone();
                m.title.clear();
                m
            },
            "title",
        ),
        (
            {
                let mut m = valid.clone();
                m.disks[0].id.clear();
                m
            },
            "unique",
        ),
        (
            {
                let mut m = valid.clone();
                m.disks.push(m.disks[0].clone());
                m
            },
            "unique",
        ),
        (
            {
                let mut m = valid.clone();
                m.disks[0].path = "../escape.adf".into();
                m
            },
            "relative path",
        ),
        (
            {
                let mut m = valid.clone();
                m.mounted[1] = Some("unknown".into());
                m
            },
            "Unknown mounted",
        ),
        (
            {
                let mut m = valid.clone();
                m.artwork = Some("../cover.png".into());
                m
            },
            "relative path",
        ),
    ] {
        assert!(manifest.validate().unwrap_err().contains(expected));
    }
    assert!(relative_asset(&root, "").is_err());
    assert!(relative_asset(&root, "disks/disk.adf")
        .unwrap()
        .starts_with(&root));
    let association = debugger::SaveAssociation {
        game_id: Some(valid.id.clone()),
        mounted: [Some("unknown".into()), None, None, None],
    };
    assert!(valid
        .validate_association(&association)
        .unwrap_err()
        .contains("unknown disk"));
    let duplicate = root.join("Duplicate");
    std::fs::create_dir(&duplicate).unwrap();
    std::fs::write(
        duplicate.join("game.json"),
        serde_json::to_vec(&valid).unwrap(),
    )
    .unwrap();
    let malformed = root.join("Malformed");
    std::fs::create_dir(&malformed).unwrap();
    std::fs::write(malformed.join("game.json"), b"{").unwrap();
    let library = GameLibrary::discover(&root).unwrap();
    assert_eq!(library.games.len(), 1);
    assert_eq!(library.games[0].manifest.id, valid.id);
    assert_eq!(library.errors.len(), 2);
    assert!(library
        .errors
        .iter()
        .any(|e| e.contains("Duplicate game ID")));
    assert!(library.errors.iter().any(|e| e.contains("Malformed")));
}

#[test]
fn stale_edit_rejects_external_manifest_and_description_changes_without_writes() {
    let root = root();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Original".into();
    draft.description = "Original description".into();
    let game = draft.save(&root).unwrap();
    for manifest_change in [false, true] {
        let mut edit = GameDraft::edit(game.clone());
        edit.name = "Rename must fail".into();
        edit.description = "Must not overwrite".into();
        let mut current = game.manifest.clone();
        if manifest_change {
            current.configuration = "external-configuration".into();
        }
        let manifest = serde_json::to_vec(&current).unwrap();
        std::fs::write(game.directory.join("game.json"), &manifest).unwrap();
        let description = if manifest_change {
            "Original description"
        } else {
            "External description"
        };
        std::fs::write(
            game.directory.join(current.description.as_ref().unwrap()),
            description,
        )
        .unwrap();
        assert!(edit.save(&root).unwrap_err().contains("changed on disk"));
        assert_eq!(
            std::fs::read(game.directory.join("game.json")).unwrap(),
            manifest
        );
        assert_eq!(
            std::fs::read_to_string(game.directory.join(current.description.unwrap())).unwrap(),
            description
        );
        assert!(!root.join("Rename must fail").exists());
    }
}
