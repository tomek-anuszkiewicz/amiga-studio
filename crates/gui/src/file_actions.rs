//! Host file selection and buffer handoff to the headless session.

use crate::app_host::HostEffect;
use crate::disk_import::{DiskSelection, DISK_EXTENSIONS};
use crate::host_commands::{HostCommand, HostValue};
use crate::layout::top_menu_bar::TOAST_DURATION_FRAMES;
use crate::EmulatorApp;

#[cfg(not(target_arch = "wasm32"))]
mod history;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use history::STORAGE_KEY as HISTORY_STORAGE_KEY;
#[cfg(not(target_arch = "wasm32"))]
pub use history::{FileHistory, RecentBinary};

/// Selected binary, read on confirmation so a native file can be corrected and retried.
#[derive(Debug)]
pub enum BinarySource {
    #[cfg(not(target_arch = "wasm32"))]
    File(std::path::PathBuf),
    Bytes {
        name: String,
        data: Vec<u8>,
        #[cfg(target_arch = "wasm32")]
        handle: wasm_bindgen::JsValue,
    },
}

impl BinarySource {
    pub fn name(&self) -> String {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::File(path) => file_name(path),
            Self::Bytes { name, .. } => name.clone(),
        }
    }

    fn read(&self) -> Result<std::borrow::Cow<'_, [u8]>, std::io::Error> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::File(path) => std::fs::read(path).map(std::borrow::Cow::Owned),
            Self::Bytes { data, .. } => Ok(std::borrow::Cow::Borrowed(data)),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum FileAction {
    Binary,
    Rom,
    State,
    SetupRom,
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
            Self::Binary => "Load Binary from File",
            Self::Rom => "Load ROM from File",
            Self::State => "Load State from File",
            Self::SetupRom => "Select Kickstart for Setup",
            Self::SetupDisk(_) => "Select disk for Setup",
        }
    }

    fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Binary => &["bin", "exe", "dat", "rom"],
            Self::Rom => &["rom", "bin"],
            Self::State => &["json"],
            Self::SetupRom => &["rom", "bin"],
            Self::SetupDisk(_) => DISK_EXTENSIONS,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

#[derive(Debug)]
pub(crate) enum FileCompletion {
    Rom {
        name: String,
        #[cfg(not(target_arch = "wasm32"))]
        path: std::path::PathBuf,
        #[cfg(target_arch = "wasm32")]
        handle: wasm_bindgen::JsValue,
    },
    Binary {
        source: BinarySource,
        target: u32,
        auto_prime: bool,
        size: usize,
    },
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
        Ok(HostCommand::RestorePlayer {
            json,
            game_id: saved.header.association.game_id,
            ports: self.current_control_ports(),
            resume: false,
        })
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

    pub(crate) fn open_setup_asset(&mut self, drive: Option<usize>) {
        self.player.captured = false;
        self.player.drive_disk_choice = None;
        self.open_file_dialog(drive.map_or(FileAction::SetupRom, FileAction::SetupDisk));
    }

    pub(crate) fn open_load_binary_dialog(&mut self) {
        self.open_file_dialog(FileAction::Binary);
    }

    pub(crate) fn open_load_rom_dialog(&mut self) {
        self.open_file_dialog(FileAction::Rom);
    }

    pub(crate) fn open_load_state_dialog(&mut self) {
        self.open_file_dialog(FileAction::State);
    }

    fn select_binary(&mut self, source: BinarySource) {
        #[cfg(not(target_arch = "wasm32"))]
        if let BinarySource::File(path) = &source {
            self.file_history.binary_directory = path.parent().map(std::path::Path::to_owned);
        }
        self.pending_binary = Some(source);
        self.load_binary_modal_open = true;
        self.file_error = None;
    }

    pub(crate) fn confirm_binary_load(&mut self, target: u32, auto_prime: bool) {
        if let Some(source) = self.pending_binary.take() {
            self.load_binary_source(source, target, auto_prime);
        }
    }

    fn load_binary_source(&mut self, source: BinarySource, target: u32, auto_prime: bool) {
        match source.read() {
            Ok(bytes) => {
                let bytes = bytes.into_owned();
                let size = bytes.len();
                self.host.queue_effect(
                    HostCommand::LoadBinary {
                        address: target,
                        bytes,
                        prime: auto_prime,
                    },
                    HostEffect::File(FileCompletion::Binary {
                        source,
                        target,
                        auto_prime,
                        size,
                    }),
                );
            }
            Err(error) => {
                self.file_error = Some(format!("Could not load {}: {error}", source.name()));
                #[cfg(not(target_arch = "wasm32"))]
                if let BinarySource::File(path) = &source {
                    if error.kind() == std::io::ErrorKind::NotFound
                        && self
                            .file_history
                            .binaries
                            .iter()
                            .any(|entry| entry.path == *path)
                    {
                        self.file_history
                            .binaries
                            .retain(|entry| entry.path != *path);
                        self.file_error = Some(format!(
                            "{} no longer exists. Removed from recent binaries.",
                            path.display()
                        ));
                    }
                }
                if self.load_binary_modal_open {
                    self.pending_binary = Some(source);
                }
            }
        }
    }

    pub(crate) fn complete_file_action(
        &mut self,
        action: FileCompletion,
        value: Result<HostValue, String>,
    ) {
        if let Err(error) = value {
            self.file_error = Some(format!("Could not complete file action: {error}"));
            if let FileCompletion::Binary { source, .. } = action {
                if self.load_binary_modal_open {
                    self.pending_binary = Some(source);
                }
            }
            return;
        }
        if let Ok(HostValue::PlayerRestored(association)) = &value {
            for id in association.mounted.iter().flatten() {
                if !self.player.disks.iter().any(|disk| &disk.id == id) {
                    self.player.disks.push(crate::player::SessionDisk {
                        id: id.clone(),
                        label: id.clone(),
                        bytes: None,
                    });
                }
            }
            self.player.mounted = association.mounted.clone();
            self.player.captured = false;
        }
        self.file_error = None;
        match action {
            FileCompletion::Rom {
                name,
                #[cfg(not(target_arch = "wasm32"))]
                path,
                #[cfg(target_arch = "wasm32")]
                handle,
            } => {
                #[cfg(not(target_arch = "wasm32"))]
                self.file_history.remember_rom(&path);
                #[cfg(target_arch = "wasm32")]
                self.browser_history
                    .remember(FileAction::Rom, &name, &handle);
                self.loaded_rom_name = Some(name);
                self.inspection.ready = false;
                self.disassembly_view_addr = None;
                self.toast_message = Some((
                    "ROM loaded; press Run to boot.".into(),
                    TOAST_DURATION_FRAMES,
                ));
            }
            FileCompletion::Binary {
                source,
                target,
                auto_prime,
                size,
            } => {
                #[cfg(not(target_arch = "wasm32"))]
                if let BinarySource::File(path) = &source {
                    self.file_history.remember_binary(path, target, auto_prime);
                }
                #[cfg(target_arch = "wasm32")]
                {
                    let BinarySource::Bytes { name, handle, .. } = &source;
                    let _ = auto_prime;
                    self.browser_history
                        .remember(FileAction::Binary, name, handle);
                }
                if !self.host.status.running {
                    self.inspection.ready = false;
                }
                let address = target & physical_memory::MASK_PHYSICAL_ADDRESS;
                if (address as usize).saturating_add(size)
                    > config::address_map::KICKSTART_BASE as usize
                {
                    self.loaded_rom_name = Some(source.name());
                }
                self.hex_base_addr =
                    target & crate::layout::right_dock::memory_hex::MASK_ROW_ADDRESS;
                self.load_binary_modal_open = false;
            }
            FileCompletion::State(message) => {
                if self.player.active_configuration.is_none() {
                    self.player.activate_selected_configuration();
                }
                self.state_loaded(message);
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
    fn accept_file_data(
        &mut self,
        action: FileAction,
        name: String,
        data: Vec<u8>,
        handle: wasm_bindgen::JsValue,
    ) {
        match action {
            FileAction::SetupRom => self.select_setup_rom(name, data),
            FileAction::SetupDisk(_) => {}
            FileAction::Binary => self.select_binary(BinarySource::Bytes { name, data, handle }),
            FileAction::Rom => self.request_player_launch(crate::player::PendingLaunch::Rom {
                bytes: data,
                completion: FileCompletion::Rom { name, handle },
            }),
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

    pub(crate) fn state_loaded(&mut self, message: String) {
        self.inspection.ready = false;
        self.loaded_rom_name = Some("from saved state".into());
        self.file_error = None;
        self.disassembly_view_addr = None;
        self.toast_message = Some((message, TOAST_DURATION_FRAMES));
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn accept_dropped_file(&mut self, file: egui::DroppedFileHandle) {
        self.select_binary(BinarySource::File(file.path().to_owned()));
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl EmulatorApp {
    fn open_file_dialog(&mut self, action: FileAction) {
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
        let mut dialog = if matches!(action, FileAction::SetupDisk(_)) {
            self.disk_file_dialog(action.title())
        } else {
            rfd::FileDialog::new()
                .set_title(action.title())
                .add_filter("Supported files", action.extensions())
        };
        let directory = match action {
            FileAction::Binary => self.file_history.picker_directory(true),
            FileAction::Rom => self.file_history.picker_directory(false),
            FileAction::State | FileAction::SetupRom | FileAction::SetupDisk(_) => None,
        };
        if let Some(directory) = directory {
            dialog = dialog.set_directory(directory);
        }
        if let Some(path) = dialog.pick_file() {
            match action {
                FileAction::Binary => {
                    self.file_history.binary_directory =
                        path.parent().map(std::path::Path::to_owned)
                }
                FileAction::Rom => {
                    self.file_history.rom_directory = path.parent().map(std::path::Path::to_owned)
                }
                FileAction::State | FileAction::SetupRom | FileAction::SetupDisk(_) => {}
            }
            match action {
                FileAction::Binary => self.select_binary(BinarySource::File(path)),
                FileAction::Rom => self.load_rom_file(&path),
                FileAction::State => self.load_state_file(&path),
                FileAction::SetupRom => match std::fs::read(&path) {
                    Ok(bytes) => self.select_setup_rom(file_name(&path), bytes),
                    Err(error) => self.file_error = Some(format!("Could not read asset: {error}")),
                },
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

    /// Loads a confirmed desktop disk source through the manual setup path.
    pub fn load_setup_disk(&mut self, drive: usize, path: &std::path::Path) {
        self.remember_disk_selection(path);
        self.accept_disk_selection(drive, crate::disk_import::read_disk_path(path));
    }

    pub fn load_rom_file(&mut self, path: &std::path::Path) {
        match std::fs::read(path) {
            Ok(data) => self.request_player_launch(crate::player::PendingLaunch::Rom {
                bytes: data,
                completion: FileCompletion::Rom {
                    name: file_name(path),
                    path: path.to_owned(),
                },
            }),
            Err(error) => {
                self.file_error = Some(format!("Could not read {}: {error}", path.display()));
                if error.kind() == std::io::ErrorKind::NotFound
                    && self.file_history.roms.iter().any(|entry| entry == path)
                {
                    self.file_history.roms.retain(|entry| entry != path);
                    self.file_error = Some(format!(
                        "{} no longer exists. Removed from recent ROMs.",
                        path.display()
                    ));
                }
            }
        }
    }

    pub fn load_binary_file(&mut self, path: &std::path::Path, target: u32, auto_prime: bool) {
        self.load_binary_source(BinarySource::File(path.to_owned()), target, auto_prime);
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
pub(crate) use browser::{BrowserFileTask, BrowserHistory};
