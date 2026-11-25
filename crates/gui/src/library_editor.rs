//! Desktop Add game / Edit game form, separate from machine configuration.

use crate::library_edit::{validate_game_name, GameDraft};
use crate::EmulatorApp;
use std::borrow::Cow;

impl EmulatorApp {
    pub(crate) fn open_library_editor(&mut self, game: Option<crate::game_library::LibraryGame>) {
        self.file_error = None;
        self.player.library_editor = Some(game.map_or_else(
            || GameDraft::new(self.player.catalog.selected.clone()),
            GameDraft::edit,
        ));
    }

    pub(crate) fn render_library_editor(&mut self, ui: &mut egui::Ui, writable: bool) {
        let Some(mut draft) = self.player.library_editor.take() else {
            return;
        };
        ui.heading(if draft.original.is_some() {
            "Edit game"
        } else {
            "Add game"
        });
        let name_id = ui.make_persistent_id("game_name");
        let description_id = ui.make_persistent_id("game_description");
        let label = ui.label("Game name");
        let response = ui.add(
            egui::TextEdit::singleline(&mut draft.name)
                .id(name_id)
                .hint_text("Game name")
                .desired_width(400.0),
        );
        response
            .labelled_by(label.id)
            .on_hover_text("The game name also determines its directory inside the library");
        ui.label(format!(
            "Folder: {}",
            self.player
                .library_root
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new(""))
                .join(&draft.name)
                .display()
        ));
        ui.small(
            "The name is also the folder name. Renaming keeps existing saves and disk assignments.",
        );
        if !draft.name.is_empty() {
            if let Err(error) = validate_game_name(&draft.name) {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        }
        ui.add_space(8.0);
        let label = ui.label("Description (optional)");
        let response = ui.add(
            egui::TextEdit::multiline(&mut draft.description)
                .id(description_id)
                .desired_rows(4)
                .desired_width(f32::INFINITY),
        );
        response
            .labelled_by(label.id)
            .on_hover_text("Optional notes shown when selecting this game in the library");
        let selected = self
            .player
            .catalog
            .configurations
            .iter()
            .find(|c| c.id == draft.configuration)
            .map_or("Select configuration", |c| c.name.as_str());
        egui::ComboBox::from_label("Configuration")
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for configuration in &self.player.catalog.configurations {
                    ui.selectable_value(
                        &mut draft.configuration,
                        configuration.id.clone(),
                        &configuration.name,
                    );
                }
            });
        ui.separator();
        render_artwork(ui, &mut draft, [name_id, description_id]);
        ui.separator();
        ui.strong("Disks");
        if let Some(game) = &draft.original {
            for (index, disk) in game.manifest.disks.iter().enumerate() {
                ui.label(format!("{}. {}", index + 1, disk.filename.as_str()));
            }
        }
        let mut remove = None;
        for disk in draft.imports() {
            ui.horizontal(|ui| {
                ui.label(format!("To copy: {}", disk.filename()))
                    .on_hover_text(disk.source());
                if ui
                    .button("Remove from draft")
                    .on_hover_text(
                        "Do not copy this selected disk; its source file stays untouched",
                    )
                    .clicked()
                {
                    remove = Some(disk.key());
                }
            });
        }
        if let Some(key) = remove {
            draft.remove_import(key);
        }
        if ui.button("Add disks...").on_hover_text("Select ADF, ZIP or 7z files. The app copies the chosen ADFs when you save the game.").clicked() {
            if let Some(paths) = self.disk_file_dialog("Add disks").pick_files() {
                if let Some(path) = paths.first() {
                    self.remember_disk_selection(path);
                }
                let issues = draft.add_disks(&paths);
                self.file_error = (!issues.is_empty()).then(|| issues.join("\n"));
            }
        }
        let connected = self
            .player
            .catalog
            .configurations
            .iter()
            .find(|configuration| configuration.id == draft.configuration)
            .map(|configuration| configuration.launch.drives);
        ui.separator();
        ui.strong("Startup disks");
        for drive in 0..config::FLOPPY_DRIVE_COUNT {
            ui.horizontal(|ui| {
                let label = ui.label(format!("DF{drive}"));
                let selected = draft.startup_disk(drive);
                let selected_label = selected.as_ref().map_or("Empty", |disk| {
                    draft
                        .startup_choices()
                        .find_map(|(choice, name)| (choice == *disk).then_some(name))
                        .unwrap_or("Unavailable disk")
                });
                let mut change = None;
                egui::ComboBox::from_id_salt(("startup_disk", drive))
                    .selected_text(selected_label)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(selected.is_none(), "Empty").clicked() {
                            change = Some(None);
                        }
                        for (disk, name) in draft.startup_choices() {
                            if ui
                                .selectable_label(selected.as_ref() == Some(&disk), name)
                                .clicked()
                            {
                                change = Some(Some(disk));
                            }
                        }
                    })
                    .response
                    .labelled_by(label.id)
                    .on_hover_text("Choose the disk to insert on normal boot or player Reset");
                if let Some(choice) = change {
                    draft.select_startup_disk(drive, choice);
                }
                if connected.is_some_and(|drives| !drives[drive]) {
                    ui.weak("Disconnected");
                }
            });
        }
        if let Some(error) = &self.file_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        ui.separator();
        let mut save = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            save = ui
                .add_enabled(
                    writable && connected.is_some() && validate_game_name(&draft.name).is_ok(),
                    egui::Button::new("Save game"),
                )
                .on_hover_text(
                    "Create or update this game's files; selected disk sources are preserved",
                )
                .clicked();
            cancel = ui
                .button("Cancel")
                .on_hover_text("Discard this draft without changing game files")
                .clicked();
        });
        if cancel {
            self.file_error = None;
            return;
        }
        if let Some((connected, sync)) = connected
            .zip(self.player.library_sync.as_ref())
            .filter(|_| save)
        {
            let edits_active = self
                .player
                .active_game
                .as_ref()
                .zip(draft.original.as_ref())
                .is_some_and(|(active, original)| active.same_entry(original));
            match draft.save(&sync.root, connected) {
                Ok(saved) => {
                    if let Some(original) = &draft.original {
                        self.relocate_start_state_deletions(original, &saved.game);
                    }
                    self.complete_library_edit(saved.game, edits_active, ui.ctx());
                    self.file_error =
                        (!saved.import_issues.is_empty()).then(|| saved.import_issues.join("\n"));
                    return;
                }
                Err(error) => self.file_error = Some(error),
            }
        }
        self.player.library_editor = Some(draft);
    }

    fn complete_library_edit(
        &mut self,
        game: crate::game_library::LibraryGame,
        edits_active: bool,
        ctx: &egui::Context,
    ) {
        self.file_error = None;
        if edits_active {
            // The machine retains its mounted bytes. Newly imported disks become selectable.
            for disk in &game.manifest.disks {
                if !self
                    .player
                    .disks
                    .iter()
                    .any(|existing| existing.id == disk.id)
                {
                    self.player.disks.push(crate::player::SessionDisk {
                        id: disk.id.clone(),
                        label: disk.filename.clone(),
                        bytes: disk
                            .asset_path(&game.directory)
                            .ok()
                            .and_then(|path| std::fs::read(path).ok()),
                    });
                }
            }
            let renamed = self
                .player
                .active_game
                .as_ref()
                .is_some_and(|active| active.manifest.title != game.manifest.title);
            self.player.active_game = Some(game.clone());
            if renamed {
                self.sync_session_title(ctx);
            }
        }
        self.player
            .catalog
            .selected
            .clone_from(&game.manifest.configuration);
        self.player.settings_dirty = true;
        self.player.selected_game = Some(game.manifest.id.clone());
        let games = &mut self.player.library.games;
        games.retain(|entry| entry.manifest.id != game.manifest.id);
        games.push(game);
        games.sort_by(|a, b| a.manifest.title.cmp(&b.manifest.title));
        self.player.refresh_selected_game_details();
        if let Some(sync) = &mut self.player.library_sync {
            sync.changed_locally();
        }
    }
}

fn render_artwork(ui: &mut egui::Ui, draft: &mut GameDraft, text_fields: [egui::Id; 2]) {
    ui.strong("Cover image (optional)");
    let paste_hint = if cfg!(target_os = "macos") {
        "Paste an image with Command+V."
    } else {
        "Paste an image with Ctrl+V."
    };
    ui.horizontal_wrapped(|ui| {
        if ui
            .button("Choose image...")
            .on_hover_text(
                "Select a JPEG or PNG to preview and store with this game when you save.",
            )
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JPEG and PNG images", &["jpg", "jpeg", "png"])
                .pick_file()
            {
                draft.artwork_error = draft.choose_artwork(&path).err();
            }
        }
        ui.label(paste_hint);
    });

    let paste = crate::window_keyboard::with_artwork_paste(ui.ctx(), |paste| paste.pending.take());
    if let Some(paste) = paste {
        let text_paste = ui
            .memory(|memory| memory.focused().is_some_and(|id| text_fields.contains(&id)))
            && ui.input(|input| {
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Paste(_)))
            });
        if !text_paste {
            draft.artwork_error = paste.and_then(|image| draft.set_artwork(image)).err();
        }
    }

    let preview = if let Some(artwork) = &draft.artwork {
        crate::player_view::draw_image_preview(ui, "draft_cover", artwork.revision, 160.0, || {
            Some(Cow::Borrowed(&artwork.image))
        })
    } else {
        draft
            .original
            .as_ref()
            .and_then(|game| game.artwork.as_deref())
            .is_some_and(|bytes| crate::player_view::preview(ui, "draft_cover", bytes, 160.0))
    };
    if !preview {
        ui.weak("No cover image");
    }
    if let Some(error) = &draft.artwork_error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    ui.small("With no declared image, the library checks cover.jpg, cover.jpeg and cover.png in that order. The library picks up file changes automatically.");
}
