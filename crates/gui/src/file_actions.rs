//! Host file selection and buffer handoff to the headless session.

use crate::app_host::HostEffect;
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
            Self::SetupDisk(_) => "Select ADF for Setup",
        }
    }

    fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Binary => &["bin", "exe", "dat", "rom"],
            Self::Rom => &["rom", "bin"],
            Self::State => &["json"],
            Self::SetupRom => &["rom", "bin"],
            Self::SetupDisk(_) => &["adf"],
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
    QuickSave(usize),
}

impl EmulatorApp {
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
            self.player
                .catalog
                .validate_controls(&configuration.controls)?;
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
        self.open_file_dialog(drive.map_or(FileAction::SetupRom, FileAction::SetupDisk));
    }

    pub(crate) fn open_load_binary_dialog(&mut self) {
        self.open_file_dialog(FileAction::Binary);
    }

    pub(crate) fn open_load_rom_dialog(&mut self) {
        self.open_file_dialog(FileAction::Rom);
    }

    pub(crate) fn open_load_state_dialog(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.player.active_game.is_some() {
            self.load_player();
            return;
        }
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
                    self.player.active_configuration = self.player.catalog.selected().cloned();
                }
                self.state_loaded(message);
            }
            FileCompletion::QuickSave(slot) => {
                self.toast_message = Some((
                    format!("Quick-saved to Slot #{slot}"),
                    TOAST_DURATION_FRAMES,
                ));
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
            FileAction::SetupRom => self.select_setup_asset(None, name, data),
            FileAction::SetupDisk(drive) => self.select_setup_asset(Some(drive), name, data),
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

    pub(crate) fn accept_dropped_file(&mut self, file: egui::DroppedFile) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = file.path {
            self.select_binary(BinarySource::File(path));
            return;
        }
        if let Some(data) = file.bytes {
            self.select_binary(BinarySource::Bytes {
                name: file.name,
                data: data.to_vec(),
                #[cfg(target_arch = "wasm32")]
                handle: wasm_bindgen::JsValue::NULL,
            });
        }
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
        let mut dialog = rfd::FileDialog::new()
            .set_title(action.title())
            .add_filter("Supported files", action.extensions());
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
                FileAction::SetupRom | FileAction::SetupDisk(_) => match std::fs::read(&path) {
                    Ok(bytes) => {
                        let drive = match action {
                            FileAction::SetupDisk(drive) => Some(drive),
                            _ => None,
                        };
                        self.select_setup_asset(drive, file_name(&path), bytes);
                        if drive.is_none() {
                            if let Some(configuration) = self
                                .player
                                .catalog
                                .configurations
                                .iter_mut()
                                .find(|c| c.id == self.player.catalog.selected)
                            {
                                configuration.launch.rom = path.to_string_lossy().into_owned();
                            }
                        }
                    }
                    Err(error) => self.file_error = Some(format!("Could not read asset: {error}")),
                },
            }
        }
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
