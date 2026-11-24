//! Reassign a library's games before removing one named configuration.

use crate::game_library::{
    load_game, relative_asset, replace_configuration, GameLibrary, LibraryGame,
};
use crate::library_catalog::CatalogWrite;
use crate::EmulatorApp;
use std::path::Path;

#[derive(Debug)]
pub(crate) struct ConfigurationDeletion {
    target: String,
    affected: Vec<LibraryGame>,
    replacement: String,
}

impl EmulatorApp {
    pub(crate) fn begin_configuration_deletion(&mut self) {
        if !self.configuration_deletion_allowed() {
            return;
        }
        let (Some(editor), Some(root)) = (&mut self.player.editor, &self.player.library_root)
        else {
            return;
        };
        let target = editor.catalog.selected.clone();
        match affected_games(root, &target) {
            Ok(affected) => {
                editor.error = None;
                editor.deletion = Some(ConfigurationDeletion {
                    target,
                    affected,
                    replacement: String::new(),
                });
            }
            Err(error) => {
                editor.error = Some(format!("Could not check configuration references: {error}"))
            }
        }
    }

    pub(crate) fn render_configuration_deletion(&mut self, ctx: &egui::Context) {
        let Some(editor) = &mut self.player.editor else {
            return;
        };
        let Some(deletion) = &mut editor.deletion else {
            return;
        };
        let Some(target) = editor
            .catalog
            .configurations
            .iter()
            .find(|cfg| cfg.id == deletion.target)
        else {
            editor.deletion = None;
            return;
        };
        let mut confirm = false;
        let mut cancel = false;
        let dialog =
            egui::Modal::new(egui::Id::new("delete-library-configuration")).show(ctx, |ui| {
                ui.set_max_width((ctx.screen_rect().width() - 40.0).min(470.0));
                ui.heading(format!("Delete {}?", target.name));
                let has_references = !deletion.affected.is_empty();
                if has_references {
                    ui.label("Choose the configuration these games will use for future launches.");
                    egui::ScrollArea::vertical()
                        .max_height(ctx.screen_rect().height() * 0.35)
                        .show(ui, |ui| {
                            for game in &deletion.affected {
                                ui.label(&game.manifest.title);
                            }
                        });
                    let catalog = &editor.catalog.configurations;
                    if catalog.iter().all(|cfg| cfg.id == target.id) {
                        ui.label("Create another configuration before deleting this one.");
                    } else {
                        super::player_view::choose_configuration(
                            ui,
                            "Replacement configuration",
                            &mut deletion.replacement,
                            catalog.iter().filter(|cfg| cfg.id != target.id),
                        );
                    }
                } else {
                    ui.label("No games in this library use this configuration.");
                }
                ui.horizontal_wrapped(|ui| {
                    confirm = ui
                        .add_enabled(
                            !has_references || !deletion.replacement.is_empty(),
                            egui::Button::new("Confirm deletion"),
                        )
                        .clicked();
                    cancel = ui.button("Cancel deletion").clicked();
                });
            });
        if dialog.should_close() || cancel {
            editor.deletion = None;
        } else if confirm {
            self.delete_configuration();
        }
    }

    fn delete_configuration(&mut self) {
        let allowed = self.configuration_deletion_allowed();
        let Some(editor) = &mut self.player.editor else {
            return;
        };
        let Some(deletion) = editor.deletion.take() else {
            return;
        };
        if !allowed {
            editor.error = Some("Finish the current edits before deleting a configuration".into());
            return;
        }
        let mut changed = Vec::new();
        let result = self.reassign_and_delete(&deletion, &mut changed);
        match result {
            Ok((catalog, source)) => {
                self.player.catalog = catalog;
                self.player.catalog_source = Some(source);
                self.player.settings_dirty = true;
                self.file_error = None;
            }
            Err(error) => {
                self.reload_library_catalog();
                let outcome = if changed.is_empty() {
                    "No game references were written.".into()
                } else {
                    format!(
                        "Game references written before the failure: {}.",
                        changed.join(", ")
                    )
                };
                self.configuration_error(format!(
                    "Kept the configuration because deletion stopped: {error}. {outcome}"
                ));
            }
        }
        self.reload_configuration_draft();
        self.refresh_library();
    }

    fn reassign_and_delete(
        &self,
        deletion: &ConfigurationDeletion,
        changed: &mut Vec<String>,
    ) -> Result<(crate::player_config::ConfigurationCatalog, Vec<u8>), String> {
        let root = self
            .player
            .library_root
            .as_deref()
            .ok_or("Choose a library directory")?;
        let current = &self.player.catalog;
        if current
            .configurations
            .iter()
            .all(|cfg| cfg.id != deletion.target)
        {
            return Err("The selected configuration is no longer available".into());
        }
        let affected = affected_games(root, &deletion.target)?;
        if affected.len() != deletion.affected.len()
            || affected
                .iter()
                .zip(&deletion.affected)
                .any(|(fresh, shown)| {
                    fresh.directory != shown.directory || fresh.manifest != shown.manifest
                })
        {
            return Err("Affected games changed since confirmation. Review the list again".into());
        }
        let replacement = current
            .configurations
            .iter()
            .find(|cfg| cfg.id == deletion.replacement && cfg.id != deletion.target);
        if !affected.is_empty() {
            let replacement = replacement.ok_or("Choose an existing replacement configuration")?;
            for game in &affected {
                validate_media(game, replacement.launch.drives)
                    .map_err(|error| format!("{}: {error}", game.manifest.title))?;
            }
        }
        let mut next = current.clone();
        next.configurations.retain(|cfg| cfg.id != deletion.target);
        next.selected = replacement.map_or_else(
            || {
                next.configurations
                    .first()
                    .map_or_else(String::new, |cfg| cfg.id.clone())
            },
            |cfg| cfg.id.clone(),
        );
        let write = CatalogWrite::prepare(root, self.player.catalog_source.as_deref(), &next)?;
        for game in &affected {
            let saved = replace_configuration(game, &deletion.replacement)
                .map_err(|error| format!("{}: {error}", game.manifest.title))?;
            changed.push(game.manifest.title.clone());
            if load_game(&game.directory)?.manifest != saved.manifest {
                return Err(format!(
                    "Could not verify the new reference for {}",
                    game.manifest.title
                ));
            }
        }
        if !affected_games(root, &deletion.target)?.is_empty() {
            return Err("There are still games using the old configuration".into());
        }
        let source = write.publish()?;
        Ok((next, source))
    }
}

fn affected_games(root: &Path, configuration: &str) -> Result<Vec<LibraryGame>, String> {
    let library = GameLibrary::discover(root)?;
    if !library.errors.is_empty() {
        return Err(library.errors.join("\n"));
    }
    Ok(library
        .games
        .into_iter()
        .filter(|game| game.manifest.configuration == configuration)
        .collect())
}

fn validate_media(
    game: &LibraryGame,
    connected: [bool; config::FLOPPY_DRIVE_COUNT],
) -> Result<(), String> {
    for (drive, id) in game.manifest.mounted.iter().enumerate() {
        if id.is_some() && !connected[drive] {
            return Err(format!(
                "DF{drive} is disconnected in the replacement configuration"
            ));
        }
    }
    for disk in &game.manifest.disks {
        let path = relative_asset(&game.directory, &disk.path)?;
        let bytes =
            std::fs::read(path).map_err(|error| format!("{}: {error}", disk.display_name()))?;
        if !debugger::launch::disk_image_is_supported(&bytes) {
            return Err(format!(
                "{} is an unsupported ADF ({} bytes)",
                disk.display_name(),
                bytes.len()
            ));
        }
    }
    Ok(())
}
