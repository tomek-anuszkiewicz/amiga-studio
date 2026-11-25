//! Library, manual setup and compact gameplay controls.

use crate::keyboard_input::KeyCommand;
use crate::player::{PlayerScreen, SessionDisk};
use crate::EmulatorApp;

#[cfg(not(target_arch = "wasm32"))]
mod library;

pub const SAVED_PROGRESS_DIALOG: &str = "saved-progress";

const PLAYER_GUIDE: &str = include_str!("../../../docs/player/README.md");
const SIDE_PANEL_MIN_WINDOW_WIDTH: f32 = 850.0;
const SIDE_PANEL_DEFAULT_WIDTH: f32 = 220.0;
const SIDE_PANEL_MIN_WIDTH: f32 = 192.0;
const SIDE_PANEL_MAX_WIDTH: f32 = 460.0;
const DRIVE_LABELS: [&str; config::FLOPPY_DRIVE_COUNT] = ["DF0:", "DF1:", "DF2:", "DF3:"];

impl EmulatorApp {
    pub(crate) fn render_player_screen(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("player_navigation").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                #[cfg(not(target_arch = "wasm32"))]
                if ui
                    .selectable_label(self.player.screen == PlayerScreen::Library, "Library")
                    .on_hover_text("Browse the desktop game library.")
                    .clicked()
                {
                    self.navigate_player(PlayerScreen::Library);
                }
                if ui
                    .selectable_label(self.player.screen == PlayerScreen::Setup, "Manual setup")
                    .on_hover_text(
                        "Choose a configuration, Kickstart and disks for a manual launch.",
                    )
                    .clicked()
                {
                    self.navigate_player(PlayerScreen::Setup);
                }
                if ui
                    .selectable_label(
                        self.player.screen == PlayerScreen::Configuration,
                        "Configurations",
                    )
                    .on_hover_text("Edit hardware and input settings.")
                    .clicked()
                {
                    self.navigate_player(PlayerScreen::Configuration);
                }
                ui.separator();
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("Return to session").strong())
                            .selected(true),
                    )
                    .on_hover_text("Open the current machine with its disks and progress.")
                    .clicked()
                {
                    self.navigate_player(PlayerScreen::Session);
                }
            });
        });
        if self.player.screen == PlayerScreen::Configuration {
            self.render_configuration_actions(ui);
        }
        egui::CentralPanel::default().show(ui, |ui| {
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
        self.render_binding_recording(ui.ctx());
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
            #[cfg(not(target_arch = "wasm32"))]
            ui.label("Choose a library or create a configuration in Configurations.");
            return;
        };
        ui.horizontal_wrapped(|ui| {
            if ui.button("Select Kickstart...").clicked() {
                self.open_setup_asset(None);
            }
            let stored_rom =
                (!configuration.launch.rom.is_empty()).then_some(configuration.launch.rom.as_str());
            #[cfg(not(target_arch = "wasm32"))]
            let stored_rom = self
                .player
                .selected_rom(&configuration)
                .and(self.player.rom_name.as_deref())
                .or(stored_rom);
            ui.label(stored_rom.unwrap_or("Kickstart not selected"));
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
                        110.0,
                        "Empty",
                    );
                    if ui.button("Select disk...").clicked() {
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
        egui::Modal::new(egui::Id::unique("replace-player-session")).show(ctx, |ui| {
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

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn render_missing_configuration(&mut self, ctx: &egui::Context) {
        let Some(repair) = &mut self.player.missing_configuration else {
            return;
        };
        let mut save = false;
        let mut cancel = false;
        let mut configure = false;
        let dialog =
            egui::Modal::new(egui::Id::unique("missing-game-configuration")).show(ctx, |ui| {
                ui.set_max_width((ctx.viewport_rect().width() - 40.0).min(430.0));
                ui.heading("Configuration missing");
                ui.label(format!(
                    "{} needs an existing configuration from this library.",
                    repair.game.manifest.title
                ));
                if self.player.catalog.configurations.is_empty() {
                    ui.label("Create a configuration for this library before starting the game.");
                    configure = ui.button("Open Configurations").clicked();
                } else {
                    choose_configuration(
                        ui,
                        "Game configuration",
                        &mut repair.replacement,
                        self.player.catalog.configurations.iter(),
                    );
                }
                if let Some(error) = &repair.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.horizontal_wrapped(|ui| {
                    save = ui
                        .add_enabled(
                            !repair.replacement.is_empty(),
                            egui::Button::new("Save reference and continue"),
                        )
                        .clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if configure {
            self.player.missing_configuration = None;
            self.navigate_player(PlayerScreen::Configuration);
        } else if cancel || dialog.should_close() {
            self.player.missing_configuration = None;
        } else if save {
            self.repair_game_configuration();
        }
    }

    pub(crate) fn render_player_controls(&mut self, ui: &mut egui::Ui) {
        let available_width = ui.available_width();
        if available_width >= SIDE_PANEL_MIN_WINDOW_WIDTH {
            // eframe persists this Panel's size in egui memory, alongside other UI preferences.
            egui::Panel::right("game_controls")
                .default_size(SIDE_PANEL_DEFAULT_WIDTH)
                .size_range(SIDE_PANEL_MIN_WIDTH..=SIDE_PANEL_MAX_WIDTH.min(available_width * 0.45))
                .resizable(true)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    self.render_player_tabs(ui);
                });
        } else {
            let max_content_height = ui.available_height() * 0.55;
            egui::Panel::bottom("compact_game_controls").show(ui, |ui| {
                egui::CollapsingHeader::new("Game controls").show(ui, |ui| {
                    // A bottom panel initially has only its collapsed height. Give
                    // the scroll area room to measure its body and grow the panel.
                    ui.set_max_height(max_content_height);
                    self.render_player_tabs(ui);
                });
            });
        }
    }

    fn render_player_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(!self.player.show_instructions, "Controls")
                .on_hover_text("Show disk and session controls.")
                .clicked()
            {
                self.player.show_instructions = false;
            }
            if ui
                .selectable_label(self.player.show_instructions, "Instructions")
                .on_hover_text("Read the built-in player guide while keeping this session open.")
                .clicked()
            {
                self.player.show_instructions = true;
            }
        });
        ui.separator();
        let tab = if self.player.show_instructions {
            "instructions"
        } else {
            "controls"
        };
        egui::ScrollArea::vertical()
            .id_salt(tab)
            .max_height(ui.available_height())
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if self.player.show_instructions {
                    if ui.button("Open Configurations")
                        .on_hover_text("Pause the session and open the editor for the actual keyboard and controller bindings.")
                        .clicked()
                    {
                        self.navigate_player(PlayerScreen::Configuration);
                    }
                    render_player_guide(ui);
                } else {
                    self.player_controls(ui);
                }
            });
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn render_saved_progress(&mut self, ctx: &egui::Context) {
        if !self.player.show_saves || self.editing_start_state_name() || self.file_error.is_some() {
            return;
        }
        let close = egui::Modal::new(egui::Id::unique(SAVED_PROGRESS_DIALOG)).show(ctx, |ui| {
            ui.heading("Saved progress");
            egui::ScrollArea::vertical()
                .max_height(ctx.viewport_rect().height() * 0.7)
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
        let label_width = ui
            .painter()
            .layout_no_wrap(
                "DF0:".into(),
                egui::TextStyle::Body.resolve(ui.style()),
                ui.visuals().text_color(),
            )
            .size()
            .x;
        let row_height = ui.spacing().interact_size.y;
        let activity_width = ui.text_style_height(&egui::TextStyle::Body);
        for (drive, label) in DRIVE_LABELS.into_iter().enumerate() {
            let status = self.host.status.drives[drive];
            ui.horizontal(|ui| {
                let drive_label = ui.add_sized([label_width, row_height], egui::Label::new(label));
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
                        let width = ui.available_width();
                        disk_dropdown(
                            ui,
                            ("game_drive", drive),
                            &mut selected,
                            &self.player.disks,
                            width,
                            if status.inserted {
                                "Snapshot media"
                            } else {
                                "Empty"
                            },
                        )
                        .labelled_by(drive_label.id);
                    });
                    if selected != self.player.mounted[drive] {
                        self.mount_player_disk(drive, selected);
                    }
                } else {
                    ui.allocate_space(egui::vec2(activity_width, row_height));
                    ui.label("Disconnected").on_hover_text(
                        "This drive is disabled in the active hardware configuration.",
                    );
                }
            });
        }
        ui.separator();
        let execution_label = if self.host.status.running {
            "Pause"
        } else {
            "Resume"
        };
        if self
            .shortcut_tooltip(
                ui.add_enabled(
                    !self.player.busy && !self.host.pause_requested() && self.host.can_edit(),
                    egui::Button::new(execution_label),
                ),
                KeyCommand::Pause,
                self.execution_help(),
            )
            .on_disabled_hover_text("Wait for the current operation to finish.")
            .clicked()
        {
            self.apply_key_command(KeyCommand::Pause, ui.ctx());
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
        ui.horizontal_wrapped(|ui| {
            if library_game {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.spacing_mut().button_padding.x = 4.0;
            }
            let save = ui.add_enabled(!self.player.busy, egui::Button::new("Save"));
            let description = "Capture the current machine and preview. For desktop library games, create a new save.";
            let save = if library_game {
                save.on_hover_text(description)
            } else {
                self.shortcut_tooltip(save, KeyCommand::SaveState, description)
            };
            if save.on_disabled_hover_text("Wait for the current operation to finish.").clicked() {
                self.save_player(None);
            }
            if self.shortcut_tooltip(
                ui.add_enabled(!self.player.busy, egui::Button::new("Load")),
                KeyCommand::LoadState,
                "Choose saved progress for this game, or import a snapshot file in manual mode.",
            ).on_disabled_hover_text("Wait for the current operation to finish.").clicked() {
                self.load_player();
            }
            #[cfg(not(target_arch = "wasm32"))]
            if library_game {
                self.render_capture_buttons(ui);
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
        if ui
            .button("Library")
            .on_hover_text("Pause this session and browse the library.")
            .clicked()
        {
            self.navigate_player(PlayerScreen::Library);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render_capture_buttons(&mut self, ui: &mut egui::Ui) {
        let font = egui::TextStyle::Button.resolve(ui.style());
        let text_width = ["Screenshot", "Cover"]
            .into_iter()
            .map(|label| {
                ui.painter()
                    .layout_no_wrap(label.into(), font.clone(), ui.visuals().text_color())
                    .size()
                    .x
                    .ceil()
            })
            .sum::<f32>();
        let width = text_width + 4.0 * ui.spacing().button_padding.x + ui.spacing().item_spacing.x;
        ui.allocate_ui_with_layout(
            egui::vec2(width, ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                if ui
                    .push_id("amiga-game-screenshot-button", |ui| {
                        ui.button("Screenshot")
                            .on_hover_text(
                                "Save the displayed Amiga image to this game's Screenshots folder.",
                            )
                            .clicked()
                    })
                    .inner
                {
                    self.take_game_screenshot();
                }
                if ui
                    .push_id("amiga-game-cover-button", |ui| {
                        ui.button("Cover")
                            .on_hover_text(
                                "Use the displayed Amiga image as the active game's library cover.",
                            )
                            .clicked()
                    })
                    .inner
                {
                    self.capture_library_cover();
                }
            },
        );
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
                format!("Some saved progress is unavailable: {error}"),
            );
        }
        let mut load = None;
        let mut promote = None;
        let available = active || self.player.library_available();
        for save in saves {
            ui.horizontal(|ui| {
                preview(ui, ("save", &save.path), &save.header.screenshot_png, 110.0);
                if ui
                    .add_enabled(
                        available,
                        egui::Button::new(format!(
                            "Load state - {}",
                            crate::host_clock::format_utc(save.header.timestamp)
                        )),
                    )
                    .clicked()
                {
                    load = Some(save.path.clone());
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        available && !self.player.busy,
                        egui::Button::new("Add to start states"),
                    )
                    .on_hover_text("Copy this snapshot to the game's named start states.")
                    .clicked()
                {
                    promote = Some((save.path.clone(), save.quicksave));
                }
                if save.quicksave {
                    ui.label("Quicksave");
                }
            });
        }
        if load.is_none() && promote.is_none() {
            return;
        }
        let game = if active {
            self.player.active_game.clone()
        } else {
            self.player.selected_entry().cloned()
        };
        if let Some(game) = game {
            if let Some(path) = load {
                self.request_player_launch(crate::player::PendingLaunch::Library {
                    game,
                    save: Some(path),
                    normal_boot: false,
                });
                self.player.show_saves = false;
            } else if let Some((source, quicksave)) = promote {
                self.begin_start_state_promotion(game, source, quicksave);
            }
        }
    }
}

fn disk_dropdown(
    ui: &mut egui::Ui,
    id: impl egui::AsIdSalt,
    mounted: &mut Option<String>,
    disks: &[SessionDisk],
    width: f32,
    empty_label: &'static str,
) -> egui::Response {
    let label = mounted
        .as_ref()
        .and_then(|id| disks.iter().find(|disk| &disk.id == id))
        .map_or_else(
            || {
                if mounted.is_some() {
                    "Snapshot media"
                } else {
                    empty_label
                }
            },
            |disk| disk.label.as_str(),
        );
    egui::ComboBox::from_id_salt(id)
        .width(width)
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
        })
        .response
        .on_hover_ui(|ui| {
            ui.label(label);
            ui.label("Choose a disk to insert or eject.");
        })
}

fn render_player_guide(ui: &mut egui::Ui) {
    ui.set_max_width(ui.available_width().min(540.0));
    let mut paragraph = String::new();
    for line in PLAYER_GUIDE.lines().chain([""]) {
        if line.is_empty() || line.starts_with('#') {
            if !paragraph.is_empty() {
                ui.label(std::mem::take(&mut paragraph));
                ui.add_space(4.0);
            }
            if let Some(title) = line.strip_prefix("# ") {
                ui.heading(title);
            } else if let Some(title) = line.strip_prefix("## ") {
                ui.strong(title);
            }
        } else {
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(line);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn preview(ui: &mut egui::Ui, id: impl egui::AsIdSalt, bytes: &[u8], width: f32) -> bool {
    draw_image_preview(ui, id, egui::util::hash(bytes), width, || {
        crate::library_edit::decode_artwork(bytes)
            .ok()
            .map(std::borrow::Cow::Owned)
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn draw_image_preview<'a>(
    ui: &mut egui::Ui,
    id: impl egui::AsIdSalt,
    revision: u64,
    width: f32,
    image: impl FnOnce() -> Option<std::borrow::Cow<'a, image::RgbaImage>>,
) -> bool {
    type Preview = (u64, Option<egui::TextureHandle>);
    let id = ui.scope_id().with(id);
    let cached = ui.data(|data| data.get_temp::<Preview>(id));
    let texture = if let Some((_, texture)) = cached.filter(|(current, _)| *current == revision) {
        texture
    } else {
        let texture = image().map(|image| {
            let max_edge =
                u32::try_from(ui.input(|input| input.max_texture_side)).unwrap_or(u32::MAX);
            let image = if image.width().max(image.height()) > max_edge {
                std::borrow::Cow::Owned(
                    image::DynamicImage::ImageRgba8(image.into_owned())
                        .thumbnail(max_edge, max_edge)
                        .into_rgba8(),
                )
            } else {
                image
            };
            let size = [image.width() as usize, image.height() as usize];
            ui.ctx().load_texture(
                format!("{id:?}"),
                egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                egui::TextureOptions::LINEAR,
            )
        });
        ui.data_mut(|data| data.insert_temp(id, (revision, texture.clone())));
        texture
    };
    let Some(texture) = texture else {
        return false;
    };
    ui.add(egui::Image::new(&texture).max_size(egui::vec2(ui.available_width().min(width), width)));
    true
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

#[cfg(not(target_arch = "wasm32"))]
pub fn choose_configuration<'a>(
    ui: &mut egui::Ui,
    label: &'static str,
    selected: &mut String,
    choices: impl Iterator<Item = &'a crate::player_config::NamedConfiguration> + Clone,
) {
    let name = choices
        .clone()
        .find(|configuration| configuration.id == *selected)
        .map_or("Choose a configuration", |configuration| {
            configuration.name.as_str()
        });
    egui::ComboBox::new(label, label)
        .selected_text(name)
        .truncate()
        .show_ui(ui, |ui| {
            for configuration in choices {
                ui.selectable_value(selected, configuration.id.clone(), &configuration.name);
            }
        });
}
