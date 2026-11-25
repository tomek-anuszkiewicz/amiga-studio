#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use egui_kittest::{kittest::Queryable, Harness};
use gui::game_library::{GameLibrary, GameManifest};
use gui::library_edit::{GameDraft, StartupDisk};
use gui::{EmulatorApp, ViewMode};
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

const ADF_BYTES: usize = 880 * 1024;
const SINGLE_7Z: &[u8] = include_bytes!("fixtures/disk-import/single-disk.7z");
const MIXED_7Z: &[u8] = include_bytes!("fixtures/disk-import/mixed-disks.7z");
const ENCRYPTED_7Z: &[u8] = include_bytes!("fixtures/disk-import/encrypted-header.7z");
const ENCRYPTED_ZIP: &[u8] = include_bytes!("fixtures/disk-import/encrypted.zip");

fn adf(fill: u8) -> Vec<u8> {
    vec![fill; ADF_BYTES]
}

fn source(root: &Path, filename: &str, bytes: &[u8]) -> PathBuf {
    let path = root.join(filename);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    path
}

fn zip(members: &[(&str, &[u8])], method: zip::CompressionMethod) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in members {
        writer
            .start_file(
                *name,
                zip::write::SimpleFileOptions::default().compression_method(method),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn draft(name: &str) -> GameDraft {
    let mut draft = GameDraft::new("standard".into());
    draft.name = name.into();
    draft
}

#[test]
fn repeat_during_and_after_create_keeps_one_disk() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    let path = source(root, "Original name.adf", &adf(0x42));
    let renamed = source(root, "Different name.ADF", &adf(0x42));
    let mut draft = draft("Repeated selection");
    assert!(draft.add_disks(std::slice::from_ref(&path)).is_empty());
    let choice = Some(StartupDisk::Pending(draft.imports()[0].key()));
    draft.select_startup_disk(0, choice.clone());
    let repeat = draft.add_disks(&[path.clone(), renamed.clone()]);
    assert_eq!(repeat.len(), 2);
    assert!(repeat
        .iter()
        .all(|issue| issue.contains("Disk already added")));
    assert_eq!(draft.imports().len(), 1);
    assert_eq!(draft.startup_disk(0), choice);
    let saved = draft.save(root, [true; 4]).unwrap();
    assert!(saved.import_issues.is_empty());
    assert_eq!(saved.game.manifest.disks.len(), 1);
    assert_eq!(saved.game.manifest.mounted[0].as_deref(), Some("disk-1"));

    let reopened = GameLibrary::discover(root).unwrap().games.remove(0);
    let mut edit = GameDraft::edit(reopened);
    let repeat = edit.add_disks(&[path, renamed]);
    assert_eq!(repeat.len(), 2);
    assert!(repeat
        .iter()
        .all(|issue| issue.contains("Disk already added")));
    assert!(edit.imports().is_empty());
    let saved_again = edit.save(root, [true; 4]).unwrap();
    assert_eq!(saved_again.game.manifest, saved.game.manifest);
    assert_eq!(
        fs::read_dir(saved_again.game.directory.join("disks"))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        fs::read(saved_again.game.directory.join("disks/Original name.adf")).unwrap(),
        adf(0x42)
    );
}

#[test]
fn mixed_sources_preserve_original_basenames_and_boot_after_relocation() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    let plain = source(root, "External/Original disk.ADF", &adf(0x11));
    let renamed = source(root, "External/Renamed same image.adf", &adf(0x11));
    let archive = zip(
        &[
            ("folder/Another disk.adf", &adf(0x22)),
            ("different/Another disk.adf", &adf(0x77)),
            ("path/../escape.adf", &adf(0x55)),
            ("folder/CON.adf", &adf(0x55)),
            ("folder/notes.txt", b"ignore this"),
        ],
        zip::CompressionMethod::Deflated,
    );
    let zip_path = source(root, "External/Mixed.zip", &archive);
    let seven_path = source(root, "External/Mixed.7z", MIXED_7Z);
    let mut game_draft = draft("Mixed game");
    let issues =
        game_draft.add_disks(&[plain.clone(), renamed, zip_path.clone(), seven_path.clone()]);
    for message in [
        "Disk already added",
        "different/Another disk.adf: Disk filename collision",
        "escape.adf: Invalid archive member path",
        "CON.adf: Invalid ADF filename",
        "bad.adf: Unsupported ADF length",
        "Old copy.ADF: Disk already added",
    ] {
        assert!(
            issues.iter().any(|issue| issue.contains(message)),
            "{message}: {issues:?}"
        );
    }
    let mut names: Vec<_> = game_draft.startup_choices().map(|(_, name)| name).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "Another disk.adf",
            "Archive Fill.adf",
            "Original disk.ADF",
            "Other.adf"
        ]
    );
    let archived = game_draft
        .startup_choices()
        .find(|(_, name)| *name == "Archive Fill.adf")
        .unwrap()
        .0;
    game_draft.select_startup_disk(0, Some(archived));
    assert!(!root.join("Mixed game").exists());

    // Canceling a second draft does not publish a game or extract archive files.
    let mut cancelled = draft("Cancelled");
    assert!(cancelled.add_disks(std::slice::from_ref(&plain)).is_empty());
    drop(cancelled);
    assert!(!root.join("Cancelled").exists());
    assert!(!root.join("External/escape.adf").exists());

    let saved = game_draft.save(root, [true; 4]).unwrap();
    assert!(saved.import_issues.is_empty());
    let game = saved.game;
    assert_eq!(fs::read(&zip_path).unwrap(), archive);
    assert_eq!(fs::read(&seven_path).unwrap(), MIXED_7Z);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(game.directory.join("game.json")).unwrap()).unwrap();
    for (filename, fill) in [
        ("Another disk.adf", 0x22),
        ("Archive Fill.adf", 0x33),
        ("Original disk.ADF", 0x11),
        ("Other.adf", 0x44),
    ] {
        let disk = game
            .manifest
            .disks
            .iter()
            .find(|disk| disk.filename == filename)
            .unwrap();
        assert!(manifest["disks"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!({"id": disk.id, "filename": filename})));
        assert_eq!(
            fs::read(game.directory.join("disks").join(filename)).unwrap(),
            adf(fill)
        );
    }
    let mut other = draft("Other game");
    assert!(other.add_disks(&[plain]).is_empty());
    assert_eq!(
        other
            .save(root, [true; 4])
            .unwrap()
            .game
            .manifest
            .disks
            .len(),
        1
    );
    fs::remove_file(zip_path).unwrap();
    fs::remove_file(seven_path).unwrap();

    let relocated = root.join("Relocated");
    fs::rename(&game.directory, &relocated).unwrap();
    let library = GameLibrary::discover(root).unwrap();
    assert!(library.errors.is_empty());
    let reopened = library
        .games
        .into_iter()
        .find(|entry| entry.directory == relocated)
        .unwrap();
    assert_eq!(reopened.manifest, game.manifest);

    let mut rom = vec![0; 262144];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    source(root, "kick.rom", &rom);
    let mut app = EmulatorApp::default();
    app.player.library_root = Some(root.into());
    app.player.catalog.configurations[0].launch.rom = "kick.rom".into();
    app.start_library_game(reopened, None, true);
    let state = common::snapshot(&mut app);
    assert_eq!(app.player.mounted, game.manifest.mounted);
    assert!(state.floppy.drives[0].disk_inserted);
    assert_eq!(
        state.floppy.drives[0].tracks_match_adf(&adf(0x33)),
        Some(true)
    );
}

#[test]
fn save_rechecks_source_content_and_leaves_late_collisions_untouched() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    let mut game_draft = draft("Changed during edit");
    let first = source(root, "Original.adf", &adf(0x11));
    assert!(game_draft.add_disks(&[first]).is_empty());
    let game = game_draft.save(root, [true; 4]).unwrap().game;
    let mut edit = GameDraft::edit(game.clone());
    assert!(edit
        .add_disks(&[
            source(root, "Late conflict.adf", &adf(0x22)),
            source(root, "Late duplicate.adf", &adf(0x33)),
            source(root, "Valid.adf", &adf(0x44)),
        ])
        .is_empty());
    edit.select_startup_disk(0, Some(StartupDisk::Pending(edit.imports()[1].key())));

    let previous = game.directory.join("disks/Original.adf");
    fs::write(&previous, adf(0x33)).unwrap();
    let collision = source(&game.directory, "disks/Late conflict.adf", &adf(0x55));
    let saved = edit.save(root, [true; 4]).unwrap();
    assert_eq!(saved.import_issues.len(), 2);
    assert!(saved.import_issues[0].contains("Disk filename collision"));
    assert!(saved.import_issues[1].contains("Disk already added"));
    assert_eq!(saved.game.manifest.disks.len(), 2);
    assert_eq!(saved.game.manifest.mounted[0].as_deref(), Some("disk-1"));
    assert_eq!(saved.game.manifest.disks[0], game.manifest.disks[0]);
    assert_eq!(saved.game.manifest.disks[1].filename, "Valid.adf");
    assert_eq!(fs::read(collision).unwrap(), adf(0x55));
    assert_eq!(fs::read(previous).unwrap(), adf(0x33));
    assert!(!game.directory.join("disks/Late duplicate.adf").exists());
}

#[test]
fn saved_conflicts_follow_destination_case_rules() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    let mut game_draft = draft("Case rules");
    assert!(game_draft
        .add_disks(&[
            source(root, "first/Mixed Case.adf", &adf(0x11)),
            source(root, "second/mixed case.ADF", &adf(0x22)),
            source(root, "third/Independent.adf", &adf(0x33)),
        ])
        .is_empty());
    let saved = game_draft.save(root, [true; 4]).unwrap();
    let folder = saved.game.directory.join("disks");
    assert_eq!(fs::read(folder.join("Mixed Case.adf")).unwrap(), adf(0x11));
    assert_eq!(fs::read(folder.join("Independent.adf")).unwrap(), adf(0x33));
    let alternate_case = fs::read(folder.join("mixed case.ADF")).unwrap();
    if alternate_case == adf(0x11) {
        assert_eq!(saved.game.manifest.disks.len(), 2);
        assert_eq!(saved.import_issues.len(), 1);
        assert!(saved.import_issues[0].contains("Disk filename collision"));
    } else {
        assert_eq!(alternate_case, adf(0x22));
        assert_eq!(saved.game.manifest.disks.len(), 3);
        assert!(saved.import_issues.is_empty());
    }
}

#[test]
fn archive_failures_do_not_discard_unrelated_valid_selections_or_write_files() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    let empty = zip(&[], zip::CompressionMethod::Stored);
    let mut corrupt = zip(
        &[("Corrupt.adf", &adf(0x22))],
        zip::CompressionMethod::Stored,
    );
    let data_start = corrupt
        .windows(16)
        .position(|bytes| bytes == [0x22; 16])
        .unwrap();
    corrupt[data_start + 1] ^= 0x80;
    let mut game_draft = draft("Failures");
    let issues = game_draft.add_disks(&[
        source(root, "Empty.zip", &empty),
        source(root, "Invalid.zip", b"not a ZIP file"),
        source(root, "Corrupt.zip", &corrupt),
        source(root, "Encrypted.zip", ENCRYPTED_ZIP),
        source(root, "Encrypted.7z", ENCRYPTED_7Z),
        source(root, "Truncated.7z", &SINGLE_7Z[..SINGLE_7Z.len() - 16]),
        source(root, "Invalid.adf", b"invalid ADF"),
        source(root, "Valid.7z", SINGLE_7Z),
    ]);
    for (filename, problem) in [
        ("Empty.zip", "contains no ADF"),
        ("Invalid.zip", "Invalid ZIP archive"),
        ("Corrupt.zip", "Corrupt archive member"),
        ("Encrypted.zip", "Encrypted ZIP member"),
        ("Encrypted.7z", "Encrypted 7z archive"),
        ("Truncated.7z", "Corrupt 7z archive"),
        ("Invalid.adf", "Unsupported ADF length"),
    ] {
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains(filename) && issue.contains(problem)),
            "{filename}: {issues:?}"
        );
    }
    assert_eq!(
        game_draft
            .startup_choices()
            .map(|(_, name)| name)
            .collect::<Vec<_>>(),
        ["Archive Fill.adf"]
    );
    assert!(!root.join("Failures").exists());
    assert_eq!(fs::read_dir(root).unwrap().count(), 8);
    drop(game_draft);
    assert!(GameLibrary::discover(root).unwrap().games.is_empty());
}

#[test]
fn manifest_rejects_extra_disk_fields_and_path_components() {
    let manifest = serde_json::json!({
        "id": "game", "title": "Game", "configuration": "standard",
        "disks": [{"id": "disk", "filename": "Original.adf"}],
        "mounted": ["disk", null, null, null],
    });
    assert!(serde_json::from_value::<GameManifest>(manifest.clone())
        .unwrap()
        .validate()
        .is_ok());
    let mut legacy = manifest.clone();
    legacy["disks"][0]["path"] = serde_json::json!("Original.adf");
    assert!(serde_json::from_value::<GameManifest>(legacy).is_err());
    for invalid in [
        "../escape.adf",
        "nested/disk.adf",
        "nested\\disk.adf",
        "C:escape.adf",
        "LPT1.adf",
        "CON.adf",
        "trailing.adf ",
        "wrong.txt",
    ] {
        let mut invalid_manifest = manifest.clone();
        invalid_manifest["disks"][0]["filename"] = serde_json::json!(invalid);
        assert!(
            serde_json::from_value::<GameManifest>(invalid_manifest)
                .unwrap()
                .validate()
                .is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn manual_picker_requires_member_choice_and_cancel_retains_the_drive() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    let plain = source(root, "Before.adf", &adf(0x11));
    let zip_file = source(
        root,
        "Choices.zip",
        &zip(
            &[
                ("One/First choice.adf", &adf(0x22)),
                ("Two/Second choice.adf", &adf(0x33)),
            ],
            zip::CompressionMethod::Deflated,
        ),
    );
    let mut app = EmulatorApp::default();
    app.set_view_mode(ViewMode::ScreenOnly);
    app.player.screen = gui::player::PlayerScreen::Setup;
    app.load_setup_disk(0, &plain);
    let first = app.player.setup_mounted[0].clone();
    app.load_setup_disk(0, &zip_file);
    assert_eq!(app.player.setup_mounted[0], first);
    assert_eq!(app.player.setup_disks.len(), 1);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(common::show_full_viewport, app);
    harness.run_steps(3);
    harness.get_by_label("Choose a disk for DF0");
    harness.get_by_label("Cancel").click();
    harness.run_steps(3);
    assert_eq!(harness.state().player.setup_mounted[0], first);
    assert_eq!(harness.state().player.setup_disks.len(), 1);

    harness.state_mut().load_setup_disk(0, &zip_file);
    harness.run_steps(2);
    harness
        .get_by_label(&format!("{} — Two/Second choice.adf", zip_file.display()))
        .click();
    harness.run_steps(3);
    assert_eq!(
        harness.state().player.setup_mounted[0].as_deref(),
        Some("manual-1")
    );
    assert_eq!(
        harness.state().player.setup_disks[1].label,
        "Second choice.adf"
    );
    assert_eq!(
        harness.state().player.setup_disks[1].bytes.as_deref(),
        Some(adf(0x33).as_slice())
    );
}
