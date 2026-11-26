//! Host file selection and buffer handoff to the headless session.

use crate::app_host::HostEffect;
use crate::disk_import::{DiskSelection, DISK_EXTENSIONS};
use crate::host_commands::{HostCommand, HostValue};
use crate::layout::top_menu_bar::TOAST_DURATION_FRAMES;
use crate::EmulatorApp;

#[cfg(not(target_arch = "wasm32"))]
mod history;
#[cfg(not(target_arch = "wasm32"))]
pub use history::FileHistory;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use history::STORAGE_KEY as HISTORY_STORAGE_KEY;

#[derive(Clone, Copy, Debug)]
pub(crate) enum FileAction {
    State,
    AddKickstart,
    #[cfg(not(target_arch = "wasm32"))]
    RestoreKickstart,
    SetupDisk(usize),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy)]
pub(crate) enum NativeDialog {
    Open(FileAction),
    Save,
}

impl FileAction {
    fn title(self) -> &'static str {
        match self {
            Self::State => "Load State from File",
            Self::AddKickstart => "Add Kickstart",
            #[cfg(not(target_arch = "wasm32"))]
            Self::RestoreKickstart => "Supply Kickstart for restore",
            Self::SetupDisk(_) => "Select disk for Setup",
        }
    }

    fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::State => &["json"],
            Self::AddKickstart => &["rom", "bin"],
            #[cfg(not(target_arch = "wasm32"))]
            Self::RestoreKickstart => &["rom", "bin"],
            Self::SetupDisk(_) => DISK_EXTENSIONS,
        }
    }
}

#[derive(Debug)]
pub(crate) enum FileCompletion {
    State(String),
    #[cfg(not(target_arch = "wasm32"))]
    Save(std::path::PathBuf),
    #[cfg(target_arch = "wasm32")]
    Download,
    QuickSave,
}

#[derive(Debug)]
pub(crate) struct DriveDiskChoice {
    drive: usize,
    selection: DiskSelection,
}

impl EmulatorApp {
    pub(crate) fn accept_disk_selection(&mut self, drive: usize, mut selection: DiskSelection) {
        self.player.drive_disk_choice = None;
        self.file_error = None;
        match selection.images.len() {
            0 => {}
            1 => {
                if let Some(image) = selection.images.pop() {
                    self.select_setup_disk(drive, image);
                }
            }
            _ => {
                self.player.drive_disk_choice = Some(DriveDiskChoice { drive, selection });
                return;
            }
        }
        if !selection.issues.is_empty() {
            self.file_error = Some(selection.issues.join("\n"));
        }
    }

    pub(crate) fn render_drive_disk_choice(&mut self, ctx: &egui::Context) {
        let Some(choice) = &mut self.player.drive_disk_choice else {
            return;
        };
        let mut selected = None;
        let dialog = egui::Modal::new(egui::Id::unique("choose-drive-disk")).show(ctx, |ui| {
            ui.set_max_width((ctx.viewport_rect().width() - 40.0).min(520.0));
            ui.heading(format!("Choose a disk for DF{}", choice.drive));
            egui::ScrollArea::vertical()
                .max_height(280.0)
                .show(ui, |ui| {
                    for (index, image) in choice.selection.images.iter().enumerate() {
                        if ui.button(image.description().as_ref()).clicked() {
                            selected = Some(index);
                        }
                    }
                    for issue in &choice.selection.issues {
                        ui.colored_label(ui.visuals().error_fg_color, issue);
                    }
                });
            ui.button("Cancel").clicked()
        });
        if let Some(index) = selected {
            let image = choice.selection.images.swap_remove(index);
            let drive = choice.drive;
            self.player.drive_disk_choice = None;
            self.select_setup_disk(drive, image);
        } else if dialog.inner || dialog.should_close() {
            self.player.drive_disk_choice = None;
        }
    }

    fn prepare_state_import(&self, json: String) -> Result<HostCommand, String> {
        let saved = debugger::SaveState::from_json(&json).map_err(|e| e.to_string())?;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(game) = &self.player.active_game {
            game.manifest
                .validate_association(&saved.header.association)?;
        }
        self.validate_current_input()?;
        let media = self.file_import_media(&saved)?;
        Ok(HostCommand::RestorePlayer {
            json,
            media,
            game_id: saved.header.association.game_id,
            ports: self.current_control_ports(),
            resume: false,
        })
    }

    fn file_import_media(
        &self,
        saved: &debugger::SaveState,
    ) -> Result<crate::host_commands::RestoreMedia, String> {
        let manual = saved.header.association.game_id.is_none();
        let media = self.player.restore_media(
            self.player.setup_disks.iter().chain(&self.player.disks),
            manual,
        );
        #[cfg(not(target_arch = "wasm32"))]
        let media = {
            let mut media = media;
            let game = self.player.active_game.as_ref().or_else(|| {
                self.player.library.games.iter().find(|game| {
                    Some(game.manifest.id.as_str()) == saved.header.association.game_id.as_deref()
                })
            });
            if let Some(game) = game {
                game.manifest
                    .validate_association(&saved.header.association)?;
                for disk in &game.manifest.disks {
                    let path = disk.asset_path(&game.directory)?;
                    if let Ok(bytes) = std::fs::read(path) {
                        media.disks.push((
                            disk.id.clone(),
                            debugger::SourceImage {
                                name: Some(disk.filename.clone()),
                                bytes,
                            },
                        ));
                    }
                }
            }
            let root = if self.player.active_game.is_some() {
                self.player.active_library_root.as_deref()
            } else {
                self.player.library_root.as_deref()
            };
            let configuration = game
                .and_then(|game| {
                    self.player
                        .catalog
                        .configurations
                        .iter()
                        .find(|configuration| configuration.id == game.manifest.configuration)
                })
                .or_else(|| self.current_input_configuration());
            if let Some(root) = root {
                if let Some(rom) =
                    configuration.and_then(|config| self.read_legacy_save_rom(root, config))
                {
                    media.roms.push(rom);
                }
                crate::kickstarts::native::extend_restore_roms(root, &mut media.roms);
            }
            if let Some(opened) = self
                .player
                .library_root
                .as_deref()
                .filter(|opened| Some(*opened) != root)
            {
                crate::kickstarts::native::extend_restore_roms(opened, &mut media.roms);
            }
            media
        };
        Ok(media)
    }

    pub(crate) fn validate_current_input(&self) -> Result<(), String> {
        if let Some(configuration) = self.current_input_configuration() {
            configuration
                .controls
                .validate(self.player.current_mappings())?;
            self.host_sources_available(&configuration.controls)?;
        }
        Ok(())
    }

    pub(crate) fn current_input_configuration(
        &self,
    ) -> Option<&crate::player_config::NamedConfiguration> {
        self.player
            .active_configuration
            .as_ref()
            .or_else(|| self.player.catalog.selected())
    }

    pub(crate) fn current_control_ports(&self) -> [debugger::launch::DeviceKind; 2] {
        self.current_input_configuration()
            .map_or(self.host.status.ports, |c| {
                c.controls.ports.each_ref().map(|p| p.device)
            })
    }

    pub(crate) fn open_setup_disk(&mut self, drive: usize) {
        self.player.captured = false;
        self.player.drive_disk_choice = None;
        self.open_file_dialog(FileAction::SetupDisk(drive));
    }

    pub(crate) fn open_load_state_dialog(&mut self) {
        self.open_file_dialog(FileAction::State);
    }

    pub(crate) fn complete_file_action(
        &mut self,
        action: FileCompletion,
        value: Result<HostValue, String>,
        ctx: &egui::Context,
    ) {
        if let Err(error) = value {
            self.file_error = Some(format!("Could not complete file action: {error}"));
            return;
        }
        if let Ok(HostValue::PlayerRestored(association)) = &value {
            for id in association.mounted.iter().flatten() {
                if !self.player.disks.iter().any(|disk| &disk.id == id) {
                    let selected = self.player.setup_disks.iter().find(|disk| &disk.id == id);
                    self.player.disks.push(selected.cloned().unwrap_or_else(|| {
                        crate::player::SessionDisk {
                            id: id.clone(),
                            label: id.clone(),
                            bytes: None,
                        }
                    }));
                }
            }
            self.player.mounted = association.mounted.clone();
            self.player.captured = false;
        }
        self.file_error = None;
        match action {
            FileCompletion::State(message) => {
                for disk in &self.player.setup_disks {
                    if !self.player.disks.iter().any(|entry| entry.id == disk.id) {
                        self.player.disks.push(disk.clone());
                    }
                }
                if self.player.active_configuration.is_none() {
                    self.player.activate_selected_configuration();
                }
                self.state_loaded(message);
                self.sync_session_title(ctx);
            }
            FileCompletion::QuickSave => {
                self.toast_message = Some(("Progress saved".into(), TOAST_DURATION_FRAMES));
            }
            #[cfg(not(target_arch = "wasm32"))]
            FileCompletion::Save(path) => {
                if let Ok(HostValue::Snapshot(json)) = value {
                    match std::fs::write(&path, json) {
                        Ok(()) => {
                            self.toast_message = Some((
                                format!("State saved to {}", path.display()),
                                TOAST_DURATION_FRAMES,
                            ))
                        }
                        Err(error) => {
                            self.file_error = Some(format!("Could not save state: {error}"))
                        }
                    }
                }
            }
            #[cfg(target_arch = "wasm32")]
            FileCompletion::Download => {
                if let Ok(HostValue::Snapshot(json)) = value {
                    match browser::download_state(&json) {
                        Ok(()) => {
                            self.toast_message =
                                Some(("State download requested.".into(), TOAST_DURATION_FRAMES))
                        }
                        Err(error) => {
                            self.file_error = Some(format!("Could not download state: {error:?}"))
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn accept_file_data(&mut self, action: FileAction, name: String, data: Vec<u8>) {
        match action {
            FileAction::AddKickstart => match self.supply_browser_kickstart(name, data) {
                Ok(()) => {
                    self.file_error = None;
                    if let Some(editor) = &mut self.player.editor {
                        editor.error = None;
                    }
                }
                Err(error) => {
                    self.configuration_error(error.clone());
                    self.file_error = Some(error);
                }
            },
            FileAction::SetupDisk(_) => {}
            FileAction::State => match String::from_utf8(data) {
                Ok(json) => {
                    let command = match self.prepare_state_import(json) {
                        Ok(command) => command,
                        Err(error) => {
                            self.file_error = Some(error);
                            return;
                        }
                    };
                    self.host.queue_effect(
                        command,
                        HostEffect::File(FileCompletion::State(format!(
                            "State loaded from {name}"
                        ))),
                    )
                }
                Err(error) => self.file_error = Some(format!("Could not load {name}: {error}")),
            },
        }
    }

    fn state_loaded(&mut self, message: String) {
        self.inspection.ready = false;
        self.file_loaded_machine = true;
        self.file_error = None;
        self.disassembly_view_addr = None;
        self.toast_message = Some((message, TOAST_DURATION_FRAMES));
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl EmulatorApp {
    pub(crate) fn open_file_dialog(&mut self, action: FileAction) {
        self.player.captured = false;
        self.host.keyboard_pending.clear();
        if self
            .host
            .runtime
            .as_ref()
            .is_some_and(|runtime| !runtime.input_sender().set_enabled(false, false, false))
        {
            self.player.dialog = Some(NativeDialog::Open(action));
            return;
        }
        let dialog = match action {
            FileAction::SetupDisk(_) => self.disk_file_dialog(action.title()),
            FileAction::AddKickstart | FileAction::RestoreKickstart => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title(action.title())
                    .add_filter("Kickstart", action.extensions());
                if let Some(directory) = self.file_history.rom_picker_directory() {
                    dialog = dialog.set_directory(directory);
                }
                dialog
            }
            FileAction::State => rfd::FileDialog::new()
                .set_title(action.title())
                .add_filter("Supported files", action.extensions()),
        };
        if let Some(path) = dialog.pick_file() {
            match action {
                FileAction::State => self.load_state_file(&path),
                FileAction::AddKickstart => {
                    let _ = self.import_kickstart(&path);
                }
                FileAction::RestoreKickstart => {
                    let result = self.supply_restore_kickstart(&path);
                    self.show_kickstart_file_result(&result);
                }
                FileAction::SetupDisk(drive) => self.load_setup_disk(drive, &path),
            }
        }
    }

    pub(crate) fn disk_file_dialog(&self, title: &str) -> rfd::FileDialog {
        let mut dialog = rfd::FileDialog::new()
            .set_title(title)
            .add_filter("ADF or archive", DISK_EXTENSIONS);
        if let Some(directory) = self.file_history.disk_picker_directory() {
            dialog = dialog.set_directory(directory);
        }
        dialog
    }

    pub(crate) fn remember_disk_selection(&mut self, path: &std::path::Path) {
        self.file_history.remember_disk(path);
        self.player.settings_dirty = true;
    }

    pub(crate) fn remember_kickstart_import(&mut self, path: &std::path::Path) {
        self.file_history.remember_rom(path);
        self.player.settings_dirty = true;
    }

    /// Supplies explicit external ROM bytes for file restoration without changing the machine.
    pub fn supply_restore_kickstart(&mut self, path: &std::path::Path) -> Result<(), String> {
        self.player.restore_kickstart = Some(crate::kickstarts::native::read_image(path)?);
        Ok(())
    }

    /// Loads a confirmed desktop disk source through the manual setup path.
    pub fn load_setup_disk(&mut self, drive: usize, path: &std::path::Path) {
        self.remember_disk_selection(path);
        self.accept_disk_selection(drive, crate::disk_import::read_disk_path(path));
    }

    /// Imports a confirmed desktop selection into the currently opened library.
    pub fn import_kickstart(&mut self, path: &std::path::Path) -> Result<(), String> {
        let result = self.import_kickstart_file(path);
        self.show_kickstart_file_result(&result);
        result
    }

    fn show_kickstart_file_result(&mut self, result: &Result<(), String>) {
        self.file_error = result.as_ref().err().cloned();
        if let Some(editor) = &mut self.player.editor {
            editor.error.clone_from(&self.file_error);
        }
    }

    pub fn load_state_file(&mut self, path: &std::path::Path) {
        match std::fs::read_to_string(path) {
            Ok(json) => {
                let command = match self.prepare_state_import(json) {
                    Ok(command) => command,
                    Err(error) => {
                        self.file_error = Some(error);
                        return;
                    }
                };
                self.host.queue_effect(
                    command,
                    HostEffect::File(FileCompletion::State(format!(
                        "State loaded from {}",
                        path.display()
                    ))),
                )
            }
            Err(error) => {
                self.file_error = Some(format!("Could not load {}: {error}", path.display()))
            }
        }
    }

    pub(crate) fn open_save_state_dialog(&mut self) {
        self.player.captured = false;
        self.host.keyboard_pending.clear();
        if self
            .host
            .runtime
            .as_ref()
            .is_some_and(|runtime| !runtime.input_sender().set_enabled(false, false, false))
        {
            self.player.dialog = Some(NativeDialog::Save);
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Save Amiga 500 State")
            .add_filter("Amiga Save State", &["json"])
            .set_file_name("amiga-state.json")
            .save_file()
        {
            self.save_state_file(&path);
        }
    }

    /// Saves an owner-encoded snapshot through the native host file boundary.
    pub fn save_state_file(&mut self, path: &std::path::Path) {
        self.host.queue_effect(
            HostCommand::SaveSnapshot,
            HostEffect::File(FileCompletion::Save(path.to_owned())),
        );
    }

    pub(crate) fn poll_file_dialog(&mut self, ctx: &egui::Context) {
        if let Some(dialog) = self.player.dialog.take() {
            match dialog {
                NativeDialog::Open(action) => self.open_file_dialog(action),
                NativeDialog::Save => self.open_save_state_dialog(),
            }
            if self.player.dialog.is_some() {
                ctx.request_repaint_after(crate::host_runtime::HOST_RETRY_DELAY);
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod browser;

#[cfg(target_arch = "wasm32")]
pub(crate) use browser::BrowserFileTask;
