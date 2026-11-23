//! Manually prepared desktop games and durable progress, outside machine state.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameDisk {
    pub id: String,
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameManifest {
    pub id: String,
    pub title: String,
    pub configuration: String,
    pub disks: Vec<GameDisk>,
    pub mounted: [Option<String>; 4],
    pub initial_state: Option<String>,
    pub artwork: Option<String>,
    pub description: Option<String>,
}

impl GameManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() || self.title.trim().is_empty() {
            return Err("Game ID and title are required".into());
        }
        let mut ids = Vec::new();
        for disk in &self.disks {
            if disk.id.is_empty() || ids.contains(&disk.id) {
                return Err("Disk IDs must be nonempty and unique".into());
            }
            ids.push(disk.id.clone());
            relative_asset(Path::new("."), &disk.path)?;
        }
        for id in self.mounted.iter().flatten() {
            if !ids.contains(id) {
                return Err(format!("Unknown mounted disk ID: {id}"));
            }
        }
        for asset in [&self.initial_state, &self.artwork, &self.description]
            .into_iter()
            .flatten()
        {
            relative_asset(Path::new("."), asset)?;
        }
        Ok(())
    }
    pub fn validate_association(
        &self,
        association: &debugger::SaveAssociation,
    ) -> Result<(), String> {
        if association.game_id.as_deref() != Some(&self.id) {
            return Err("Save belongs to another game".into());
        }
        for id in association.mounted.iter().flatten() {
            if !self.disks.iter().any(|disk| &disk.id == id) {
                return Err(format!("Save refers to an unknown disk: {id}"));
            }
        }
        Ok(())
    }
}

pub fn relative_asset(directory: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if relative.is_empty()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return Err(format!(
            "Asset must use a relative path inside the game directory: {relative}"
        ));
    }
    Ok(directory.join(path))
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct LibraryGame {
    pub directory: PathBuf,
    pub manifest: GameManifest,
    pub description: String,
    pub artwork: Option<Vec<u8>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl LibraryGame {
    pub(crate) fn same_entry(&self, other: &Self) -> bool {
        self.manifest.id == other.manifest.id
            && (self.directory == other.directory
                || self.directory.canonicalize().is_ok_and(|directory| {
                    other.directory.canonicalize().is_ok_and(|other| directory == other)
                }))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Default)]
pub struct GameLibrary {
    pub games: Vec<LibraryGame>,
    pub errors: Vec<String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl GameLibrary {
    pub fn discover(root: &Path) -> Result<Self, String> {
        let mut library = Self::default();
        let entries = std::fs::read_dir(root).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let directory = entry.path();
            if !directory.is_dir() {
                continue;
            }
            let manifest_path = directory.join("game.json");
            if !manifest_path.is_file() {
                continue;
            }
            let loaded = load_game(&directory).and_then(|game| {
                if library
                    .games
                    .iter()
                    .any(|existing| existing.manifest.id == game.manifest.id)
                {
                    return Err(format!("Duplicate game ID: {}", game.manifest.id));
                }
                Ok(game)
            });
            match loaded {
                Ok(game) => library.games.push(game),
                Err(error) => library
                    .errors
                    .push(format!("{}: {error}", manifest_path.display())),
            }
        }
        library
            .games
            .sort_by(|a, b| a.manifest.title.cmp(&b.manifest.title));
        Ok(library)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_game(directory: &Path) -> Result<LibraryGame, String> {
    let bytes = std::fs::read(directory.join("game.json")).map_err(|e| e.to_string())?;
    let manifest: GameManifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    manifest.validate()?;
    let description = manifest
        .description
        .as_ref()
        .and_then(|path| relative_asset(directory, path).ok())
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default();
    let artwork_path = manifest
        .artwork
        .as_ref()
        .map(|path| relative_asset(directory, path))
        .transpose()?
        .or_else(|| {
            ["cover.jpg", "cover.jpeg", "cover.png"]
                .iter()
                .map(|name| directory.join(name))
                .find(|path| path.is_file())
        });
    let artwork = artwork_path.and_then(|path| std::fs::read(path).ok());
    Ok(LibraryGame {
        directory: directory.to_path_buf(),
        manifest,
        description,
        artwork,
    })
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct PlayerSave {
    pub path: PathBuf,
    pub header: debugger::SaveStateHeader,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn saves(game: &LibraryGame) -> Result<Vec<PlayerSave>, String> {
    let directory = game.directory.join("saves");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let saved = debugger::SaveState::from_json(&json)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        game.manifest
            .validate_association(&saved.header.association)?;
        result.push(PlayerSave {
            path,
            header: saved.header,
        });
    }
    result.sort_by(|a, b| {
        b.header
            .timestamp
            .cmp(&a.header.timestamp)
            .then_with(|| b.path.cmp(&a.path))
    });
    Ok(result)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn persist_save(
    game: &LibraryGame,
    json: &str,
    quick_slot: Option<usize>,
) -> Result<PathBuf, String> {
    use std::io::Write;
    let saved = debugger::SaveState::from_json(json).map_err(|e| e.to_string())?;
    game.manifest
        .validate_association(&saved.header.association)?;
    let directory = game.directory.join("saves");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    for serial in 0..u32::MAX {
        let name = format!("{}-{serial}.json", saved.header.timestamp);
        let path = directory.join(name);
        let pending = path.with_extension("pending");
        if path.exists() {
            continue;
        }
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        };
        file.write_all(json.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(&pending, &path).map_err(|e| e.to_string())?;
        if let Some(slot) = quick_slot {
            if !(1..=debugger::session::QUICK_SLOT_COUNT).contains(&slot) {
                return Err("Invalid quick slot".into());
            }
            // A tiny pointer selects a durable immutable save; it never overwrites the initial state.
            std::fs::write(
                directory.join(format!("slot-{slot}.txt")),
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .as_bytes(),
            )
            .map_err(|e| e.to_string())?;
        }
        return Ok(path);
    }
    Err("No available save filename".into())
}

/// Writes a new screenshot to the game directory and publishes it after the write succeeds.
#[cfg(not(target_arch = "wasm32"))]
pub fn persist_screenshot(
    game: &LibraryGame,
    write_png: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};

    let directory = game.directory.join("Screenshots");
    std::fs::create_dir_all(&directory)?;
    // On Windows, tempfile passes paths directly to Win32 during publication.
    #[cfg(windows)]
    let directory = directory.canonicalize()?;
    let mut pending = tempfile::Builder::new().suffix(".pending").tempfile_in(&directory)?;
    write_png(pending.as_file_mut())?;
    pending.as_file().sync_all()?;
    for serial in 1..=u32::MAX {
        let path = directory.join(format!("screenshot-{serial}.png"));
        if path.try_exists()? {
            continue;
        }
        match pending.persist_noclobber(&path) {
            Ok(_) => return Ok(()),
            Err(error) if error.error.kind() == ErrorKind::AlreadyExists => pending = error.file,
            Err(error) => return Err(error.error),
        }
    }
    Err(Error::other("No available screenshot filename"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_manifest(directory: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let pending = directory.join("game.json.pending");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(pending, directory.join("game.json")).map_err(|e| e.to_string())
}
