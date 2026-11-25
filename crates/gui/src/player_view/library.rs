//! Native library navigation and selected game details.

use super::preview;
use crate::EmulatorApp;

impl EmulatorApp {
    pub(super) fn render_library(&mut self, ui: &mut egui::Ui) {
        let available = self.player.library_available();
        let writable = available
            && self
                .player
                .library_sync
                .as_ref()
                .is_some_and(|sync| sync.writable());
        if let Some(sync) = &self.player.library_sync {
            if let Some(error) = &sync.read_error {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!(
                    "Could not read library: {error}. Choose a usable directory in Configurations."
                ),
                );
            } else if sync.reconciling() {
                ui.label("Checking the library on disk...");
            } else if let Some(error) = sync.write_error() {
                ui.colored_label(ui.visuals().error_fg_color, format!(
                    "Could not write to the library directory: {error}. Choose a writable directory in Configurations."
                ));
            }
            if let Some(warning) = sync.warning() {
                ui.colored_label(ui.visuals().error_fg_color, format!(
                    "Automatic library updates may be incomplete: {warning}. Check the directory in Configurations."
                ));
            }
        } else if self.player.library_root.is_none() {
            ui.label("Choose the library directory in Configurations.");
        }
        if let Some(error) = &self.player.library_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        if self.player.library_editor.is_some() {
            self.render_library_editor(ui, writable);
            return;
        }
        ui.heading("Game library");
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.player.busy && writable && !self.player.catalog.configurations.is_empty(),
                    egui::Button::new("Add game"),
                )
                .on_hover_text(
                    "Create a game folder and copy selected disk images into the library",
                )
                .clicked()
            {
                self.open_library_editor(None);
            }
            self.player.render_library_filter(ui);
        });
        if available && self.player.catalog.configurations.is_empty() {
            ui.label("Create a profile in Configurations before adding games.");
        }
        for error in &self.player.library.errors {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let filter = self.player.normalized_library_filter();
        let filter_active = !filter.is_empty();
        let mut selected = None;
        let mut visible = 0;
        let mut selected_visible = false;
        let cover_side = ui.available_width().min(110.0);
        ui.with_layout(
            egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
            |ui| {
                for game in &self.player.library.games {
                    if filter_active && !game.title_matches(filter) {
                        continue;
                    }
                    visible += 1;
                    let is_selected = self.player.selected_game.as_ref() == Some(&game.manifest.id);
                    selected_visible |= is_selected;
                    ui.allocate_ui_with_layout(
                        egui::Vec2::splat(cover_side),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            library_cover(ui, game, cover_side);
                            if ui
                                .selectable_label(is_selected, &game.manifest.title)
                                .clicked()
                            {
                                selected = Some(game.manifest.id.clone());
                                selected_visible = true;
                            }
                        },
                    );
                }
            },
        );
        if let Some(id) = selected {
            if self.player.selected_game.as_ref() != Some(&id) {
                self.player.selected_game = Some(id);
                if let Some(game) = self.player.selected_entry() {
                    let configuration = &game.manifest.configuration;
                    if self
                        .player
                        .catalog
                        .configurations
                        .iter()
                        .any(|entry| &entry.id == configuration)
                    {
                        self.player.catalog.selected = configuration.clone();
                        self.player.settings_dirty = true;
                    }
                }
                self.player.refresh_selected_game_details();
            }
        }
        if filter_active && visible == 0 {
            ui.label("No games match the filter.");
            return;
        }
        if !selected_visible {
            ui.label("Choose a game, or use Add game to create one.");
            return;
        }
        let Some(game) = self.player.selected_entry().cloned() else {
            return;
        };
        ui.separator();
        ui.heading(&game.manifest.title);
        if ui
            .add_enabled(
                !self.player.busy && writable,
                egui::Button::new("Edit game"),
            )
            .on_hover_text(
                "Change this game's name, folder, cover, description, configuration or disks",
            )
            .clicked()
        {
            self.open_library_editor(Some(game.clone()));
        }
        for paragraph in game.description.lines() {
            if let Some(title) = paragraph.strip_prefix("# ") {
                ui.heading(title);
            } else if let Some(title) = paragraph.strip_prefix("## ") {
                ui.strong(title);
            } else {
                ui.label(paragraph);
            }
        }
        let configuration = self
            .player
            .catalog
            .configurations
            .iter()
            .find(|configuration| configuration.id == game.manifest.configuration)
            .map_or("Configuration missing", |configuration| {
                configuration.name.as_str()
            });
        ui.label(format!("Configuration: {configuration}"));
        if ui
            .add_enabled(
                available && !self.player.busy,
                egui::Button::new("Start game"),
            )
            .clicked()
        {
            self.request_player_launch(crate::player::PendingLaunch::Library {
                game: game.clone(),
                save: None,
                normal_boot: false,
            });
        }
        if game.manifest.initial_state.is_some()
            && ui
                .add_enabled(
                    available && !self.player.busy,
                    egui::Button::new("Boot normally without initial state"),
                )
                .clicked()
        {
            self.request_player_launch(crate::player::PendingLaunch::Library {
                game: game.clone(),
                save: None,
                normal_boot: true,
            });
        }
        self.render_named_start_states(ui, &game, available, writable);
        ui.separator();
        ui.heading("Saved progress");
        self.render_save_catalog(ui, &game, false);
    }
}

fn library_cover(ui: &mut egui::Ui, game: &crate::game_library::LibraryGame, side: f32) {
    let size = egui::Vec2::splat(side);
    ui.allocate_ui_with_layout(
        size,
        egui::Layout::centered_and_justified(egui::Direction::TopDown),
        |ui| {
            ui.set_min_size(size);
            if !game
                .artwork
                .as_deref()
                .is_some_and(|bytes| preview(ui, ("cover", &game.directory), bytes, side))
            {
                let outline = &ui.visuals().widgets.noninteractive;
                ui.painter().rect_stroke(
                    ui.min_rect(),
                    outline.corner_radius,
                    outline.bg_stroke,
                    egui::StrokeKind::Inside,
                );
            }
        },
    );
}
