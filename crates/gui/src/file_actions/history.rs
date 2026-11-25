//! Persistent desktop file locations and successful load history.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const RECENT_FILE_LIMIT: usize = 5;
pub const STORAGE_KEY: &str = "file_history";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FileHistory {
    pub rom_directory: Option<PathBuf>,
    #[serde(default)]
    pub disk_directory: Option<PathBuf>,
    pub roms: Vec<PathBuf>,
}

impl FileHistory {
    /// Starting folder for host pickers, falling back through missing parents.
    pub fn rom_picker_directory(&self) -> Option<&Path> {
        existing_directory(self.rom_directory.as_deref()?)
    }

    pub fn disk_picker_directory(&self) -> Option<&Path> {
        existing_directory(self.disk_directory.as_deref()?)
    }

    pub fn remember_disk(&mut self, path: &Path) {
        self.disk_directory = absolute_path(path).parent().map(Path::to_owned);
    }

    pub(super) fn remember_rom(&mut self, path: &Path) {
        let path = absolute_path(path);
        self.rom_directory = path.parent().map(Path::to_owned);
        self.roms.retain(|entry| entry != &path);
        self.roms.insert(0, path);
        self.roms.truncate(RECENT_FILE_LIMIT);
    }
}

fn existing_directory(directory: &Path) -> Option<&Path> {
    directory.ancestors().find(|path| path.is_dir())
}

fn absolute_path(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_owned())
}
