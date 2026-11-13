//! Host file selection and buffer handoff to the headless session.

use crate::layout::top_menu_bar::TOAST_DURATION_FRAMES;
use crate::EmulatorApp;

/// Selected binary, read on confirmation so a native file can be corrected and retried.
#[derive(Debug)]
pub enum BinarySource {
    #[cfg(not(target_arch = "wasm32"))]
    File(std::path::PathBuf),
    Bytes {
        name: String,
        data: Vec<u8>,
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

    fn read(&self) -> Result<std::borrow::Cow<'_, [u8]>, String> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::File(path) => std::fs::read(path)
                .map(std::borrow::Cow::Owned)
                .map_err(|error| error.to_string()),
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
        self.pending_binary = Some(source);
        self.load_binary_modal_open = true;
        self.file_error = None;
    }

    /// Installs firmware through the same buffer API on desktop and in the browser.
    pub fn load_rom_data(&mut self, name: &str, data: &[u8]) {
        match self.session.load_rom(data) {
            Ok(()) => {
                self.loaded_rom_name = Some(name.to_owned());
                self.file_error = None;
                self.disassembly_view_addr = None;
                self.toast_message = Some((
                    "ROM loaded; press Run to boot.".into(),
                    TOAST_DURATION_FRAMES,
                ));
            }
            Err(error) => self.file_error = Some(format!("Could not load {name}: {error}")),
        }
    }

    pub(crate) fn confirm_binary_load(&mut self, target: u32, auto_prime: bool) {
        let Some(source) = self.pending_binary.take() else {
            return;
        };
        self.load_binary_source(&source, target, auto_prime);
        if self.load_binary_modal_open {
            self.pending_binary = Some(source);
        }
    }

    fn load_binary_source(&mut self, source: &BinarySource, target: u32, auto_prime: bool) {
        let result = source.read().and_then(|data| {
            self.session
                .load_binary(target, &data, auto_prime)
                .map_err(|error| error.to_string())
        });
        match result {
            Ok(size) => {
                let address = target & physical_memory::MASK_PHYSICAL_ADDRESS;
                if (address as usize).saturating_add(size)
                    > config::address_map::KICKSTART_BASE as usize
                {
                    self.loaded_rom_name = Some(source.name());
                }
                self.hex_base_addr =
                    target & crate::layout::right_dock::memory_hex::MASK_ROW_ADDRESS;
                self.file_error = None;
                self.load_binary_modal_open = false;
            }
            Err(error) => {
                self.file_error = Some(format!("Could not load {}: {error}", source.name()))
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn accept_file_data(&mut self, action: FileAction, name: String, data: Vec<u8>) {
        match action {
            FileAction::Binary => self.select_binary(BinarySource::Bytes { name, data }),
            FileAction::Rom => self.load_rom_data(&name, &data),
            FileAction::State => {
                let result = std::str::from_utf8(&data)
                    .map_err(|error| error.to_string())
                    .and_then(|json| {
                        self.session
                            .load_state_from_json(json)
                            .map_err(|error| error.to_string())
                    });
                match result {
                    Ok(()) => self.state_loaded(format!("State loaded from {name}")),
                    Err(error) => self.file_error = Some(format!("Could not load {name}: {error}")),
                }
            }
        }
    }

    pub(crate) fn state_loaded(&mut self, message: String) {
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
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl EmulatorApp {
    fn open_file_dialog(&mut self, action: FileAction) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(action.title())
            .add_filter("Supported files", action.extensions())
            .pick_file()
        {
            match action {
                FileAction::Binary => self.select_binary(BinarySource::File(path)),
                FileAction::Rom => self.load_rom_file(&path),
                FileAction::State => self.load_state_file(&path),
            }
        }
    }

    pub fn load_rom_file(&mut self, path: &std::path::Path) {
        match std::fs::read(path) {
            Ok(data) => self.load_rom_data(&file_name(path), &data),
            Err(error) => {
                self.file_error = Some(format!("Could not read {}: {error}", path.display()))
            }
        }
    }

    pub fn load_binary_file(&mut self, path: &std::path::Path, target: u32, auto_prime: bool) {
        self.load_binary_source(&BinarySource::File(path.to_owned()), target, auto_prime);
    }

    pub fn load_state_file(&mut self, path: &std::path::Path) {
        match self.session.load_state_from_file(path) {
            Ok(()) => self.state_loaded(format!("State loaded from {}", path.display())),
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
            match self.session.save_state_to_file(&path) {
                Ok(()) => {
                    self.toast_message = Some((
                        format!("State saved to {}", path.display()),
                        TOAST_DURATION_FRAMES,
                    ))
                }
                Err(error) => self.file_error = Some(format!("Could not save state: {error}")),
            }
        }
    }

    pub(crate) fn poll_file_dialog(&mut self, _ctx: &egui::Context) {}
}

#[cfg(target_arch = "wasm32")]
mod browser;

#[cfg(target_arch = "wasm32")]
pub(crate) use browser::BrowserFileTask;
