#![cfg(not(target_arch = "wasm32"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;

use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness, HarnessBuilder,
};
use gui::game_library::{GameLibrary, GameManifest};
use gui::library_edit::{validate_game_name, GameDraft, StartupDisk};
use gui::player_config::ConfigurationCatalog;
use std::collections::HashMap;
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
    assert!(draft.add_disks(std::slice::from_ref(&source)).is_empty());
    source
}

#[test]
fn startup_choices_follow_pending_disk_ids_across_removal_save_and_normal_boot() {
    let root = root();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Startup choices".into();
    draft.select_startup_disk(0, None);
    add_disk(&mut draft, &root, 0x11);
    let second = add_disk(&mut draft, &root, 0x22);
    let third = add_disk(&mut draft, &root, 0x33);
    let first_key = draft.imports()[0].key();
    let second_choice = Some(StartupDisk::Pending(draft.imports()[1].key()));
    let third_choice = Some(StartupDisk::Pending(draft.imports()[2].key()));
    assert_eq!(draft.startup_disk(0), None, "DF0 was explicitly left empty");
    draft.select_startup_disk(1, Some(StartupDisk::Pending(first_key)));
    draft.select_startup_disk(2, third_choice.clone());
    draft.select_startup_disk(3, second_choice.clone());
    draft.remove_import(first_key);
    assert_eq!(draft.startup_disk(1), None);
    assert_eq!(draft.startup_disk(2), third_choice);
    assert_eq!(draft.startup_disk(3), second_choice);
    assert!(draft
        .save(&root, [true, true, true, false])
        .unwrap_err()
        .contains("DF3 is disconnected"));
    assert!(!root.join(&draft.name).exists());

    let game = draft.save(&root, [true; 4]).unwrap().game;
    let expected_ids = [None, None, Some("disk-2".into()), Some("disk-1".into())];
    assert_eq!(game.manifest.mounted, expected_ids);
    let reopened = GameDraft::edit(GameLibrary::discover(&root).unwrap().games.remove(0));
    assert_eq!(
        std::array::from_fn(|drive| reopened.startup_disk(drive)),
        expected_ids.clone().map(|id| id.map(StartupDisk::Existing))
    );
    assert_eq!(
        reopened
            .startup_choices()
            .map(|(_, name)| name)
            .collect::<Vec<_>>(),
        ["source-34.adf", "source-51.adf"]
    );

    let mut rom = vec![0; 262144];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    let rom_path = root.join("kick.rom");
    std::fs::write(&rom_path, rom).unwrap();
    let mut app = gui::EmulatorApp::default();
    app.player.library_root = Some(root.clone());
    let launch = &mut app.player.catalog.configurations[0].launch;
    launch.rom = "kick.rom".into();
    launch.drives = [true, true, true, false];
    app.start_library_game(game.clone(), None, true);
    common::settle(&mut app, &egui::Context::default());
    assert!(app
        .file_error
        .as_deref()
        .unwrap()
        .contains("DF3 is disconnected"));
    assert!(app.player.active_game.is_none());
    app.player.catalog.configurations[0].launch.drives[3] = true;
    app.start_library_game(game, None, true);
    let state = common::snapshot(&mut app);
    assert_eq!(app.player.mounted, expected_ids);
    let mut expected = debugger::DebuggerSession::new();
    expected.machine.floppy.set_connections([true; 4]).unwrap();
    let expected = &mut expected.machine.floppy.drives;
    assert!(expected[2].insert_disk(&std::fs::read(third).unwrap()));
    assert!(expected[3].insert_disk(&std::fs::read(second).unwrap()));
    for (actual, expected) in state.floppy.drives.iter().zip(expected) {
        assert_eq!(actual.connected, expected.connected);
        assert_eq!(actual.disk_inserted, expected.disk_inserted);
        assert_eq!(
            serde_json::to_value(actual).unwrap()["track_data"],
            serde_json::to_value(expected).unwrap()["track_data"]
        );
    }
}

#[test]
fn create_import_and_rename_preserve_media_ids_saved_progress_and_other_files() {
    let root = root();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "A new game".into();
    draft.description = "A short description".into();
    let first = add_disk(&mut draft, &root, 0x11);
    let second = add_disk(&mut draft, &root, 0x22);
    draft.select_startup_disk(0, Some(StartupDisk::Pending(draft.imports()[0].key())));
    assert!(
        !root.join(&draft.name).exists(),
        "Picking files must not publish a game"
    );
    let mut game = draft.save(&root, [true; 4]).unwrap().game;
    assert_eq!(game.directory.file_name().unwrap(), "A new game");
    assert_eq!(game.description, draft.description);
    assert_eq!(
        game.manifest.mounted,
        [Some("disk-1".into()), None, None, None]
    );
    assert_eq!(
        std::fs::read(
            game.directory
                .join("disks")
                .join(&game.manifest.disks[0].filename)
        )
        .unwrap(),
        std::fs::read(first).unwrap()
    );
    assert_eq!(
        std::fs::read(
            game.directory
                .join("disks")
                .join(&game.manifest.disks[1].filename)
        )
        .unwrap(),
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
    let updated = edit.save(&root, [true; 4]).unwrap().game;
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
        std::fs::read(
            updated
                .directory
                .join("disks")
                .join(&updated.manifest.disks[2].filename)
        )
        .unwrap(),
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
    let game = draft.save(&root, [true; 4]).unwrap().game;
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
        assert!(edit.save(&root, [true; 4]).is_err());
        assert_eq!(
            std::fs::read(game.directory.join("game.json")).unwrap(),
            original
        );
    }
    let bad = root.join("bad.adf");
    std::fs::write(&bad, b"unsupported disk").unwrap();
    assert!(!draft.add_disks(&[bad]).is_empty());
    assert!(draft.imports().is_empty());
    std::fs::create_dir(root.join("Other")).unwrap();
    std::fs::write(root.join("Other/preserve.txt"), b"unrelated").unwrap();
    let mut edit = GameDraft::edit(game.clone());
    edit.name = "Other".into();
    edit.description = "Must not be written".into();
    assert!(edit.save(&root, [true; 4]).is_err());
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
    let game = draft.save(&root, [true; 4]).unwrap().game;
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
    app.player.library_root = Some(root.clone());
    app.navigate_player(gui::player::PlayerScreen::Library);
    let ctx = egui::Context::default();
    let uploaded_color = |output: common::FrameOutput| {
        common::texture_updates(&output.textures_delta)
            .find_map(|(_, delta)| match &delta.image {
                egui::ImageData::Color(image) if image.size == [2, 2] => Some(image.pixels[0]),
                _ => None,
            })
            .expect("The displayed cover must be uploaded")
    };
    let red = uploaded_color(common::wait(&mut app, &ctx, |app| {
        app.player.library.games.len() == 1
    }));
    assert!(red.r() > red.b());
    image::RgbImage::from_pixel(2, 2, image::Rgb([10, 20, 240]))
        .save(&cover)
        .unwrap();
    let second = GameLibrary::discover(&root).unwrap().games.remove(0);
    assert_ne!(first.artwork, second.artwork);
    let blue = uploaded_color(common::wait(&mut app, &ctx, |app| {
        app.player
            .library
            .games
            .first()
            .is_some_and(|game| game.artwork == second.artwork)
    }));
    assert!(
        blue.b() > blue.r(),
        "Refresh retained the obsolete cover texture"
    );
    std::fs::rename(&cover, game.directory.join("saved-cover.jpg")).unwrap();
    image.save(game.directory.join("cover.png")).unwrap();
    let png = GameLibrary::discover(&root).unwrap().games.remove(0);
    assert!(image::load_from_memory(png.artwork.as_ref().unwrap()).is_ok());
}

fn artwork_harness(root: &Path, width: f32) -> Harness<'static, gui::EmulatorApp> {
    artwork_harness_with(
        root,
        width,
        Harness::builder().renderer(common::TextureUploads),
    )
}

fn artwork_harness_with(
    root: &Path,
    width: f32,
    builder: HarnessBuilder<gui::EmulatorApp>,
) -> Harness<'static, gui::EmulatorApp> {
    let mut app = gui::EmulatorApp::default();
    app.player.library_root = Some(root.to_path_buf());
    app.navigate_player(gui::player::PlayerScreen::Library);
    let mut harness = builder
        .with_size(egui::vec2(width, 900.0))
        .build_ui_state(common::show_full_viewport, app);
    let timeout = std::time::Duration::from_secs(10);
    let start = std::time::Instant::now();
    loop {
        common::settle_harness(&mut harness);
        if harness
            .query_by_label("Add game")
            .is_some_and(|node| !node.accesskit_node().is_disabled())
        {
            return harness;
        }
        assert!(start.elapsed() < timeout, "Library did not become writable");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn assert_uploaded_artwork_preview(output: &egui::FullOutput, expected: &image::RgbaImage) {
    let size = [expected.width() as usize, expected.height() as usize];
    let updates = common::TextureUploads::take();
    let (id, uploaded) = updates
        .iter()
        .find_map(|(id, delta)| {
            let egui::ImageData::Color(image) = &delta.image;
            (image.size == size).then_some((*id, image))
        })
        .expect("The chosen image must be uploaded");
    let actual: Vec<_> = uploaded
        .pixels
        .iter()
        .flat_map(egui::Color32::to_srgba_unmultiplied)
        .collect();
    assert_eq!(&actual, expected.as_raw());
    let bounds = output
        .shapes
        .iter()
        .find_map(|shape| {
            // With no rotation, egui draws the image as a textured rectangle.
            let egui::Shape::Rect(rect) = &shape.shape else {
                return None;
            };
            let brush = rect.brush.as_ref()?;
            (brush.fill_texture_id == id
                && rect.fill.a() > 0
                && shape.clip_rect.intersect(rect.rect).is_positive())
            .then_some(rect.rect)
        })
        .expect("The chosen image texture must actually be drawn");
    let expected_aspect = expected.width() as f32 / expected.height() as f32;
    assert!((bounds.width() / bounds.height() - expected_aspect).abs() < 0.03);
}

const COVER_TEST_GAME: &str = "Patterned game";
const COVER_TEST_SIZE: [usize; 2] = [120, 60];

#[derive(Default)]
struct DrawnCover {
    textures: HashMap<egui::TextureId, egui::ColorImage>,
}

impl egui_kittest::TestRenderer for DrawnCover {
    fn handle_delta(&mut self, delta: &mut egui::TexturesDelta) {
        common::TextureUploads::capture(delta);
        for (id, update) in common::texture_updates(delta) {
            let egui::ImageData::Color(image) = &update.image;
            if update.pos.is_none() && image.size == COVER_TEST_SIZE {
                self.textures.insert(*id, image.as_ref().clone());
            }
        }
        for id in &delta.free {
            self.textures.remove(id);
        }
        delta.clear();
    }

    fn render(
        &mut self,
        _ctx: &egui::Context,
        output: &egui::FullOutput,
    ) -> Result<image::RgbaImage, String> {
        let title = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == COVER_TEST_GAME => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                _ => None,
            })
            .ok_or("The Library did not draw the edited game's tile")?;
        let image = tile_rect(output, title, |rect| {
            rect.fill.a() > 0 && rect.brush.is_some()
        })
        .and_then(|rect| self.textures.get(&rect.brush.as_ref()?.fill_texture_id))
        .ok_or("The Library did not draw a cover above the edited game's title")?;
        let bytes = image
            .pixels
            .iter()
            .flat_map(egui::Color32::to_srgba_unmultiplied)
            .collect();
        image::RgbaImage::from_raw(image.size[0] as u32, image.size[1] as u32, bytes)
            .ok_or_else(|| "Unexpected cover size".into())
    }
}

#[test]
fn saved_paste_and_file_covers_are_drawn_on_the_first_return_to_library() {
    use egui::accesskit::Role;
    use gui::window_keyboard::receive_artwork_paste;

    let root = root();
    let [width, height] = COVER_TEST_SIZE.map(|side| side as u32);
    let original = image::RgbaImage::from_fn(width, height, |x, y| {
        image::Rgba([x as u8, y as u8, 40, 255])
    });
    let pasted = image::RgbaImage::from_fn(width, height, |x, y| {
        image::Rgba([160, x as u8, y as u8, 255])
    });
    let chosen = image::RgbaImage::from_fn(width, height, |x, y| {
        image::Rgba([y as u8, 240, x as u8, 255])
    });
    let source = root.join("replacement.png");
    chosen.save(&source).unwrap();
    let mut draft = GameDraft::new("standard".into());
    draft.name = COVER_TEST_GAME.into();
    let game = draft.save(&root, [true; 4]).unwrap().game;
    original.save(game.directory.join("cover.png")).unwrap();

    let mut h = artwork_harness_with(
        &root,
        1024.0,
        Harness::builder().renderer(DrawnCover::default()),
    );
    assert!(h
        .get_by_role_and_label(Role::Button, COVER_TEST_GAME)
        .rect()
        .is_positive());
    assert_eq!(h.render().unwrap(), original);
    h.get_by_role_and_label(Role::Button, COVER_TEST_GAME)
        .click_accesskit();
    common::settle_harness(&mut h);

    for (expected, source) in [(&pasted, None), (&chosen, Some(source.as_path()))] {
        h.get_by_label("Edit game").click_accesskit();
        common::settle_harness(&mut h);
        if let Some(source) = source {
            // Supply the path the native file picker would return.
            h.state_mut()
                .player
                .library_editor
                .as_mut()
                .unwrap()
                .choose_artwork(source)
                .unwrap();
        } else {
            receive_artwork_paste(&h.ctx, || Ok(Some(pasted.clone())));
        }
        h.step();
        assert_uploaded_artwork_preview(h.output(), expected);

        h.get_by_label("Save game").click_accesskit();
        h.step();
        assert!(h.state().file_error.is_none());
        assert!(h.state().player.library_editor.is_none());
        let entry = &h.state().player.library.games[0];
        assert_eq!(entry.manifest.id, game.manifest.id);
        assert_eq!(
            &image::load_from_memory(entry.artwork.as_deref().unwrap())
                .unwrap()
                .into_rgba8(),
            expected,
            "The Save game action must update the in-memory entry immediately"
        );
        // Save can finish while egui is still drawing the editor. Observe the
        // first pass with the Library tile, including if it drew on the click.
        if h.query_by_role_and_label(Role::Button, COVER_TEST_GAME)
            .is_none()
        {
            h.step();
        }
        assert!(h
            .get_by_role_and_label(Role::Button, COVER_TEST_GAME)
            .rect()
            .is_positive());
        assert_eq!(&h.render().unwrap(), expected);

        let manifest: GameManifest =
            serde_json::from_slice(&std::fs::read(game.directory.join("game.json")).unwrap())
                .unwrap();
        let asset = game.directory.join(manifest.artwork.unwrap());
        assert_eq!(&image::open(asset).unwrap().into_rgba8(), expected);
    }
}

#[test]
fn selected_jpeg_previews_and_saves_after_source_deletion_and_rejecting_an_invalid_image() {
    let root = root();
    let source = root.join("source.jpg");
    image::RgbImage::from_fn(180, 90, |x, y| image::Rgb([x as u8, y as u8, 100]))
        .save(&source)
        .unwrap();
    let expected = image::open(&source).unwrap().into_rgba8();
    let invalid = root.join("invalid.png");
    std::fs::write(&invalid, b"not an image").unwrap();
    let mut h = artwork_harness(&root, 1024.0);
    h.get_by_label("Add game").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.get_by_label("Choose image...").rect().is_positive());
    let draft = h.state_mut().player.library_editor.as_mut().unwrap();
    draft.name = "Owned artwork".into();
    // Supply the path the native file picker would return.
    draft.choose_artwork(&source).unwrap();
    assert!(!root.join("Owned artwork").exists());
    std::fs::remove_file(source).unwrap();
    h.step();
    assert_uploaded_artwork_preview(h.output(), &expected);
    let draft = h.state_mut().player.library_editor.as_mut().unwrap();
    assert!(draft
        .choose_artwork(&invalid)
        .unwrap_err()
        .contains("JPEG or PNG"));
    h.get_by_label("Save game").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.state().file_error.is_none());
    let reopened = GameLibrary::discover(&root).unwrap();
    let game = &reopened.games[0];
    assert_eq!(game.manifest.title, "Owned artwork");
    let asset = game
        .directory
        .join(game.manifest.artwork.as_deref().unwrap());
    assert_eq!(image::open(asset).unwrap().into_rgba8(), expected);
}

#[test]
fn artwork_paste_routes_only_to_the_active_editor_and_keeps_text_paste_and_cancel() {
    use gui::window_keyboard::receive_artwork_paste;

    let root = root();
    let mut h = artwork_harness(&root, 640.0);
    receive_artwork_paste(&h.ctx, || {
        panic!("The library list must not read image paste")
    });
    h.get_by_label("Add game").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h
        .get_all_by_value(if cfg!(target_os = "macos") {
            "Paste an image with Command+V."
        } else {
            "Paste an image with Ctrl+V."
        })
        .next()
        .is_some());
    h.state_mut().player.library_editor.as_mut().unwrap().name = "Pasted artwork".into();
    let expected =
        image::RgbaImage::from_fn(80, 160, |x, y| image::Rgba([50, x as u8, y as u8, 255]));
    assert!(!gui::keyboard_input::receive_key(
        &h.ctx,
        common::keyboard::key("KeyV", true),
        None
    ));
    receive_artwork_paste(&h.ctx, || Ok(Some(expected.clone())));
    h.step();
    assert_uploaded_artwork_preview(h.output(), &expected);
    assert!(!root.join("Pasted artwork").exists());

    receive_artwork_paste(&h.ctx, || Ok(Some(image::RgbaImage::new(0, 9))));
    h.step();
    assert!(h
        .get_all_by_value("The image has no pixels")
        .next()
        .is_some());
    h.get_by_role(egui::accesskit::Role::MultilineTextInput)
        .focus();
    h.step();
    // The window can receive both an image and text for a single clipboard action.
    receive_artwork_paste(&h.ctx, || {
        Ok(Some(image::RgbaImage::from_pixel(
            2,
            3,
            image::Rgba([240, 31, 15, 255]),
        )))
    });
    h.input_mut()
        .events
        .push(egui::Event::Paste("Pasted description".into()));
    h.step();
    let draft = h.state().player.library_editor.as_ref().unwrap();
    assert_eq!(draft.description, "Pasted description");
    h.get_by_label("Save game").click_accesskit();
    common::settle_harness(&mut h);
    let reopened = GameLibrary::discover(&root).unwrap();
    let game = &reopened.games[0];
    let asset = game
        .directory
        .join(game.manifest.artwork.as_deref().unwrap());
    let file_bytes = std::fs::read(&asset).unwrap();
    assert_eq!(image::open(&asset).unwrap().into_rgba8(), expected);
    let mut filenames: Vec<_> = std::fs::read_dir(&game.directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    filenames.sort();
    let manifest_path = game.directory.join("game.json");
    let manifest_bytes = std::fs::read(&manifest_path).unwrap();

    h.get_by_label("Edit game").click_accesskit();
    common::settle_harness(&mut h);
    let replacement = image::RgbaImage::from_pixel(2, 3, image::Rgba([240, 31, 15, 255]));
    receive_artwork_paste(&h.ctx, || Ok(Some(replacement.clone())));
    h.step();
    assert_uploaded_artwork_preview(h.output(), &replacement);
    h.get_by_label("Cancel").click_accesskit();
    common::settle_harness(&mut h);
    assert_eq!(std::fs::read(asset).unwrap(), file_bytes);
    assert_eq!(std::fs::read(manifest_path).unwrap(), manifest_bytes);
    let mut after: Vec<_> = std::fs::read_dir(&game.directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    after.sort();
    assert_eq!(after, filenames);
    h.get_by_label("Manual setup").click_accesskit();
    common::settle_harness(&mut h);
    receive_artwork_paste(&h.ctx, || panic!("Manual setup must not read image paste"));
}

fn cover_tiles_fixture(root: &Path) -> GameManifest {
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Artwork".into();
    let game = draft.save(root, [true; 4]).unwrap().game;
    image::RgbImage::from_fn(180, 90, |x, y| {
        if (x / 18 + y / 18).is_multiple_of(2) {
            image::Rgb([42, 120, 171])
        } else {
            image::Rgb([203, 161, 80])
        }
    })
    .save(game.directory.join("cover.png"))
    .unwrap();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Missing cover".into();
    draft.save(root, [true; 4]).unwrap();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Corrupt cover".into();
    let game = draft.save(root, [true; 4]).unwrap().game;
    let mut manifest = game.manifest;
    manifest.artwork = Some("cover.jpg".into());
    std::fs::write(
        game.directory.join("game.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(game.directory.join("cover.jpg"), b"corrupt contents").unwrap();
    image::RgbImage::from_pixel(2, 2, image::Rgb([100, 20, 251]))
        .save(game.directory.join("cover.png"))
        .unwrap();
    let mut rom = vec![0; 262144];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    std::fs::write(root.join("kick.rom"), rom).unwrap();
    let mut catalog = ConfigurationCatalog::default();
    catalog.configurations[0].launch.rom = "kick.rom".into();
    std::fs::write(
        root.join("configurations.json"),
        serde_json::json!({
            "configurations": catalog.configurations,
            "mappings": catalog.mappings,
        })
        .to_string(),
    )
    .unwrap();
    manifest
}

fn tile_rect(
    output: &egui::FullOutput,
    title: egui::Rect,
    matches: impl Fn(&egui::epaint::RectShape) -> bool,
) -> Option<&egui::epaint::RectShape> {
    let rects = output.shapes.iter().filter_map(|shape| {
        let egui::Shape::Rect(rect) = &shape.shape else {
            return None;
        };
        (shape.clip_rect.contains_rect(rect.rect)
            && rect.rect.bottom() <= title.top()
            && rect.rect.x_range().intersects(title.x_range())
            && matches(rect))
        .then_some(rect)
    });
    rects.max_by(|a, b| a.rect.bottom().total_cmp(&b.rect.bottom()))
}

fn empty_cover(output: &egui::FullOutput, title: egui::Rect, stroke: egui::Stroke) -> egui::Rect {
    let outline = tile_rect(output, title, |rect| !rect.stroke.is_empty())
        .expect("The tile must draw an outline above the title");
    assert_eq!(outline.stroke, stroke);
    assert_eq!(outline.fill, egui::Color32::TRANSPARENT);
    assert!(outline.brush.is_none());
    let interior = outline.rect.shrink(2.0);
    assert!(!output.shapes.iter().any(|shape| {
        if matches!(&shape.shape, egui::Shape::Rect(rect) if std::ptr::eq(rect, outline)) {
            return false;
        }
        let bounds = shape
            .shape
            .visual_bounding_rect()
            .intersect(shape.clip_rect);
        let is_background = bounds.contains_rect(outline.rect) && bounds != outline.rect;
        bounds.intersect(interior).is_positive() && !is_background
    }));
    outline.rect
}

#[test]
fn missing_and_unreadable_covers_reserve_a_blank_frame_and_allow_launch() {
    let root = root();
    let manifest = cover_tiles_fixture(&root);
    let mut h = artwork_harness(&root, 1024.0);
    let spacing = h.ctx.global_style().spacing.item_spacing.y;
    let stroke = h
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    for width in [1024.0, 250.0, 104.0] {
        h.set_size(egui::vec2(width, 900.0));
        common::settle_harness(&mut h);
        let artwork_title = h.get_by_label("Artwork").rect();
        let corrupt_title = h.get_by_label("Corrupt cover").rect();
        let missing_title = h.get_by_label("Missing cover").rect();
        let output = h.output();
        assert!(h.query_all_by_value("No cover image").next().is_none());
        let corrupt = empty_cover(output, corrupt_title, stroke);
        let missing = empty_cover(output, missing_title, stroke);
        let side = corrupt.width();
        assert_eq!(corrupt.size(), egui::Vec2::splat(side));
        assert_eq!(missing.size(), corrupt.size());
        assert!((corrupt_title.top() - corrupt.bottom() - spacing).abs() < 1.0);
        assert!((missing_title.top() - missing.bottom() - spacing).abs() < 1.0);
        let artwork = tile_rect(output, artwork_title, |rect| rect.brush.is_some())
            .expect("The tile must draw artwork above the title")
            .rect;
        assert!((artwork.width() / artwork.height() - 2.0).abs() < 0.03);
        assert!((artwork.width() - side).abs() < 1.0);
        let artwork_slot = egui::Rect::from_min_size(
            egui::pos2(artwork.left(), artwork_title.top() - spacing - side),
            egui::Vec2::splat(side),
        );
        assert!(artwork_slot.expand(1.0).contains_rect(artwork));
        for rect in [corrupt, missing, artwork_slot] {
            assert!(h.ctx.content_rect().contains_rect(rect));
        }
        if width == 1024.0 {
            assert_eq!(side, 110.0);
            assert_eq!(artwork_title.top(), corrupt_title.top());
            assert_eq!(corrupt_title.top(), missing_title.top());
        } else if width == 250.0 {
            assert_eq!(side, 110.0);
            assert_eq!(artwork_title.top(), corrupt_title.top());
            assert!(missing.top() > corrupt_title.bottom());
        } else {
            assert!(side > 0.0 && side < 110.0);
            assert!(corrupt.top() > artwork_title.bottom());
            assert!(missing.top() > corrupt_title.bottom());
        }
    }
    h.set_size(egui::vec2(1024.0, 900.0));
    common::settle_harness(&mut h);
    h.get_by_label("Corrupt cover").click_accesskit();
    common::settle_harness(&mut h);
    h.get_by_label("Start game").click_accesskit();
    common::wait_harness(&mut h, |app| app.player.active_game.is_some());
    assert!(h.state().file_error.is_none());
    assert_eq!(
        h.state().player.active_game.as_ref().unwrap().manifest.id,
        manifest.id
    );
}

fn startup_dropdown<'a>(
    h: &'a Harness<'_, gui::EmulatorApp>,
    drive: &str,
) -> egui_kittest::Node<'a> {
    let row = h
        .get_all_by_value(drive)
        .find(|node| node.accesskit_node().role() == egui::accesskit::Role::Label)
        .unwrap()
        .accesskit_node()
        .bounding_box()
        .unwrap();
    h.get_all_by_role(egui::accesskit::Role::ComboBox)
        .find(|node| {
            node.accesskit_node()
                .bounding_box()
                .is_some_and(|b| b.x0 >= row.x1 && b.y0 < row.y1 && b.y1 > row.y0)
        })
        .unwrap()
}

fn select_startup(h: &mut Harness<'_, gui::EmulatorApp>, drive: &str, disk: &str) {
    startup_dropdown(h, drive).click();
    common::settle_harness(h);
    h.get_by_role_and_label(egui::accesskit::Role::Button, disk)
        .click();
    common::settle_harness(h);
}

fn assert_startup_choice(h: &Harness<'_, gui::EmulatorApp>, drive: &str, expected: &str) {
    let dropdown = startup_dropdown(h, drive).rect();
    // egui paints the selected text without exposing it on the combo box node.
    assert!(
        h.output().shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Text(text)
                if text.galley.text() == expected
                    && dropdown.contains(text.pos)
                    && shape.clip_rect.contains(text.pos)
        )),
        "{drive} dropdown should display {expected:?}"
    );
}

#[test]
fn startup_editor_clears_removed_import_and_rejects_disconnected_drive() {
    let root = root();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Startup choices".into();
    add_disk(&mut draft, &root, 0x44);
    add_disk(&mut draft, &root, 0x66);
    for drive in 0..2 {
        draft.select_startup_disk(
            drive,
            Some(StartupDisk::Pending(draft.imports()[drive].key())),
        );
    }
    let connected = [true, true, false, false];
    let game = draft.save(&root, connected).unwrap().game;
    let manifest = game.directory.join("game.json");
    let before = std::fs::read(&manifest).unwrap();
    let first_disk_id = &game.manifest.disks[0].id;
    let second_disk_id = &game.manifest.disks[1].id;

    let mut app = gui::EmulatorApp::default();
    app.player.catalog.configurations[0].launch.drives = connected;
    app.player.library_root = Some(root.clone());
    app.player.selected_game = Some(game.manifest.id.clone());
    app.navigate_player(gui::player::PlayerScreen::Library);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1024.0, 800.0))
        .build_ui_state(|ui, app: &mut gui::EmulatorApp| app.update_ui(ui), app);
    common::wait_harness(&mut h, |app| app.player.library.games.len() == 1);
    h.get_by_label("Edit game").click_accesskit();
    common::settle_harness(&mut h);
    let draft = h.state_mut().player.library_editor.as_mut().unwrap();
    add_disk(draft, &root, 0x55);
    common::settle_harness(&mut h);
    select_startup(&mut h, "DF0", "source-85.adf");
    assert_startup_choice(&h, "DF0", "source-85.adf");
    h.get_by_label("Remove from draft").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.query_by_label("Remove from draft").is_none());
    assert_startup_choice(&h, "DF0", "Empty");
    assert_startup_choice(&h, "DF1", "source-102.adf");

    select_startup(&mut h, "DF3", "source-68.adf");
    assert_startup_choice(&h, "DF3", "source-68.adf");
    h.get_by_label("Save game").click_accesskit();
    common::settle_harness(&mut h);
    let error = h.state().file_error.as_deref().unwrap();
    assert!(error.contains("DF3 is disconnected"), "{error}");
    assert!(h.get_all_by_value(error).next().is_some());
    assert_eq!(std::fs::read(&manifest).unwrap(), before);
    assert_eq!(
        h.state().player.selected_game.as_ref(),
        Some(&game.manifest.id)
    );
    let draft = h.state().player.library_editor.as_ref().unwrap();
    assert_eq!(
        std::array::from_fn(|drive| draft.startup_disk(drive)),
        [
            None,
            Some(StartupDisk::Existing(second_disk_id.clone())),
            None,
            Some(StartupDisk::Existing(first_disk_id.clone())),
        ]
    );

    h.get_by_label("Close").click_accesskit();
    common::settle_harness(&mut h);
    select_startup(&mut h, "DF3", "Empty");
    h.get_by_label("Save game").click_accesskit();
    common::settle_harness(&mut h);
    assert!(h.state().player.library_editor.is_none());
    let saved = GameLibrary::discover(&root).unwrap().games.remove(0);
    assert_eq!(saved.manifest.disks, game.manifest.disks);
    assert_eq!(
        saved.manifest.mounted,
        [None, Some(second_disk_id.clone()), None, None]
    );
    h.get_by_label("Edit game").click_accesskit();
    common::settle_harness(&mut h);
    assert_startup_choice(&h, "DF0", "Empty");
    assert_startup_choice(&h, "DF1", "source-102.adf");
    assert_startup_choice(&h, "DF3", "Empty");
}

#[test]
fn editor_cancel_does_not_write_and_save_updates_the_active_games_path_without_restarting() {
    let root = root();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Before".into();
    add_disk(&mut draft, &root, 0x44);
    draft.select_startup_disk(0, Some(StartupDisk::Pending(draft.imports()[0].key())));
    let game = draft.save(&root, [true; 4]).unwrap().game;
    let rom_path = root.join("kick.rom");
    let mut rom = vec![0; 262144];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    std::fs::write(&rom_path, rom).unwrap();
    let mut app = gui::EmulatorApp::default();
    app.player.library_root = Some(root.clone());
    app.player.catalog.configurations[0].launch.rom = "kick.rom".into();
    app.player.catalog.configurations[0].launch.drives[1] = true;
    app.start_library_game(game.clone(), None, false);
    common::settle(&mut app, &egui::Context::default());
    app.player.selected_game = Some(game.manifest.id.clone());
    app.navigate_player(gui::player::PlayerScreen::Library);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 800.0))
        .build_ui_state(|ui, app: &mut gui::EmulatorApp| app.update_ui(ui), app);
    common::wait_harness(&mut harness, |app| app.player.library.games.len() == 1);
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
    let manifest_path = game.directory.join("game.json");
    let before_cancel = std::fs::read(&manifest_path).unwrap();
    let before_media = common::snapshot(harness.state_mut());
    harness.get_by_label("Edit game").click_accesskit();
    common::settle_harness(&mut harness);
    select_startup(&mut harness, "DF0", "Empty");
    assert_eq!(
        harness
            .state()
            .player
            .library_editor
            .as_ref()
            .unwrap()
            .startup_disk(0),
        None
    );
    harness.get_by_label("Cancel").click_accesskit();
    common::settle_harness(&mut harness);
    assert_eq!(std::fs::read(manifest_path).unwrap(), before_cancel);
    assert_eq!(common::snapshot(harness.state_mut()), before_media);

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
    select_startup(&mut harness, "DF0", "Empty");
    select_startup(&mut harness, "DF1", "source-85.adf");
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
    let startup = [None, Some("disk-2".into()), None, None];
    assert_eq!(active.manifest.mounted, startup);
    assert_eq!(harness.state().player.disks.len(), 2);
    assert_eq!(harness.state().player.mounted[0].as_deref(), Some("disk-1"));
    assert_eq!(
        harness.state().player.disks[1].bytes.as_deref().unwrap(),
        vec![0x55; 901120]
    );

    let other_root = tempfile::tempdir().unwrap();
    let mut other = GameDraft::new("standard".into());
    other.name = "Another game".into();
    add_disk(&mut other, other_root.path(), 0x66);
    other.select_startup_disk(0, Some(StartupDisk::Pending(other.imports()[0].key())));
    let other = other.save(other_root.path(), [true; 4]).unwrap().game;
    assert_eq!(other.manifest.id, game.manifest.id);
    common::select_library(&mut harness, other_root.path());
    common::wait_harness(&mut harness, |app| {
        app.player
            .library
            .games
            .first()
            .is_some_and(|game| game.manifest.title == "Another game")
    });
    harness
        .state_mut()
        .navigate_player(gui::player::PlayerScreen::Session);
    common::settle_harness(&mut harness);
    harness.get_by_label("Reset").click_accesskit();
    common::wait_harness(&mut harness, |app| app.host.status.generation > generation);
    assert_eq!(harness.state().player.mounted, startup);
    assert!(harness.state().host.status.running);
    let state = common::snapshot(harness.state_mut());
    assert!(!state.floppy.drives[0].disk_inserted);
    let mut expected = debugger::DebuggerSession::new();
    assert!(expected.machine.floppy.drives[1].insert_disk(&vec![0x55; 901120]));
    assert_eq!(
        serde_json::to_value(&state.floppy.drives[1]).unwrap()["track_data"],
        serde_json::to_value(&expected.machine.floppy.drives[1]).unwrap()["track_data"]
    );
}

#[test]
fn invalid_manifests_and_duplicate_ids_are_reported_without_hiding_valid_games() {
    use gui::game_library::relative_asset;
    let root = root();
    let mut draft = GameDraft::new("standard".into());
    draft.name = "Valid".into();
    add_disk(&mut draft, &root, 0x11);
    let game = draft.save(&root, [true; 4]).unwrap().game;
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
                m.disks[0].filename = "../escape.adf".into();
                m
            },
            "Invalid ADF filename",
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
    let game = draft.save(&root, [true; 4]).unwrap().game;
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
        assert!(edit
            .save(&root, [true; 4])
            .unwrap_err()
            .contains("changed on disk"));
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
