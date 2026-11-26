//! Named start-state actions in the native player.

use super::PendingLaunch;
use crate::game_library::{GameManifest, LibraryGame};
use crate::start_state_storage::{self, StartStateTarget};
use crate::EmulatorApp;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const UNDO_PERIOD: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub(super) struct NameEdit {
    action: NameAction,
    name: String,
    error: Option<String>,
    focus_name: bool,
}

#[derive(Debug)]
enum NameAction {
    Promote {
        directory: PathBuf,
        game_id: String,
        source: PathBuf,
        quicksave: bool,
    },
    Rename(StartStateTarget),
}

#[derive(Debug)]
pub(super) struct PendingDeletion {
    target: StartStateTarget,
    label: String,
    deadline: Instant,
}

enum StateAction {
    Load(String),
    Rename(String, String),
    Delete(String, String),
}

impl EmulatorApp {
    pub(crate) fn begin_start_state_promotion(
        &mut self,
        game: LibraryGame,
        source: PathBuf,
        quicksave: bool,
    ) {
        self.player.start_state_name = Some(NameEdit {
            action: NameAction::Promote {
                directory: game.directory,
                game_id: game.manifest.id,
                source,
                quicksave,
            },
            name: String::new(),
            error: None,
            focus_name: true,
        });
    }

    pub(crate) fn editing_start_state_name(&self) -> bool {
        self.player.start_state_name.is_some()
    }

    pub(crate) fn render_start_state_name(&mut self, ctx: &egui::Context) {
        let Some(mut edit) = self.player.start_state_name.take() else {
            return;
        };
        let (title, confirm_label) = match &edit.action {
            NameAction::Promote { .. } => ("Add to start states", "Add start state"),
            NameAction::Rename(_) => ("Rename start state", "Save name"),
        };
        let mut confirm = false;
        let mut cancel = false;
        let modal = egui::Modal::new(egui::Id::unique("name-start-state")).show(ctx, |ui| {
            ui.heading(title);
            let label = ui.label("Start state name");
            let name =
                crate::keyboard_input::text_edit(ui, egui::TextEdit::singleline(&mut edit.name))
                    .labelled_by(label.id);
            if std::mem::take(&mut edit.focus_name) {
                name.request_focus();
            }
            if let Some(error) = &edit.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.horizontal(|ui| {
                confirm = ui
                    .add_enabled(
                        !edit.name.trim().is_empty(),
                        egui::Button::new(confirm_label),
                    )
                    .clicked();
                cancel = ui.button("Cancel").clicked();
            });
        });
        if cancel || modal.should_close() {
            return;
        }
        if confirm {
            let name = edit.name.trim().to_owned();
            let (directory, result) = match &edit.action {
                NameAction::Promote {
                    directory,
                    game_id,
                    source,
                    quicksave,
                } => (
                    directory,
                    start_state_storage::promote(directory, game_id, source, *quicksave, name),
                ),
                NameAction::Rename(target) => (&target.directory, target.rename(name)),
            };
            match result {
                Ok(manifest) => {
                    self.start_state_collection_changed(directory, &manifest);
                    return;
                }
                Err(error) => edit.error = Some(format!("Could not save the start state: {error}")),
            }
        }
        self.player.start_state_name = Some(edit);
    }

    pub(crate) fn render_named_start_states(
        &mut self,
        ui: &mut egui::Ui,
        game: &LibraryGame,
        available: bool,
        writable: bool,
    ) {
        ui.separator();
        ui.heading("Start states");
        if game.manifest.start_states.is_empty() {
            ui.label("Add a start state from saved progress.");
        }
        let mut action = None;
        for (index, state) in game.manifest.start_states.iter().enumerate() {
            let error = self.player.unavailable_start_states.get(&state.id);
            let pending = self.start_state_pending(game, &state.id);
            ui.push_id((index, &state.id), |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            available && !self.player.busy && !pending && error.is_none(),
                            egui::Button::new(format!("Load {}", state.name)),
                        )
                        .on_hover_text("Restore this saved machine and resume play.")
                        .clicked()
                    {
                        action = Some(StateAction::Load(state.id.clone()));
                    }
                    if ui
                        .add_enabled(
                            writable && !pending,
                            egui::Button::new(format!("Rename {}", state.name)),
                        )
                        .on_hover_text("Change the start state's display name.")
                        .clicked()
                    {
                        action = Some(StateAction::Rename(state.id.clone(), state.name.clone()));
                    }
                    if ui
                        .add_enabled(
                            writable && !pending,
                            egui::Button::new(format!("Delete {}", state.name)),
                        )
                        .on_hover_text("Remove this start state after the Undo period.")
                        .clicked()
                    {
                        action = Some(StateAction::Delete(state.id.clone(), state.name.clone()));
                    }
                });
                if let Some(error) = error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
            });
        }
        match action {
            Some(StateAction::Load(id)) => {
                self.request_player_launch(PendingLaunch::LibraryStartState {
                    game: game.clone(),
                    id,
                });
            }
            Some(StateAction::Rename(id, name)) => {
                self.player.start_state_name = Some(NameEdit {
                    action: NameAction::Rename(StartStateTarget::new(game, id)),
                    name,
                    error: None,
                    focus_name: true,
                });
            }
            Some(StateAction::Delete(id, name)) => {
                self.player.start_state_deletions.push(PendingDeletion {
                    target: StartStateTarget::new(game, id),
                    label: format!("{} / {name}", game.manifest.title),
                    deadline: Instant::now() + UNDO_PERIOD,
                });
                ui.ctx().request_repaint();
            }
            None => {}
        }
    }

    pub(super) fn start_named_state(&mut self, mut game: LibraryGame, id: String) {
        if self.start_state_pending(&game, &id) {
            self.file_error =
                Some("Undo the pending deletion before loading this start state".into());
            return;
        }
        let target = StartStateTarget::new(&game, id);
        match target.resolve() {
            Ok((path, manifest)) => {
                game.manifest = manifest;
                if self.prompt_for_missing_configuration(&game, Some(&path), false) {
                    // Verify the file before offering to write a replacement configuration reference.
                    if let Err(error) =
                        crate::game_library::read_associated_save(&path, &game.manifest)
                    {
                        self.player.missing_configuration = None;
                        self.file_error = Some(format!("Could not load the start state: {error}"));
                    } else if let Some(repair) = &mut self.player.missing_configuration {
                        repair.start_state = Some(target.id);
                    }
                } else {
                    self.open_library_game(game, Some(path), false, false);
                }
            }
            Err(error) => {
                self.file_error = Some(format!("Could not load the start state: {error}"))
            }
        }
    }

    fn start_state_pending(&self, game: &LibraryGame, id: &str) -> bool {
        self.player.start_state_deletions.iter().any(|pending| {
            pending.target.game_id == game.manifest.id
                && pending.target.directory == game.directory
                && pending.target.id == id
        })
    }

    pub(crate) fn render_start_state_undo(&mut self, ui: &mut egui::Ui) {
        if self.player.start_state_deletions.is_empty() {
            return;
        }
        let now = Instant::now();
        let mut undo = None;
        egui::Panel::top("start-state-undo").show(ui, |ui| {
            for (index, pending) in self.player.start_state_deletions.iter().enumerate() {
                let remaining = pending.deadline.saturating_duration_since(now);
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!(
                        "Deleting {} in {} s",
                        pending.label,
                        remaining.as_millis().div_ceil(1000)
                    ));
                    if ui
                        .add_enabled(!remaining.is_zero(), egui::Button::new("Undo"))
                        .clicked()
                    {
                        undo = Some(index);
                    }
                });
                ui.ctx()
                    .request_repaint_after(remaining.min(Duration::from_secs(1)));
            }
        });
        if let Some(index) = undo {
            self.player.start_state_deletions.remove(index);
        }
    }

    pub(crate) fn finish_start_state_deletions(&mut self, closing: bool) -> bool {
        let now = Instant::now();
        if !closing
            && self
                .player
                .start_state_deletions
                .iter()
                .all(|pending| now < pending.deadline)
        {
            return true;
        }
        let mut errors = Vec::new();
        for pending in std::mem::take(&mut self.player.start_state_deletions) {
            if now < pending.deadline {
                self.player.start_state_deletions.push(pending);
                continue;
            }
            match pending.target.delete() {
                Ok(manifest) => {
                    self.start_state_collection_changed(&pending.target.directory, &manifest)
                }
                Err(error) => {
                    errors.push(format!("Could not delete {}: {error}", pending.label));
                    if let Some(sync) = &mut self.player.library_sync {
                        sync.changed_locally();
                    }
                }
            }
        }
        if errors.is_empty() {
            if closing {
                self.player.start_state_deletions.clear();
            }
            true
        } else {
            self.file_error = Some(errors.join("\n"));
            false
        }
    }

    pub(crate) fn relocate_start_state_deletions(
        &mut self,
        original: &LibraryGame,
        game: &LibraryGame,
    ) {
        for pending in &mut self.player.start_state_deletions {
            if pending.target.game_id == original.manifest.id
                && pending.target.directory == original.directory
            {
                pending.target.directory.clone_from(&game.directory);
            }
        }
    }

    fn start_state_collection_changed(&mut self, directory: &Path, manifest: &GameManifest) {
        let new_states = &manifest.start_states;
        for game in self
            .player
            .library
            .games
            .iter_mut()
            .chain(self.player.active_game.iter_mut())
        {
            if game.directory == directory && game.manifest.id == manifest.id {
                game.manifest.start_states.clone_from(new_states);
            }
        }
        if let Some(game) = self.player.selected_entry() {
            if game.directory == directory && game.manifest.id == manifest.id {
                self.player.unavailable_start_states = start_state_storage::unavailable(game);
            }
        }
        if let Some(sync) = &mut self.player.library_sync {
            sync.changed_locally();
        }
    }
}
