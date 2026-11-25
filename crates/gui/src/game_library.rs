//! Manually prepared desktop games and durable progress, outside machine state.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameDisk {
    pub id: String,
    pub filename: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl GameDisk {
    pub(crate) fn asset_path(&self, directory: &Path) -> Result<PathBuf, String> {
        validate_disk_filename(&self.filename)?;
        Ok(directory.join("disks").join(&self.filename))
    }
}

pub(crate) fn validate_disk_filename(filename: &str) -> Result<(), String> {
    // Use portable basenames so a game can move between supported desktop filesystems.
    let stem = filename
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        });
    if !crate::disk_import::is_adf(filename)
        || filename.ends_with(['.', ' '])
        || filename
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || filename.len() > 255
        || reserved
    {
        return Err(format!(
            "Invalid ADF filename: {filename:?}. Use a single portable ADF basename."
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameManifest {
    pub id: String,
    pub title: String,
    #[serde(default)]
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
        let mut filenames = std::collections::HashSet::new();
        for disk in &self.disks {
            if disk.id.is_empty() || ids.contains(&disk.id) {
                return Err("Disk IDs must be nonempty and unique".into());
            }
            ids.push(disk.id.clone());
            validate_disk_filename(&disk.filename)?;
            if !filenames.insert(&disk.filename) {
                return Err(format!("Disk filename collision: {}", disk.filename));
            }
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
    // Title edits replace the whole entry through the loader.
    normalized_title: String,
    pub description: String,
    pub artwork: Option<Vec<u8>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl LibraryGame {
    fn with_assets(entry: ManifestEntry) -> Self {
        let ManifestEntry {
            directory,
            manifest,
        } = entry;
        let normalized_title = manifest.title.to_lowercase();
        let description = manifest
            .description
            .as_ref()
            .and_then(|path| relative_asset(&directory, path).ok())
            .and_then(|path| std::fs::read_to_string(path).ok())
            .unwrap_or_default();
        let artwork = artwork_path(&directory, &manifest).and_then(|path| std::fs::read(path).ok());
        Self {
            directory,
            manifest,
            normalized_title,
            description,
            artwork,
        }
    }

    pub(crate) fn title_matches(&self, normalized_query: &str) -> bool {
        self.normalized_title.contains(normalized_query)
    }

    pub(crate) fn same_entry(&self, other: &Self) -> bool {
        self.manifest.id == other.manifest.id
            && (self.directory == other.directory
                || self.directory.canonicalize().is_ok_and(|directory| {
                    other
                        .directory
                        .canonicalize()
                        .is_ok_and(|other| directory == other)
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
        let (entries, errors) = discover_manifests(root)?;
        Ok(Self {
            games: entries.into_iter().map(LibraryGame::with_assets).collect(),
            errors,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub(crate) struct ManifestEntry {
    pub directory: PathBuf,
    pub manifest: GameManifest,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn discover_manifests(root: &Path) -> Result<(Vec<ManifestEntry>, Vec<String>), String> {
    let mut games: Vec<ManifestEntry> = Vec::new();
    let mut errors = Vec::new();
    let entries = std::fs::read_dir(root).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let directory = entry.path();
        let manifest_path = directory.join("game.json");
        match manifest_path.try_exists() {
            Ok(false) => continue,
            Ok(true) => {}
            Err(error) => {
                errors.push(format!("{}: {error}", manifest_path.display()));
                continue;
            }
        }
        let loaded = load_manifest(&directory).and_then(|manifest| {
            if games
                .iter()
                .any(|existing| existing.manifest.id == manifest.id)
            {
                return Err(format!("Duplicate game ID: {}", manifest.id));
            }
            Ok(manifest)
        });
        match loaded {
            Ok(manifest) => games.push(ManifestEntry {
                directory,
                manifest,
            }),
            Err(error) => errors.push(format!("{}: {error}", manifest_path.display())),
        }
    }
    games.sort_by(|a, b| a.manifest.title.cmp(&b.manifest.title));
    Ok((games, errors))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_manifest(directory: &Path) -> Result<GameManifest, String> {
    let bytes = std::fs::read(directory.join("game.json")).map_err(|e| e.to_string())?;
    let manifest: GameManifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    manifest.validate()?;
    Ok(manifest)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_game(directory: &Path) -> Result<LibraryGame, String> {
    Ok(LibraryGame::with_assets(ManifestEntry {
        directory: directory.to_path_buf(),
        manifest: load_manifest(directory)?,
    }))
}

#[cfg(not(target_arch = "wasm32"))]
const COVER_PNG: &str = "cover.png";

#[cfg(not(target_arch = "wasm32"))]
fn artwork_path(directory: &Path, manifest: &GameManifest) -> Option<PathBuf> {
    if let Some(path) = &manifest.artwork {
        return Some(directory.join(path));
    }
    ["cover.jpg", "cover.jpeg", COVER_PNG]
        .into_iter()
        .map(|name| directory.join(name))
        .find(|path| path.is_file())
}

/// Replace the captured PNG and select it if another artwork path has priority.
#[cfg(not(target_arch = "wasm32"))]
pub fn persist_cover(
    game: &LibraryGame,
    write_png: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> Result<(), String> {
    let directory = game.directory.canonicalize().map_err(|e| e.to_string())?;
    let bytes = std::fs::read(directory.join("game.json")).map_err(|e| e.to_string())?;
    let manifest: GameManifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    manifest.validate()?;
    if manifest.id != game.manifest.id {
        return Err("The active game's entry changed on disk".into());
    }
    let cover = directory.join(COVER_PNG);
    let pending = stage_png(&directory, write_png).map_err(|e| e.to_string())?;
    pending.persist(&cover).map_err(|e| e.error.to_string())?;
    let selected = artwork_path(&directory, &manifest);
    if selected.is_some_and(|path| {
        path == cover
            || (cfg!(windows)
                && path
                    .canonicalize()
                    .ok()
                    .zip(cover.canonicalize().ok())
                    .is_some_and(|(selected, cover)| selected == cover))
    }) {
        return Ok(());
    }
    let select_cover = || {
        let current = std::fs::read(directory.join("game.json")).map_err(|e| e.to_string())?;
        if current != bytes {
            return Err("The game manifest changed while saving the cover. Try again.".into());
        }
        let mut document: serde_json::Value =
            serde_json::from_slice(&current).map_err(|e| e.to_string())?;
        document["artwork"] = serde_json::Value::String(COVER_PNG.into());
        let updated = serde_json::to_vec_pretty(&document).map_err(|e| e.to_string())?;
        write_manifest(&directory, &updated)
    };
    select_cover()
        .map_err(|error| format!("Saved cover.png but could not select it in game.json: {error}"))
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct PlayerSave {
    pub path: PathBuf,
    pub header: debugger::SaveStateHeader,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn saves(game: &LibraryGame) -> Result<Vec<PlayerSave>, String> {
    saves_in(&game.directory, &game.manifest)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn saves_in(
    game_directory: &Path,
    manifest: &GameManifest,
) -> Result<Vec<PlayerSave>, String> {
    let directory = game_directory.join("saves");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let saved = debugger::SaveState::from_bytes(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        manifest
            .validate_association(&saved.header.association)
            .map_err(|error| format!("{}: {error}", path.display()))?;
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
    let directory = game.directory.join("Screenshots");
    std::fs::create_dir_all(&directory)?;
    persist_numbered_png(&directory, "screenshot", write_png).map(|_| ())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn persist_numbered_png(
    directory: &Path,
    stem: &str,
    write_png: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> std::io::Result<String> {
    use std::io::{Error, ErrorKind};

    // On Windows, tempfile passes paths directly to Win32 during publication.
    let directory = directory.canonicalize()?;
    let mut pending = stage_png(&directory, write_png)?;
    for serial in 1..=u32::MAX {
        let name = format!("{stem}-{serial}.png");
        let path = directory.join(&name);
        if path.try_exists()? {
            continue;
        }
        match pending.persist_noclobber(path) {
            Ok(_) => return Ok(name),
            Err(error) if error.error.kind() == ErrorKind::AlreadyExists => pending = error.file,
            Err(error) => return Err(error.error),
        }
    }
    Err(Error::other(format!("No available {stem} filename")))
}

#[cfg(not(target_arch = "wasm32"))]
fn stage_png(
    directory: &Path,
    write_png: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> std::io::Result<tempfile::NamedTempFile> {
    let mut pending = tempfile::Builder::new()
        .suffix(".pending")
        .tempfile_in(directory)?;
    write_png(pending.as_file_mut())?;
    pending.as_file().sync_all()?;
    Ok(pending)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn write_rgba_png(
    file: &mut std::fs::File,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> std::io::Result<()> {
    use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
    use std::io::{Error, Write};

    // Write the complete encoded PNG through an operation that reports errors.
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(pixels, width, height, ExtendedColorType::Rgba8)
        .map_err(Error::other)?;
    file.write_all(&png)
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
    let written = file.write_all(bytes).and_then(|_| file.sync_all());
    drop(file);
    if let Err(error) = written.and_then(|_| std::fs::rename(&pending, directory.join("game.json")))
    {
        std::fs::remove_file(pending).map_err(|cleanup| {
            format!("{error}. Could not remove the temporary manifest: {cleanup}")
        })?;
        return Err(error.to_string());
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn replace_configuration(
    directory: &Path,
    expected: &GameManifest,
    replacement: &str,
) -> Result<GameManifest, String> {
    let path = directory.join("game.json");
    let before = std::fs::read(&path).map_err(|error| error.to_string())?;
    let mut manifest: GameManifest =
        serde_json::from_slice(&before).map_err(|error| error.to_string())?;
    if manifest != *expected {
        return Err(
            "The game manifest changed on disk. Select the updated game and try again.".into(),
        );
    }
    let mut document: serde_json::Value =
        serde_json::from_slice(&before).map_err(|error| error.to_string())?;
    document["configuration"] = replacement.into();
    let updated = serde_json::to_vec_pretty(&document).map_err(|error| error.to_string())?;
    write_manifest(directory, &updated)?;
    manifest.configuration = replacement.into();
    Ok(manifest)
}
