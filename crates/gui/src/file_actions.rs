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
enum FileAction {
    Binary,
    Rom,
    State,
}

impl FileAction {
    fn title(self) -> &'static str {
        match self {
            Self::Binary => "Load Binary from File",
            Self::Rom => "Load ROM from File",
            Self::State => "Load State from File",
        }
    }

    fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Binary => &["bin", "exe", "dat", "rom"],
            Self::Rom => &["rom", "bin"],
            Self::State => &["json"],
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
            FileCompletion::State(message) => self.state_loaded(message),
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
            FileAction::Binary => self.select_binary(BinarySource::Bytes { name, data, handle }),
            FileAction::Rom => self.host.queue_effect(
                HostCommand::LoadRom(data),
                HostEffect::File(FileCompletion::Rom { name, handle }),
            ),
            FileAction::State => match String::from_utf8(data) {
                Ok(json) => self.host.queue_effect(
                    HostCommand::LoadSnapshot(json),
                    HostEffect::File(FileCompletion::State(format!("State loaded from {name}"))),
                ),
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
        let mut dialog = rfd::FileDialog::new()
            .set_title(action.title())
            .add_filter("Supported files", action.extensions());
        let directory = match action {
            FileAction::Binary => self.file_history.picker_directory(true),
            FileAction::Rom => self.file_history.picker_directory(false),
            FileAction::State => None,
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
                FileAction::State => {}
            }
            match action {
                FileAction::Binary => self.select_binary(BinarySource::File(path)),
                FileAction::Rom => self.load_rom_file(&path),
                FileAction::State => self.load_state_file(&path),
            }
        }
    }

    pub fn load_rom_file(&mut self, path: &std::path::Path) {
        match std::fs::read(path) {
            Ok(data) => self.host.queue_effect(
                HostCommand::LoadRom(data),
                HostEffect::File(FileCompletion::Rom {
                    name: file_name(path),
                    path: path.to_owned(),
                }),
            ),
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
            Ok(json) => self.host.queue_effect(
                HostCommand::LoadSnapshot(json),
                HostEffect::File(FileCompletion::State(format!(
                    "State loaded from {}",
                    path.display()
                ))),
            ),
            Err(error) => {
                self.file_error = Some(format!("Could not load {}: {error}", path.display()))
            }
        }
    }

    pub(crate) fn open_save_state_dialog(&mut self) {
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

    pub(crate) fn poll_file_dialog(&mut self, _ctx: &egui::Context) {}
}

#[cfg(target_arch = "wasm32")]
mod browser;

#[cfg(target_arch = "wasm32")]
pub(crate) use browser::{BrowserFileTask, BrowserHistory};
