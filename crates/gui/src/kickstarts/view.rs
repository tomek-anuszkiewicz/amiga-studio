//! Kickstart choices and removal in the player frontend.

use super::Entry;
use crate::player::PlayerScreen;
use crate::EmulatorApp;

const DELETE_DESCRIPTION: &str =
    "Saved state files may still need these exact Kickstart bytes from another source.";

fn choice(entries: &[Entry], position: usize) -> String {
    let entry = &entries[position];
    #[cfg(not(target_arch = "wasm32"))]
    if entry.release.is_some() {
        return format!("{} ({})", entry.name(), entry.filename);
    }
    if entries
        .iter()
        .filter(|other| other.name() == entry.name())
        .count()
        > 1
    {
        return format!("{} ({})", entry.name(), position + 1);
    }
    entry.name().to_owned()
}

fn describe(ui: &mut egui::Ui, entry: &Entry) {
    let status = ui.label(entry.status());
    if let Some(release) = entry.release {
        status.on_hover_text(format!(
            "Part {} · SHA-1 {} · CRC32 {}",
            release.part, release.sha1, release.crc32
        ));
    }
    ui.label(entry.support());
}

pub fn selector(
    ui: &mut egui::Ui,
    reference: &mut String,
    entries: &[Entry],
    #[cfg(not(target_arch = "wasm32"))] legacy: &[Entry],
) {
    let position = entries
        .iter()
        .position(|entry| entry.reference == *reference);
    #[cfg(not(target_arch = "wasm32"))]
    let old = legacy.iter().find(|entry| entry.reference == *reference);
    #[cfg(target_arch = "wasm32")]
    let old: Option<&Entry> = None;
    let selected = position
        .map(|position| choice(entries, position))
        .unwrap_or_else(|| {
            old.map(|entry| entry.name().to_owned()).unwrap_or_else(|| {
                if reference.is_empty() {
                    "Choose Kickstart".into()
                } else {
                    #[cfg(target_arch = "wasm32")]
                    let name = reference
                        .split_once('/')
                        .map_or(reference.as_str(), |(_, name)| name);
                    #[cfg(not(target_arch = "wasm32"))]
                    let name = reference.as_str();
                    format!("Unavailable: {name}")
                }
            })
        });
    ui.horizontal_wrapped(|ui| {
        let label = ui.label("Kickstart");
        egui::ComboBox::from_id_salt("kickstart")
            .selected_text(selected)
            .show_ui(ui, |ui| {
                ui.selectable_value(reference, String::new(), "None");
                for (position, entry) in entries.iter().enumerate() {
                    ui.horizontal_wrapped(|ui| {
                        ui.selectable_value(
                            reference,
                            entry.reference.clone(),
                            choice(entries, position),
                        );
                        describe(ui, entry);
                    });
                }
                if let Some(entry) = old {
                    ui.selectable_value(reference, entry.reference.clone(), &entry.filename);
                }
            })
            .response
            .labelled_by(label.id);
        if let Some(entry) = position.map(|position| &entries[position]).or(old) {
            describe(ui, entry);
        } else if entries.is_empty() {
            ui.label("Add a Kickstart in Configurations.");
        }
    });
}

impl EmulatorApp {
    pub(crate) fn render_kickstart_actions(&mut self, ui: &mut egui::Ui) {
        if self.player.screen != PlayerScreen::Configuration {
            return;
        }
        let editable = self.player.editor.as_ref().is_some_and(|editor| {
            #[cfg(not(target_arch = "wasm32"))]
            if editor.library_root != self.player.library_root || !self.player.library_available() {
                return false;
            }
            !editor.saving
        });
        if ui
            .add_enabled(editable, egui::Button::new("Add Kickstart"))
            .clicked()
        {
            self.open_file_dialog(crate::file_actions::FileAction::AddKickstart);
        }
        if ui
            .add_enabled(
                editable && !self.player.kickstarts.is_empty(),
                egui::Button::new("Manage Kickstarts"),
            )
            .clicked()
        {
            self.player.manage_kickstarts = true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if ui
            .button("Supply Kickstart for restore")
            .on_hover_text(
                "Select an external Kickstart to use when importing or opening a saved state.",
            )
            .clicked()
        {
            self.open_file_dialog(crate::file_actions::FileAction::RestoreKickstart);
        }
    }

    pub(crate) fn render_kickstart_dialogs(&mut self, ctx: &egui::Context) {
        if self.player.screen != PlayerScreen::Configuration {
            return;
        }
        if let Some(reference) = self.player.delete_kickstart.clone() {
            let Some(entry) = self
                .player
                .kickstarts
                .iter()
                .find(|entry| entry.reference == reference)
            else {
                self.player.delete_kickstart = None;
                return;
            };
            let mut confirm = false;
            let mut cancel = false;
            let dialog = egui::Modal::new(egui::Id::unique("delete-kickstart")).show(ctx, |ui| {
                ui.set_max_width((ctx.viewport_rect().width() - 40.0).min(420.0));
                ui.heading("Delete Kickstart?");
                #[cfg(not(target_arch = "wasm32"))]
                ui.label(format!(
                    "Delete the managed copy of {} from this library?",
                    entry.filename
                ));
                #[cfg(target_arch = "wasm32")]
                ui.label(format!("Remove {} from this page?", entry.filename));
                ui.label(DELETE_DESCRIPTION);
                ui.horizontal(|ui| {
                    confirm = ui.button("Confirm deletion").clicked();
                    cancel = ui.button("Cancel deletion").clicked();
                });
            });
            if confirm {
                self.player.delete_kickstart = None;
                if let Err(error) = self.delete_catalog_kickstart(&reference) {
                    self.configuration_error(error);
                }
            } else if cancel || dialog.should_close() {
                self.player.delete_kickstart = None;
            }
            return;
        }
        if !self.player.manage_kickstarts {
            return;
        }
        let mut remove = None;
        let dialog = egui::Modal::new(egui::Id::unique("manage-kickstarts")).show(ctx, |ui| {
            ui.set_max_width((ctx.viewport_rect().width() - 40.0).min(550.0));
            ui.heading("Kickstarts");
            if let Some(error) = self
                .player
                .editor
                .as_ref()
                .and_then(|editor| editor.error.as_deref())
            {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            egui::ScrollArea::vertical()
                .max_height(ctx.viewport_rect().height() * 0.6)
                .show(ui, |ui| {
                    for (position, entry) in self.player.kickstarts.iter().enumerate() {
                        ui.push_id(&entry.reference, |ui| {
                            ui.strong(choice(&self.player.kickstarts, position));
                            ui.horizontal_wrapped(|ui| describe(ui, entry));
                            if ui.button("Delete Kickstart").clicked() {
                                remove = Some(entry.reference.clone());
                            }
                            ui.separator();
                        });
                    }
                });
            if ui.button("Done").clicked() {
                self.player.manage_kickstarts = false;
            }
        });
        if dialog.should_close() {
            self.player.manage_kickstarts = false;
        }
        if let Some(reference) = remove {
            match self.kickstart_can_be_deleted(&reference) {
                Ok(()) => {
                    if let Some(editor) = &mut self.player.editor {
                        editor.error = None;
                    }
                    self.player.delete_kickstart = Some(reference);
                }
                Err(error) => self.configuration_error(error),
            }
        }
    }

    fn kickstart_can_be_deleted(&self, reference: &str) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        let (catalog, matches) = {
            let root = self
                .player
                .library_root
                .as_deref()
                .ok_or("Choose a library")?;
            let path = super::native::managed_path(root, reference)?;
            let (catalog, _) = crate::library_catalog::read(root, &self.player.catalog.selected)?;
            (catalog, move |candidate: &str| {
                super::native::references(root, candidate, &path)
            })
        };
        #[cfg(target_arch = "wasm32")]
        let (catalog, matches) = (&self.player.catalog, |candidate: &str| {
            candidate == reference
        });
        let users: Vec<_> = catalog
            .configurations
            .iter()
            .filter(|configuration| matches(&configuration.launch.rom))
            .map(|configuration| configuration.name.as_str())
            .collect();
        if !users.is_empty() {
            return Err(format!("Kickstart is used by saved configurations: {}. Change their Kickstart choices and save first.", users.join(", ")));
        }
        if self.player.editor.as_ref().is_some_and(|editor| {
            editor
                .catalog
                .configurations
                .iter()
                .any(|configuration| matches(&configuration.launch.rom))
        }) {
            return Err("A configuration draft uses this Kickstart. Change its choice or discard the draft first.".into());
        }
        Ok(())
    }

    fn delete_catalog_kickstart(&mut self, reference: &str) -> Result<(), String> {
        self.kickstart_can_be_deleted(reference)?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let root = self
                .player
                .library_root
                .as_deref()
                .ok_or("Choose a library")?;
            let path = super::native::managed_path(root, reference)?;
            let result = std::fs::remove_file(&path)
                .map_err(|error| format!("Could not delete {}: {error}", path.display()));
            self.refresh_kickstarts();
            result?;
        }
        #[cfg(target_arch = "wasm32")]
        self.player
            .kickstarts
            .retain(|entry| entry.reference != reference);
        if self.player.manual_kickstart.as_deref() == Some(reference) {
            self.player.manual_kickstart = None;
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn refresh_kickstarts(&mut self) {
        self.player.legacy_kickstarts.clear();
        self.player.kickstarts.clear();
        self.player.kickstart_error = None;
        let Some(root) = self.player.library_root.as_deref() else {
            return;
        };
        match super::native::entries(root) {
            Ok((entries, errors)) => {
                self.player.kickstarts = entries;
                self.player.kickstart_error = (!errors.is_empty()).then(|| errors.join("\n"));
            }
            Err(error) => self.player.kickstart_error = Some(error),
        }
        for profile in &self.player.catalog.configurations {
            let reference = &profile.launch.rom;
            if super::native::managed_path(root, reference).is_ok()
                || self
                    .player
                    .legacy_kickstarts
                    .iter()
                    .any(|entry| &entry.reference == reference)
            {
                continue;
            }
            let entry = crate::library_catalog::rom_path(root, reference)
                .and_then(|path| super::native::entry(reference.clone(), &path));
            if let Ok(entry) = entry {
                self.player.legacy_kickstarts.push(entry);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_kickstart_file(&mut self, path: &std::path::Path) -> Result<(), String> {
        if !self.player.library_available()
            || self
                .player
                .editor
                .as_ref()
                .is_some_and(|editor| editor.library_root != self.player.library_root)
        {
            return Err("Open a library in Configurations before adding a Kickstart".into());
        }
        let root = self
            .player
            .library_root
            .as_deref()
            .ok_or("Choose a library")?;
        super::native::import(root, path)?;
        self.remember_kickstart_import(path);
        self.refresh_kickstarts();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn supply_browser_kickstart(
        &mut self,
        name: String,
        bytes: Vec<u8>,
    ) -> Result<(), String> {
        let entry = Entry::browser(name, bytes)?;
        if !self
            .player
            .kickstarts
            .iter()
            .any(|existing| existing.reference == entry.reference)
        {
            self.player.kickstarts.push(entry);
        }
        Ok(())
    }
}
