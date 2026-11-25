//! Persistent desktop file locations and successful load history.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const RECENT_FILE_LIMIT: usize = 5;
pub const STORAGE_KEY: &str = "file_history";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecentBinary {
    pub path: PathBuf,
    pub target: u32,
    pub auto_prime: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FileHistory {
    pub binary_directory: Option<PathBuf>,
    pub rom_directory: Option<PathBuf>,
    #[serde(default)]
    pub disk_directory: Option<PathBuf>,
    pub binaries: Vec<RecentBinary>,
    pub roms: Vec<PathBuf>,
}

impl FileHistory {
    /// Starting folder for host pickers, falling back through missing parents.
    pub fn picker_directory(&self, binary: bool) -> Option<&Path> {
        let directory = if binary {
            &self.binary_directory
        } else {
            &self.rom_directory
        };
        existing_directory(directory.as_deref()?)
    }

    pub fn disk_picker_directory(&self) -> Option<&Path> {
        existing_directory(self.disk_directory.as_deref()?)
    }

    pub fn remember_disk(&mut self, path: &Path) {
        self.disk_directory = absolute_path(path).parent().map(Path::to_owned);
    }

    pub(super) fn remember_binary(&mut self, path: &Path, target: u32, auto_prime: bool) {
        let path = absolute_path(path);
        self.binary_directory = path.parent().map(Path::to_owned);
        self.binaries.retain(|entry| entry.path != path);
        self.binaries.insert(
            0,
            RecentBinary {
                path,
                target,
                auto_prime,
            },
        );
        self.binaries.truncate(RECENT_FILE_LIMIT);
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
