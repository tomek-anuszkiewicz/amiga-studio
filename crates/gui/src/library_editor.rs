//! Desktop Add game / Edit game form, separate from machine configuration.

use crate::library_edit::{validate_game_name, GameDraft};
use crate::EmulatorApp;
use std::path::Path;

impl EmulatorApp {
    pub(crate) fn open_library_editor(&mut self, game: Option<crate::game_library::LibraryGame>) {
        self.file_error = None;
        self.player.library_editor = Some(game.map_or_else(
            || GameDraft::new(self.player.catalog.selected.clone()),
            GameDraft::edit,
        ));
    }

    pub(crate) fn render_library_editor(&mut self, ui: &mut egui::Ui) {
        let Some(mut draft) = self.player.library_editor.take() else {
            return;
        };
        ui.heading(if draft.original.is_some() {
            "Edit game"
        } else {
            "Add game"
        });
        let label = ui.label("Game name");
        let response = ui.add(
            egui::TextEdit::singleline(&mut draft.name)
                .hint_text("Game name")
                .desired_width(400.0),
        );
        response
            .labelled_by(label.id)
            .on_hover_text("The game name also determines its directory inside the library");
        ui.label(format!(
            "Folder: {}",
            Path::new(&self.player.catalog.library_root)
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
        ui.strong("Cover image (optional)");
        ui.label("Place cover.jpg, cover.jpeg or cover.png in the game folder, then Refresh the library. JPEG and PNG are supported.");
        if let Some(path) = draft
            .original
            .as_ref()
            .and_then(|game| game.manifest.artwork.as_ref())
        {
            ui.label(format!("This game uses its declared cover: {path}"));
        }
        ui.separator();
        ui.strong("Disks");
        if let Some(game) = &draft.original {
            for (index, disk) in game.manifest.disks.iter().enumerate() {
                ui.label(format!("{}. {}", index + 1, disk.label));
            }
        }
        let mut remove = None;
        for (index, disk) in draft.imports.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("To copy: {}", disk.name));
                if ui
                    .button("Remove from draft")
                    .on_hover_text(
                        "Do not copy this selected disk; its source file stays untouched",
                    )
                    .clicked()
                {
                    remove = Some(index);
                }
            });
        }
        if let Some(index) = remove {
            draft.imports.remove(index);
        }
        if ui.button("Add disk...").on_hover_text("Select one standard 880 KiB ADF. Repeat for more disks; files are copied when you save the game.").clicked() {
            if let Some(path) = rfd::FileDialog::new().add_filter("Amiga disk images", &["adf"]).pick_file() {
                if let Err(error) = draft.add_disk(&path) { self.file_error = Some(error); }
            }
        }
        ui.small("The first disk starts in DF0 for a new game. Existing assignments stay unchanged; switch disks in the player panel.");
        if let Some(error) = &self.file_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        ui.separator();
        let configured = self
            .player
            .catalog
            .configurations
            .iter()
            .any(|c| c.id == draft.configuration);
        let mut save = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            save = ui
                .add_enabled(
                    configured && validate_game_name(&draft.name).is_ok(),
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
        if save {
            let edits_active = self
                .player
                .active_game
                .as_ref()
                .zip(draft.original.as_ref())
                .is_some_and(|(active, original)| active.same_entry(original));
            match draft.save(Path::new(&self.player.catalog.library_root)) {
                Ok(game) => {
                    self.complete_library_edit(game, edits_active);
                    return;
                }
                Err(error) => self.file_error = Some(error),
            }
        }
        self.player.library_editor = Some(draft);
    }

    fn complete_library_edit(&mut self, game: crate::game_library::LibraryGame, edits_active: bool) {
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
                        label: disk.label.clone(),
                        bytes: std::fs::read(game.directory.join(&disk.path)).ok(),
                    });
                }
            }
            self.player.active_game = Some(game.clone());
        }
        self.player
            .catalog
            .selected
            .clone_from(&game.manifest.configuration);
        self.player.settings_dirty = true;
        self.refresh_library();
        self.player.selected_game = self
            .player
            .library
            .games
            .iter()
            .position(|entry| entry.manifest.id == game.manifest.id);
        match crate::game_library::saves(&game) {
            Ok(saves) => self.player.saves = saves,
            Err(error) => self.file_error = Some(error),
        }
    }
}
