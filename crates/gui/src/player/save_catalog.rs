use crate::game_library::{self, LibraryGame};
use crate::EmulatorApp;
use std::path::{Path, PathBuf};
use std::time::Duration;

const UNDO_SECONDS: f64 = 10.0;

#[derive(Debug)]
struct PendingDeletion {
    game_directory: PathBuf,
    game_id: String,
    path: PathBuf,
    deadline: f64,
}

impl PendingDeletion {
    fn matches(&self, game: &LibraryGame, path: &Path) -> bool {
        self.game_directory == game.directory
            && self.game_id == game.manifest.id
            && self.path == path
    }
}

#[derive(Debug)]
pub(crate) struct DescriptionDraft {
    pub game: LibraryGame,
    pub path: PathBuf,
    pub text: String,
}

#[derive(Debug, Default)]
pub(crate) struct SaveCatalogActions {
    deletions: Vec<PendingDeletion>,
    pub description: Option<DescriptionDraft>,
    pub times: crate::host_clock::SaveTimes,
}

impl SaveCatalogActions {
    pub fn deadline(&self, game: &LibraryGame, path: &Path) -> Option<f64> {
        self.deletions
            .iter()
            .find(|deletion| deletion.matches(game, path))
            .map(|deletion| deletion.deadline)
    }
}

impl EmulatorApp {
    pub(crate) fn load_catalog_save(&mut self, game: LibraryGame, path: PathBuf, quick: bool) {
        if self.player.busy || self.player.save_catalog.deadline(&game, &path).is_some() {
            return;
        }
        if quick {
            match game_library::current_quicksave(&game.directory) {
                Ok(Some(selected)) if game_library::same_save(&selected, &path) => {}
                Ok(_) => {
                    self.file_error = Some(
                        "The quicksave designation changed. Check saved progress and try again."
                            .into(),
                    );
                    self.refresh_player_saves();
                    return;
                }
                Err(error) => {
                    self.file_error = Some(error);
                    self.refresh_player_saves();
                    return;
                }
            }
        }
        let directory = game.directory.clone();
        let game_id = game.manifest.id.clone();
        self.open_library_game(game, Some(path), false, quick);
        if !self.player.busy && self.file_error.is_some() {
            self.refresh_game_saves(&directory, &game_id);
        }
    }

    pub(crate) fn begin_delete_save(
        &mut self,
        ctx: &egui::Context,
        game: &LibraryGame,
        path: PathBuf,
    ) {
        if self.player.save_catalog.deadline(game, &path).is_some() {
            return;
        }
        self.player.save_catalog.deletions.push(PendingDeletion {
            game_directory: game.directory.clone(),
            game_id: game.manifest.id.clone(),
            path,
            deadline: ctx.input(|input| input.time) + UNDO_SECONDS,
        });
        ctx.request_repaint();
    }

    pub(crate) fn undo_delete_save(&mut self, game: &LibraryGame, path: &Path) {
        self.player
            .save_catalog
            .deletions
            .retain(|deletion| !deletion.matches(game, path));
        self.refresh_game_saves(&game.directory, &game.manifest.id);
    }

    pub(crate) fn update_save_deletions(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|input| input.time);
        let deletions = &mut self.player.save_catalog.deletions;
        let mut expired = Vec::new();
        let mut next_update = 1.0_f64;
        deletions.retain_mut(|deletion| {
            let remaining = deletion.deadline - now;
            if remaining <= 0.0 {
                expired.push(PendingDeletion {
                    game_directory: std::mem::take(&mut deletion.game_directory),
                    game_id: std::mem::take(&mut deletion.game_id),
                    path: std::mem::take(&mut deletion.path),
                    deadline: deletion.deadline,
                });
                false
            } else {
                next_update = next_update.min(remaining);
                !self.closing
            }
        });
        if !deletions.is_empty() {
            ctx.request_repaint_after(Duration::from_secs_f64(next_update));
        }
        for deletion in expired {
            if let Err(error) = game_library::delete_save(
                &deletion.game_directory,
                &deletion.game_id,
                &deletion.path,
            ) {
                let name = deletion
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                self.file_error = Some(format!("Could not complete deletion of {name}: {error}"));
            }
            self.refresh_game_saves(&deletion.game_directory, &deletion.game_id);
        }
    }

    pub(crate) fn refresh_game_saves(&mut self, directory: &Path, game_id: &str) {
        if let Some(sync) = &mut self.player.library_sync {
            if sync.owns_game_directory(directory) {
                sync.changed_locally();
            }
        }
        let is_game =
            |game: &LibraryGame| game.directory == directory && game.manifest.id == game_id;
        if self.player.active_game.as_ref().is_some_and(is_game) {
            self.refresh_player_saves();
        }
        if self.player.selected_entry().is_some_and(is_game) {
            self.player.refresh_selected_saves();
        }
    }

    pub(crate) fn poll_save_times(&mut self, ctx: &egui::Context) {
        let player = &mut self.player;
        player.save_catalog.times.poll(ctx, || {
            player
                .saves
                .iter()
                .chain(&player.selected_saves)
                .filter_map(|save| save.timestamp())
        });
    }
}
