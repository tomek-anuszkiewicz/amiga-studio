//! Catalog transaction and stopped owner coordination for the configuration screen.

use crate::app_host::HostEffect;
use crate::configuration_bindings::BindingRecording;
use crate::host_commands::{HostCommand, HostValue};
use crate::input_mapping::GamepadRecording;
use crate::player::PlayerScreen;
use crate::player_config::ConfigurationCatalog;
use crate::{EmulatorApp, ViewMode};
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[derive(Debug)]
pub struct ConfigurationEdit {
    pub catalog: ConfigurationCatalog,
    baseline: ConfigurationCatalog,
    #[cfg(not(target_arch = "wasm32"))]
    pub library_root: Option<PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    source: Option<Vec<u8>>,
    #[cfg(not(target_arch = "wasm32"))]
    write: Option<crate::library_catalog::CatalogWrite>,
    #[cfg(not(target_arch = "wasm32"))]
    owner_applied: bool,
    #[cfg(not(target_arch = "wasm32"))]
    rollback_error: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) deletion: Option<crate::configuration_delete::ConfigurationDeletion>,
    pub(crate) recording: Option<BindingRecording>,
    pub(crate) mapping_controller: Option<String>,
    pub(crate) new_guest_key: u8,
    pub(crate) expanded_mapping: Option<String>,
    origin: PlayerScreen,
    origin_mode: ViewMode,
    pub(crate) destination: Option<PlayerScreen>,
    pub(crate) destination_developer: bool,
    pub(crate) saving: bool,
    ready: bool,
    pub(crate) error: Option<String>,
}

impl ConfigurationEdit {
    pub(crate) fn gamepad_recording(&self) -> Option<&GamepadRecording> {
        self.recording
            .as_ref()
            .and_then(|recording| recording.controller.as_ref())
    }
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
            #[cfg(not(target_arch = "wasm32"))]
            self.render_configuration_deletion(ctx);
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
            baseline: self.player.catalog.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            library_root: self.player.library_root.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            source: self.player.catalog_source.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            write: None,
            #[cfg(not(target_arch = "wasm32"))]
            owner_applied: false,
            #[cfg(not(target_arch = "wasm32"))]
            rollback_error: None,
            #[cfg(not(target_arch = "wasm32"))]
            deletion: None,
            recording: None,
            mapping_controller: None,
            new_guest_key: crate::host_keys::SCANCODE_ESCAPE,
            expanded_mapping: None,
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
        let dirty = editor.catalog != editor.baseline;
        #[cfg(not(target_arch = "wasm32"))]
        let dirty = dirty || editor.library_root != self.player.library_root;
        if dirty {
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
        editor.baseline = self.player.catalog.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            editor.library_root = self.player.library_root.clone();
            editor.source = self.player.catalog_source.clone();
            editor.write = None;
            editor.owner_applied = false;
            editor.rollback_error = None;
            editor.deletion = None;
        }
        editor.recording = None;
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
        #[cfg(not(target_arch = "wasm32"))]
        {
            if editor.library_root != self.player.library_root {
                self.save_library_root_edit();
                return;
            }
            if editor.catalog != editor.baseline && !self.player.library_available() {
                let error = self.player.library_error.as_ref().map_or_else(
                    || "Choose an accessible library directory before saving profiles".into(),
                    |error| format!("Could not prepare configuration save: {error}"),
                );
                self.configuration_error(error);
                return;
            }
        }
        let active = self.configuration_being_edited();
        let validation = (|| {
            editor.catalog.validate_all()?;
            if let Some(active) = self
                .player
                .active_configuration
                .as_ref()
                .filter(|_| self.player.is_active_library())
            {
                for mapping in &self.player.active_mappings {
                    if active.controls.uses_mapping(&mapping.id)
                        && editor.baseline.mapping(&mapping.id).is_some()
                        && editor.catalog.mapping(&mapping.id).is_none()
                    {
                        return Err(format!(
                            "{} is still used by the active session ({}).",
                            mapping.name, active.name
                        ));
                    }
                }
            }
            if let Some(configuration) = active {
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
            self.configuration_error(error);
            return;
        }
        let ports = active
            .filter(|configuration| {
                self.player
                    .active_configuration
                    .as_ref()
                    .is_some_and(|active| active.controls != configuration.controls)
                    || self.player.active_mappings != editor.catalog.mappings
            })
            .map(|configuration| configuration.controls.ports.each_ref().map(|p| p.device));
        #[cfg(not(target_arch = "wasm32"))]
        let write = if !crate::library_catalog::same_profiles(&editor.catalog, &editor.baseline) {
            let Some(root) = &self.player.library_root else {
                return;
            };
            match crate::library_catalog::CatalogWrite::prepare(
                root,
                editor.source.as_deref(),
                &editor.catalog,
            ) {
                Ok(write) => Some(write),
                Err(error) => {
                    self.configuration_error(format!(
                        "Could not prepare configuration save: {error}"
                    ));
                    return;
                }
            }
        } else {
            None
        };
        #[cfg(not(target_arch = "wasm32"))]
        if editor.source != self.player.catalog_source {
            self.configuration_error(
                "Configurations changed on disk. Reopen Configurations before saving.".into(),
            );
            return;
        }
        let Some(editor) = &mut self.player.editor else {
            return;
        };
        editor.error = None;
        editor.saving = true;
        #[cfg(not(target_arch = "wasm32"))]
        {
            editor.write = write;
        }
        if let Some(ports) = ports {
            self.host.queue_effect(
                HostCommand::ReconfigurePorts(ports),
                HostEffect::Configuration,
            );
        } else {
            editor.ready = true;
        }
    }

    pub(crate) fn complete_configuration_save(&mut self, result: Result<HostValue, String>) {
        if let Some(editor) = &mut self.player.editor {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(error) = editor.rollback_error.take() {
                editor.saving = false;
                editor.error = Some(match result {
                    Ok(_) => error,
                    Err(owner) => {
                        format!("{error}. Previous machine ports could not be restored: {owner}")
                    }
                });
                return;
            }
            match result {
                Ok(_) => {
                    editor.ready = true;
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        editor.owner_applied = true;
                    }
                }
                Err(error) => {
                    editor.saving = false;
                    editor.error = Some(error);
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        editor.write = None;
                    }
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
                    let error = "The machine owner is unavailable".to_string();
                    #[cfg(not(target_arch = "wasm32"))]
                    let error = {
                        editor.write = None;
                        editor
                            .rollback_error
                            .take()
                            .map_or(error.clone(), |save| format!("{save}. {error}"))
                    };
                    editor.error = Some(error);
                }
            }
        }
        let Some(editor) = &self.player.editor else {
            return;
        };
        if !editor.ready {
            return;
        }
        let active = self
            .configuration_being_edited()
            .map(|configuration| configuration.controls.clone());
        if let (Some(controls), Some(runtime)) = (&active, &self.host.runtime) {
            if !runtime
                .input_sender()
                .configure(controls, &editor.catalog.mappings)
            {
                ctx.request_repaint_after(crate::host_runtime::HOST_RETRY_DELAY);
                return;
            }
        }
        let Some(editor) = self.player.editor.take() else {
            return;
        };
        #[cfg(not(target_arch = "wasm32"))]
        let mut editor = editor;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(write) = editor.write.take() {
            match write.publish() {
                Ok(source) => {
                    self.player.catalog_source = Some(source);
                    if let Some(sync) = &mut self.player.library_sync {
                        sync.changed_locally();
                    }
                }
                Err(error) => {
                    self.fail_catalog_publication(editor, error);
                    return;
                }
            }
        }
        if let (Some(controls), Some(active)) = (active, &mut self.player.active_configuration) {
            active.controls = controls;
            self.player
                .active_mappings
                .clone_from(&editor.catalog.mappings);
        }
        self.player.settings_dirty |= editor.catalog != self.player.catalog;
        self.player.catalog = editor.catalog.clone();
        self.file_error = None;
        self.finish_configuration_navigation(editor);
    }

    fn configuration_being_edited(&self) -> Option<&crate::player_config::NamedConfiguration> {
        if !self.player.is_active_library() {
            return None;
        }
        let active = self.player.active_configuration.as_ref()?;
        let editor = self.player.editor.as_ref()?;
        let baseline = editor
            .baseline
            .configurations
            .iter()
            .find(|cfg| cfg.id == active.id)?;
        let configuration = editor
            .catalog
            .configurations
            .iter()
            .find(|cfg| cfg.id == active.id)?;
        let matches_session = baseline.controls == active.controls
            && editor.baseline.mappings == self.player.active_mappings;
        let changed_controls = configuration.controls != baseline.controls
            || editor.catalog.mappings != editor.baseline.mappings;
        (matches_session || changed_controls).then_some(configuration)
    }

    pub(crate) fn configuration_error(&mut self, error: String) {
        if let Some(editor) = &mut self.player.editor {
            editor.error = Some(error);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_library_root_edit(&mut self) {
        let Some(editor) = &self.player.editor else {
            return;
        };
        if !crate::library_catalog::same_profiles(&editor.catalog, &editor.baseline) {
            self.configuration_error("Finish or discard your profile edits before switching libraries. Use Keep current directory to save them here.".into());
            return;
        }
        let target = editor
            .library_root
            .as_ref()
            .ok_or_else(|| "Choose a library directory".to_string())
            .and_then(|root| {
                crate::library_catalog::open(root, "")
                    .map(|(catalog, source)| (root.clone(), catalog, source))
            });
        match target {
            Ok((root, catalog, source)) => {
                if let Some(editor) = self.player.editor.take() {
                    self.commit_library_root(root, catalog, source);
                    self.finish_configuration_navigation(editor);
                }
            }
            Err(error) => self.configuration_error(error),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn fail_catalog_publication(&mut self, mut editor: ConfigurationEdit, error: String) {
        let error = format!("Could not save configurations: {error}");
        editor.ready = false;
        if let Some(active) = self
            .player
            .active_configuration
            .as_ref()
            .filter(|_| editor.owner_applied)
        {
            editor.rollback_error = Some(error);
            self.host.queue_effect(
                HostCommand::ReconfigurePorts(
                    active.controls.ports.each_ref().map(|port| port.device),
                ),
                HostEffect::Configuration,
            );
        } else {
            editor.saving = false;
            editor.error = Some(error);
        }
        editor.owner_applied = false;
        self.player.editor = Some(editor);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn configuration_deletion_allowed(&self) -> bool {
        self.player.library_available()
            && self.player.editor.as_ref().is_some_and(|editor| {
                !editor.saving
                    && editor.library_root == self.player.library_root
                    && editor.source == self.player.catalog_source
                    && crate::library_catalog::same_profiles(&editor.catalog, &editor.baseline)
                    && editor.catalog.selected().is_some()
            })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn refresh_clean_configuration_draft(&mut self, ctx: &egui::Context) {
        if let Some(editor) = self.player.editor.as_mut().filter(|editor| {
            !editor.saving
                && editor.deletion.is_none()
                && editor.library_root == self.player.library_root
                && editor.catalog == editor.baseline
        }) {
            if editor.recording.is_some() {
                crate::configuration_bindings::finish_recording(editor, ctx);
            }
            self.reload_configuration_draft();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn reload_configuration_draft(&mut self) {
        if let Some(editor) = &mut self.player.editor {
            editor.catalog = self.player.catalog.clone();
            editor.baseline = self.player.catalog.clone();
            editor.source = self.player.catalog_source.clone();
        }
    }
}
