//! Library, manual setup and compact gameplay controls.

use crate::player::{PlayerScreen, SessionDisk};
use crate::{EmulatorApp, ViewMode};

pub const SAVED_PROGRESS_DIALOG: &str = "saved-progress";

impl EmulatorApp {
    pub(crate) fn render_player_screen(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("player_navigation").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Library").clicked() {
                    self.navigate_player(PlayerScreen::Library);
                }
                if ui.button("Manual setup").clicked() {
                    self.navigate_player(PlayerScreen::Setup);
                }
                if ui.button("Configurations").clicked() {
                    self.navigate_player(PlayerScreen::Configuration);
                }
                if ui.button("Return to session").clicked() {
                    self.navigate_player(PlayerScreen::Session);
                }
                if ui.button("Debug").clicked() {
                    self.set_view_mode(ViewMode::Developer);
                }
            });
        });
        if self.player.screen == PlayerScreen::Configuration {
            self.render_configuration_actions(ctx);
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match self.player.screen {
                PlayerScreen::Configuration => {
                    let saving = self
                        .player
                        .editor
                        .as_ref()
                        .is_some_and(|editor| editor.saving);
                    ui.add_enabled_ui(!saving, |ui| self.render_configuration_editor(ui));
                }
                PlayerScreen::Setup => self.render_manual_setup(ui),
                PlayerScreen::Library => {
                    #[cfg(not(target_arch = "wasm32"))]
                    self.render_library(ui);
                }
                PlayerScreen::Session => {}
            });
        });
    }

    pub(crate) fn configuration_selector(&mut self, ui: &mut egui::Ui) {
        let before = self.player.catalog.selected.clone();
        configuration_selector(ui, &mut self.player.catalog);
        self.player.settings_dirty |= before != self.player.catalog.selected;
    }

    fn render_manual_setup(&mut self, ui: &mut egui::Ui) {
        ui.heading("Manual machine setup");
        ui.label("Choose a configuration, Kickstart and optional disks. A ROM with empty drives is valid.");
        let previous = self.player.catalog.selected.clone();
        self.configuration_selector(ui);
        if previous != self.player.catalog.selected {
            self.player.rom = None;
        }
        let Some(configuration) = self.player.catalog.selected().cloned() else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            if ui.button("Select Kickstart...").clicked() {
                self.open_setup_asset(None);
            }
            ui.label(if configuration.launch.rom.is_empty() {
                "Kickstart not selected"
            } else {
                &configuration.launch.rom
            });
            #[cfg(target_arch = "wasm32")]
            if self.player.selected_rom(&configuration).is_none() {
                ui.label("Select the ROM again for this session");
            }
        });
        for drive in 0..4 {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("DF{drive}:"));
                if configuration.launch.drives[drive] {
                    disk_dropdown(
                        ui,
                        ("setup_drive", drive),
                        &mut self.player.setup_mounted[drive],
                        &self.player.setup_disks,
                    );
                    if ui.button("Select ADF...").clicked() {
                        self.open_setup_asset(Some(drive));
                    }
                } else {
                    ui.label("Disconnected");
                }
            });
        }
        if let Err(error) = self.player.catalog.validate(&configuration) {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        if ui
            .add_enabled(
                !self.player.busy && self.manual_ready(),
                egui::Button::new("Start"),
            )
            .clicked()
        {
            self.request_player_launch(crate::player::PendingLaunch::Manual);
        }
    }

    pub(crate) fn render_player_confirmation(&mut self, ctx: &egui::Context) {
        if self.player.pending_launch.is_none() {
            return;
        }
        let mut replace = false;
        let mut keep = false;
        egui::Modal::new(egui::Id::new("replace-player-session")).show(ctx, |ui| {
            ui.heading("Replace current session?");
            ui.label("This action resets or replaces the current machine. Save progress first if needed.");
            ui.horizontal(|ui| {
                replace = ui.button("Replace session").clicked();
                keep = ui.button("Keep current session").clicked();
            });
        });
        if replace {
            if let Some(launch) = self.player.pending_launch.take() {
                self.confirm_player_launch(launch);
            }
        } else if keep {
            self.player.pending_launch = None;
        }
    }

    pub(crate) fn render_player_controls(&mut self, ctx: &egui::Context) {
        let wide = ctx.screen_rect().width() >= 850.0;
        if wide {
            egui::SidePanel::right("game_controls")
                .exact_width(220.0)
                .resizable(false)
                .show(ctx, |ui| {
                    self.player_controls(ui);
                });
        } else {
            egui::TopBottomPanel::bottom("compact_game_controls").show(ctx, |ui| {
                egui::CollapsingHeader::new("Game controls")
                    .show(ui, |ui| self.player_controls(ui));
            });
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn render_saved_progress(&mut self, ctx: &egui::Context) {
        if !self.player.show_saves || self.file_error.is_some() || self.load_binary_modal_open {
            return;
        }
        let close = egui::Modal::new(egui::Id::new(SAVED_PROGRESS_DIALOG)).show(ctx, |ui| {
            ui.heading("Saved progress");
            egui::ScrollArea::vertical()
                .max_height(ctx.screen_rect().height() * 0.7)
                .show(ui, |ui| self.render_save_catalog(ui, true));
            ui.button("Close").clicked()
        });
        if close.inner {
            self.player.show_saves = false;
        }
        if !self.player.show_saves {
            ctx.request_repaint();
        }
    }

    fn player_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Game");
        for drive in 0..4 {
            let status = self.host.status.drives[drive];
            ui.horizontal(|ui| {
                ui.label(format!("DF{drive}:"));
                if status.connected {
                    let mut selected = self.player.mounted[drive].clone();
                    self.player.drive_activity[drive].show(
                        ui,
                        drive,
                        (
                            self.host.status.generation,
                            self.host.status.drive_reset_sequence,
                        ),
                        status,
                        self.theme.tokens(),
                    );
                    ui.add_enabled_ui(!self.player.busy, |ui| {
                        disk_dropdown(ui, ("game_drive", drive), &mut selected, &self.player.disks);
                    });
                    if selected != self.player.mounted[drive] {
                        self.mount_player_disk(drive, selected);
                    }
                    if status.inserted && self.player.mounted[drive].is_none() {
                        ui.label("Snapshot media");
                    }
                } else {
                    ui.label("Disconnected");
                }
            });
        }
        ui.separator();
        let execution_label = if self.host.status.running {
            "Pause"
        } else {
            "Resume"
        };
        if ui
            .add_enabled(!self.player.busy, egui::Button::new(execution_label))
            .on_hover_text(
                "Pause at the field boundary or resume the same machine and mounted media.",
            )
            .clicked()
        {
            self.player.captured = false;
            self.toggle_run();
        }
        #[cfg(not(target_arch = "wasm32"))]
        let library_game = self.player.active_game.is_some();
        #[cfg(target_arch = "wasm32")]
        let library_game = false;
        let reset_help = if library_game {
            "Restore this game's startup disks, then reset and run with the current hardware and ROM."
        } else {
            "Reset and run the current machine with the disks already inserted."
        };
        if ui
            .add_enabled(!self.player.busy, egui::Button::new("Reset"))
            .on_hover_text(reset_help)
            .clicked()
        {
            self.reset_player();
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.player.busy, egui::Button::new("Save"))
                .on_hover_text("Capture the current machine and preview; associated desktop games create a new durable save.")
                .clicked()
            {
                self.save_player(None);
            }
            if ui
                .add_enabled(!self.player.busy, egui::Button::new("Load"))
                .on_hover_text("Choose saved progress for this game, or import a snapshot file in manual mode.")
                .clicked()
            {
                self.load_player();
            }
            #[cfg(not(target_arch = "wasm32"))]
            if self.player.active_game.is_some()
                && ui
                    .push_id("amiga-game-screenshot-button", |ui| {
                        ui.button("Screenshot")
                            .on_hover_text("Save the displayed Amiga image to this game's Screenshots folder.")
                            .clicked()
                    })
                    .inner
            {
                self.take_game_screenshot();
            }
        });
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(save) = self.player.saves.first() {
            ui.label(format!(
                "Latest save: {}",
                crate::host_clock::format_utc(save.header.timestamp)
            ));
        }
        ui.separator();
        #[cfg(not(target_arch = "wasm32"))]
        if ui.button("Library").clicked() {
            self.navigate_player(PlayerScreen::Library);
        }
        if ui.button("Setup").clicked() {
            self.navigate_player(PlayerScreen::Setup);
        }
        if ui.button("Debug").clicked() {
            self.player.captured = false;
            self.set_view_mode(ViewMode::Developer);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render_library(&mut self, ui: &mut egui::Ui) {
        let writable = self
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
        } else {
            ui.label("Choose the library directory in Configurations.");
        }
        if self.player.library_editor.is_some() {
            self.render_library_editor(ui, writable);
            return;
        }
        ui.heading("Game library");
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.player.busy && writable, egui::Button::new("Add game"))
                .on_hover_text(
                    "Create a game folder and copy selected disk images into the library",
                )
                .clicked()
            {
                self.open_library_editor(None);
            }
            let label = ui.label("Filter games");
            crate::keyboard_input::text_edit(
                ui,
                egui::TextEdit::singleline(&mut self.player.library_filter)
                    .id_salt("library-filter")
                    .desired_width(200.0)
                    .hint_text("Game name"),
            )
            .labelled_by(label.id)
            .on_hover_text("Keep game names containing this text, ignoring letter case.");
        });
        for error in &self.player.library.errors {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let filter = self.player.library_filter.to_lowercase();
        let mut selected = None;
        let mut visible = 0;
        let mut selected_visible = false;
        ui.horizontal_wrapped(|ui| {
            for game in &self.player.library.games {
                if !filter.is_empty() && !game.manifest.title.to_lowercase().contains(&filter) {
                    continue;
                }
                visible += 1;
                let is_selected = self.player.selected_game.as_ref() == Some(&game.manifest.id);
                selected_visible |= is_selected;
                ui.vertical(|ui| {
                    if let Some(bytes) = &game.artwork {
                        preview(ui, ("cover", &game.directory), bytes, 110.0);
                    }
                    if ui
                        .selectable_label(is_selected, &game.manifest.title)
                        .clicked()
                    {
                        selected = Some(game.manifest.id.clone());
                        selected_visible = true;
                    }
                });
            }
        });
        if let Some(id) = selected {
            if self.player.selected_game.as_ref() != Some(&id) {
                self.player.selected_game = Some(id);
                if let Some(game) = self.player.selected_entry() {
                    self.player.catalog.selected = game.manifest.configuration.clone();
                    self.player.settings_dirty = true;
                }
                self.player.refresh_selected_saves();
            }
        }
        if !filter.is_empty() && visible == 0 {
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
                "Change this game's name, folder, description, configuration or disk list",
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
            .map_or("Missing configuration", |configuration| {
                configuration.name.as_str()
            });
        ui.label(format!("Configuration: {configuration}"));
        if ui
            .add_enabled(!self.player.busy, egui::Button::new("Start game"))
            .clicked()
        {
            self.request_player_launch(crate::player::PendingLaunch::Library {
                game: game.clone(),
                save: None,
                normal_boot: false,
            });
        }
        if self.player.initial_error && ui.button("Boot normally without initial state").clicked() {
            self.request_player_launch(crate::player::PendingLaunch::Library {
                game,
                save: None,
                normal_boot: true,
            });
        }
        self.render_save_catalog(ui, false);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render_save_catalog(&mut self, ui: &mut egui::Ui, active: bool) {
        let (saves, error) = if active {
            (&self.player.saves, &self.player.session_save_error)
        } else {
            (
                &self.player.selected_saves,
                &self.player.selected_save_error,
            )
        };
        if let Some(error) = error {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Could not list saved progress: {error}"),
            );
        }
        let mut load = None;
        for save in saves {
            ui.horizontal(|ui| {
                preview(ui, ("save", &save.path), &save.header.screenshot_png, 110.0);
                if ui
                    .button(format!(
                        "Continue - {}",
                        crate::host_clock::format_utc(save.header.timestamp)
                    ))
                    .clicked()
                {
                    load = Some(save.path.clone());
                }
            });
        }
        if let Some(path) = load {
            let game = if active {
                self.player.active_game.clone()
            } else {
                self.player.selected_entry().cloned()
            };
            if let Some(game) = game {
                self.request_player_launch(crate::player::PendingLaunch::Library {
                    game,
                    save: Some(path),
                    normal_boot: false,
                });
                self.player.show_saves = false;
            }
        }
    }
}

fn disk_dropdown(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    mounted: &mut Option<String>,
    disks: &[SessionDisk],
) {
    let label = mounted
        .as_ref()
        .and_then(|id| disks.iter().find(|disk| &disk.id == id))
        .map_or_else(
            || {
                if mounted.is_some() {
                    "Snapshot media"
                } else {
                    "Empty"
                }
            },
            |disk| disk.label.as_str(),
        );
    egui::ComboBox::from_id_salt(id)
        .width(110.0)
        .truncate()
        .selected_text(label)
        .show_ui(ui, |ui| {
            ui.selectable_value(mounted, None, "Eject / Empty");
            for (index, disk) in disks.iter().enumerate() {
                ui.selectable_value(
                    mounted,
                    Some(disk.id.clone()),
                    format!("{} - {}", index + 1, disk.label),
                );
            }
        });
}

#[cfg(not(target_arch = "wasm32"))]
fn preview(ui: &mut egui::Ui, id: impl std::hash::Hash, bytes: &[u8], width: f32) {
    let id = ui.id().with((id, egui::Id::new(bytes)));
    let texture = ui
        .ctx()
        .data_mut(|data| data.get_temp::<egui::TextureHandle>(id));
    let texture = texture.or_else(|| {
        let image = image::load_from_memory(bytes).ok()?.into_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        let texture = ui.ctx().load_texture(
            format!("{id:?}"),
            egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
            egui::TextureOptions::LINEAR,
        );
        ui.ctx()
            .data_mut(|data| data.insert_temp(id, texture.clone()));
        Some(texture)
    });
    if let Some(texture) = texture {
        ui.add(egui::Image::new(&texture).max_width(width));
    }
}

pub fn configuration_selector(
    ui: &mut egui::Ui,
    catalog: &mut crate::player_config::ConfigurationCatalog,
) {
    let selected = catalog
        .selected()
        .map_or("Missing configuration", |c| c.name.as_str());
    egui::ComboBox::from_id_salt("named_configuration")
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for configuration in &catalog.configurations {
                ui.selectable_value(
                    &mut catalog.selected,
                    configuration.id.clone(),
                    &configuration.name,
                );
            }
        });
}
