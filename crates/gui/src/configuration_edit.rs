//! Catalog transaction and stopped owner coordination for the configuration screen.

use crate::app_host::HostEffect;
use crate::host_commands::{HostCommand, HostValue};
use crate::player::PlayerScreen;
use crate::player_config::ConfigurationCatalog;
use crate::{EmulatorApp, ViewMode};

#[derive(Debug)]
pub struct ConfigurationEdit {
    pub catalog: ConfigurationCatalog,
    origin: PlayerScreen,
    origin_mode: ViewMode,
    pub(crate) destination: Option<PlayerScreen>,
    pub(crate) destination_developer: bool,
    pub(crate) saving: bool,
    ready: bool,
    pub(crate) error: Option<String>,
}

impl EmulatorApp {
    pub(crate) fn render_configuration_actions(&mut self, ctx: &egui::Context) {
        let saving = self
            .player
            .editor
            .as_ref()
            .is_some_and(|editor| editor.saving);
        egui::TopBottomPanel::bottom("configuration_actions").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        !saving && !self.host.status.running && self.host.can_edit(),
                        egui::Button::new("Save"),
                    )
                    .on_hover_text(
                        "Validate and save all edits; update session controls while paused.",
                    )
                    .clicked()
                {
                    self.save_configuration_edit();
                }
                if ui
                    .add_enabled(!saving, egui::Button::new("Cancel"))
                    .on_hover_text("Discard all edits and return to the previous screen.")
                    .clicked()
                {
                    self.cancel_configuration_edit();
                }
                ui.label(if saving {
                    "Saving controls..."
                } else {
                    "Launch changes apply next time. Controls apply to this session."
                });
            });
            if let Some(error) = self
                .player
                .editor
                .as_ref()
                .and_then(|editor| editor.error.as_ref())
            {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        });
        if self
            .player
            .editor
            .as_ref()
            .is_none_or(|editor| editor.destination.is_none() || editor.saving)
        {
            return;
        }
        let mut save = false;
        let mut discard = false;
        let mut keep = false;
        egui::Modal::new(egui::Id::new("unsaved-configuration")).show(ctx, |ui| {
            ui.set_max_width((ctx.screen_rect().width() - 40.0).min(420.0));
            ui.heading("Unsaved configuration changes");
            ui.label("Save all edits before leaving, discard them, or keep editing.");
            ui.horizontal_wrapped(|ui| {
                save = ui.button("Save").clicked();
                discard = ui.button("Discard").clicked();
                keep = ui.button("Keep editing").clicked();
            });
        });
        if save {
            self.save_configuration_edit();
        } else if discard {
            self.discard_configuration_navigation();
        } else if keep {
            if let Some(editor) = &mut self.player.editor {
                editor.destination = None;
                editor.destination_developer = false;
            }
        }
    }

    pub(crate) fn begin_configuration_edit(&mut self) {
        self.player.editor = Some(ConfigurationEdit {
            catalog: self.player.catalog.clone(),
            origin: self.player.screen,
            origin_mode: self.view_mode,
            destination: None,
            destination_developer: false,
            saving: false,
            ready: false,
            error: None,
        });
    }

    pub(crate) fn guard_configuration_navigation(&mut self, screen: PlayerScreen) -> bool {
        let Some(editor) = &mut self.player.editor else {
            return false;
        };
        if editor.saving {
            return true;
        }
        if screen == PlayerScreen::Configuration {
            return false;
        }
        if editor.catalog != self.player.catalog {
            editor.destination = Some(screen);
            return true;
        }
        false
    }

    pub fn cancel_configuration_edit(&mut self) {
        if self
            .player
            .editor
            .as_ref()
            .is_some_and(|editor| editor.saving)
        {
            return;
        }
        if let Some(editor) = self.player.editor.take() {
            self.player.screen = editor.origin;
            self.set_view_mode(editor.origin_mode);
        }
    }

    pub(crate) fn discard_configuration_navigation(&mut self) {
        if let Some(editor) = self.player.editor.take() {
            self.finish_configuration_navigation(editor);
        }
    }

    fn finish_configuration_navigation(&mut self, mut editor: ConfigurationEdit) {
        let screen = editor.destination.unwrap_or(editor.origin);
        let mode = if editor.destination_developer {
            ViewMode::Developer
        } else if editor.destination.is_some() {
            ViewMode::ScreenOnly
        } else {
            editor.origin_mode
        };
        // A failed session-entry validation keeps a clean editor with its
        // original Cancel destination, rather than opening an editor from itself.
        editor.catalog = self.player.catalog.clone();
        editor.destination = None;
        editor.destination_developer = false;
        editor.saving = false;
        editor.ready = false;
        editor.error = None;
        self.player.editor = Some(editor);
        self.enter_player_screen(screen, mode);
    }

    pub fn save_configuration_edit(&mut self) {
        let Some(editor) = &self.player.editor else {
            return;
        };
        if editor.saving {
            return;
        }
        let validation = (|| {
            editor.catalog.validate_all()?;
            if let Some(active) = &self.player.active_configuration {
                let configuration = editor
                    .catalog
                    .configurations
                    .iter()
                    .find(|configuration| configuration.id == active.id)
                    .ok_or("The active input configuration is missing")?;
                self.host_sources_available(&configuration.controls)?;
            }
            if self.host.status.running || !self.host.can_edit() {
                return Err("Wait for the current machine to pause before saving".into());
            }
            if self.host.error.is_some() || self.host.status.shutdown {
                return Err("The machine owner is unavailable".into());
            }
            Ok(())
        })();
        if let Err(error) = validation {
            if let Some(editor) = &mut self.player.editor {
                editor.error = Some(error);
            }
            return;
        }
        let Some(editor) = &mut self.player.editor else {
            return;
        };
        editor.error = None;
        editor.saving = true;
        let controls = self
            .player
            .active_configuration
            .as_ref()
            .and_then(|active| {
                editor
                    .catalog
                    .configurations
                    .iter()
                    .find(|c| c.id == active.id)
            });
        if let Some(configuration) = controls.filter(|configuration| {
            self.player
                .active_configuration
                .as_ref()
                .is_some_and(|active| active.controls != configuration.controls)
                || self.player.catalog.mappings != editor.catalog.mappings
        }) {
            self.host.queue_effect(
                HostCommand::ReconfigurePorts(
                    configuration.controls.ports.each_ref().map(|p| p.device),
                ),
                HostEffect::Configuration,
            );
        } else {
            editor.ready = true;
        }
    }

    pub(crate) fn complete_configuration_save(&mut self, result: Result<HostValue, String>) {
        if let Some(editor) = &mut self.player.editor {
            match result {
                Ok(_) => editor.ready = true,
                Err(error) => {
                    editor.saving = false;
                    editor.error = Some(error);
                }
            }
        }
    }

    pub(crate) fn poll_configuration_save(&mut self, ctx: &egui::Context) {
        if self.host.error.is_some() {
            if let Some(editor) = &mut self.player.editor {
                if editor.saving {
                    editor.saving = false;
                    editor.ready = false;
                    editor.error = Some("The machine owner is unavailable".into());
                }
            }
        }
        let Some(editor) = &self.player.editor else {
            return;
        };
        if !editor.ready {
            return;
        }
        if let Some(active) = &self.player.active_configuration {
            if let Some(configuration) = editor
                .catalog
                .configurations
                .iter()
                .find(|c| c.id == active.id)
            {
                if let Some(runtime) = &self.host.runtime {
                    if !runtime
                        .input_sender()
                        .configure(&configuration.controls, &editor.catalog.mappings)
                    {
                        ctx.request_repaint_after(crate::host_runtime::HOST_RETRY_DELAY);
                        return;
                    }
                }
            }
        }
        let Some(editor) = self.player.editor.take() else {
            return;
        };
        if let Some(active) = &mut self.player.active_configuration {
            if let Some(configuration) = editor
                .catalog
                .configurations
                .iter()
                .find(|c| c.id == active.id)
            {
                active.controls = configuration.controls.clone();
            }
        }
        self.player.settings_dirty |= editor.catalog != self.player.catalog;
        self.player.catalog = editor.catalog.clone();
        self.file_error = None;
        self.finish_configuration_navigation(editor);
    }
}
