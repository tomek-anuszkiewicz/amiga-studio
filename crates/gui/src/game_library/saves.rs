use super::{load_manifest, relative_asset, GameManifest, LibraryGame};
use debugger::{SaveState, SaveStateError, SaveStateHeader, SAVE_STATE_VERSION};
use serde::Deserialize;
use serde_json::value::{to_raw_value, RawValue};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

const QUICKSAVE_FILE: &str = "quicksave.txt";
const LEGACY_QUICKSAVE_FILE: &str = "slot-1.txt";

#[derive(Debug, Clone)]
pub struct PlayerSave {
    pub path: PathBuf,
    pub header: Option<SaveStateHeader>,
    pub quicksave: bool,
    pub problem: Option<String>,
    preview: Option<([u32; 2], u64)>,
}

impl PlayerSave {
    pub(crate) fn preview(&self) -> Option<([u32; 2], u64, &[u8])> {
        let (dimensions, revision) = self.preview?;
        Some((dimensions, revision, &self.header.as_ref()?.screenshot_png))
    }

    pub(crate) fn timestamp(&self) -> Option<u64> {
        self.header
            .as_ref()
            .map(|header| header.timestamp)
            // Capture uses zero when host time is unavailable. Display years through 9999.
            .filter(|&time| (1..=253_402_300_799).contains(&time))
    }
}

#[derive(Debug, Default)]
pub struct SaveCatalog {
    pub entries: Vec<PlayerSave>,
    pub(crate) warning: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SaveVersion {
    header: VersionHeader,
}

#[derive(Debug, Deserialize)]
struct VersionHeader {
    version: u32,
}

fn inspect_save(bytes: &[u8], manifest: &GameManifest) -> Result<SaveStateHeader, String> {
    let saved = SaveState::from_bytes(bytes).map_err(|error| match error {
        SaveStateError::IncompatibleVersion { .. } => format!("Incompatible save: {error}"),
        _ if serde_json::from_slice::<SaveVersion>(bytes).is_ok_and(|file| {
            file.header.version == SAVE_STATE_VERSION
        }) => format!(
            "Incompatible save (found {SAVE_STATE_VERSION}, supported {SAVE_STATE_VERSION}): {error}"
        ),
        _ => format!("Corrupt save: {error}"),
    })?;
    manifest
        .validate_association(&saved.header.association)
        .map_err(|error| format!("Incompatible save: {error}"))?;
    Ok(saved.header)
}

pub(crate) fn read_associated_save(
    path: &Path,
    manifest: &GameManifest,
) -> Result<(Vec<u8>, SaveStateHeader), String> {
    let bytes = fs::read(path).map_err(|error| format!("Unreadable save: {error}"))?;
    let header = inspect_save(&bytes, manifest)?;
    Ok((bytes, header))
}

pub fn saves(game: &LibraryGame) -> Result<SaveCatalog, String> {
    saves_in(&game.directory, &game.manifest)
}

pub(crate) fn saves_in(directory: &Path, manifest: &GameManifest) -> Result<SaveCatalog, String> {
    let mut catalog = SaveCatalog::default();
    let entries = match fs::read_dir(directory.join("saves")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(catalog),
        Err(error) => return Err(error.to_string()),
    };
    let quicksave = match current_quicksave(directory) {
        Ok(quicksave) => quicksave,
        Err(error) => {
            catalog.warning = Some(error);
            None
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                catalog.warning.get_or_insert_with(|| error.to_string());
                continue;
            }
        };
        let path = entry.path();
        if !is_save_path(&path) || is_initial_state(directory, manifest, &path) {
            continue;
        }
        match entry.file_type() {
            Ok(file_type) if file_type.is_dir() => continue,
            Err(error) => {
                catalog.warning.get_or_insert_with(|| error.to_string());
            }
            _ => {}
        }
        let (header, problem) = match read_associated_save(&path, manifest) {
            Ok((_, header)) => (Some(header), None),
            Err(problem) => (None, Some(problem)),
        };
        let preview = header.as_ref().and_then(|header| {
            let png = &header.screenshot_png;
            let reader =
                image::ImageReader::with_format(io::Cursor::new(png), image::ImageFormat::Png);
            let (width, height) = reader.into_dimensions().ok()?;
            Some(([width, height], egui::util::hash(png)))
        });
        catalog.entries.push(PlayerSave {
            quicksave: quicksave
                .as_ref()
                .is_some_and(|save| same_save(save, &path)),
            path,
            header,
            problem,
            preview,
        });
    }
    catalog.entries.sort_by(|a, b| {
        b.quicksave
            .cmp(&a.quicksave)
            .then_with(|| b.timestamp().cmp(&a.timestamp()))
            .then_with(|| b.path.cmp(&a.path))
    });
    Ok(catalog)
}

fn is_save_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}

pub(crate) fn same_save(a: &Path, b: &Path) -> bool {
    // Win32 ordinarily opens case variants of a basename in the same directory.
    a == b
        || (cfg!(windows)
            && a.canonicalize()
                .ok()
                .is_some_and(|path| b.canonicalize().is_ok_and(|other| path == other)))
}

fn is_initial_state(directory: &Path, manifest: &GameManifest, path: &Path) -> bool {
    manifest
        .initial_state
        .as_deref()
        .and_then(|initial| relative_asset(directory, initial).ok())
        .is_some_and(|initial| same_save(&initial, path))
}

fn pointer_target(directory: &Path, name: &str) -> Result<Option<PathBuf>, String> {
    let name = name.trim();
    if name.is_empty() {
        return Ok(None);
    }
    let path = Path::new(name);
    if !matches!(path.components().next(), Some(Component::Normal(part)) if part == name)
        || !is_save_path(path)
        || name.contains(['/', '\\', ':'])
    {
        return Err("Invalid quicksave filename".into());
    }
    Ok(Some(directory.join(path)))
}

fn read_pointer(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(name) => Ok(Some(name)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn current_quicksave(game_directory: &Path) -> Result<Option<PathBuf>, String> {
    let directory = game_directory.join("saves");
    let pointer = directory.join(QUICKSAVE_FILE);
    let read = |path: &Path| {
        read_pointer(path).map_err(|error| format!("Could not read quicksave designation: {error}"))
    };
    if let Some(name) = read(&pointer)? {
        return pointer_target(&directory, &name);
    }
    let Some(legacy) = read(&directory.join(LEGACY_QUICKSAVE_FILE))? else {
        return Ok(None);
    };
    Ok(pointer_target(&directory, &legacy)
        .ok()
        .flatten()
        .filter(|path| path.is_file()))
}

fn lock_quicksave(directory: &Path) -> Result<fs::File, String> {
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join(".quicksave.lock"))
        .map_err(|error| error.to_string())?;
    lock.try_lock().map_err(|error| error.to_string())?;
    Ok(lock)
}

pub(crate) fn initialize_quicksave(game: &LibraryGame) -> Result<(), String> {
    let directory = game.directory.join("saves");
    let pointer = directory.join(QUICKSAVE_FILE);
    let legacy = directory.join(LEGACY_QUICKSAVE_FILE);
    let needed = || -> io::Result<bool> { Ok(!pointer.try_exists()? && legacy.try_exists()?) };
    let initialize = || {
        if needed().map_err(|error| error.to_string())? {
            let _lock = lock_quicksave(&directory)?;
            if needed().map_err(|error| error.to_string())? {
                let candidate = current_quicksave(&game.directory)?
                    .filter(|path| !is_initial_state(&game.directory, &game.manifest, path));
                let name = candidate
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                replace_file(&pointer, name.as_bytes())?;
            }
        }
        Ok::<(), String>(())
    };
    initialize()
        .map_err(|error| format!("Could not record the legacy quicksave designation: {error}"))
}

fn stage(directory: &Path, bytes: &[u8]) -> io::Result<tempfile::NamedTempFile> {
    let mut pending = tempfile::Builder::new()
        .suffix(".pending")
        .tempfile_in(directory.canonicalize()?)?;
    pending.write_all(bytes)?;
    pending.as_file().sync_all()?;
    Ok(pending)
}

fn replace_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let directory = path.parent().ok_or("The file has no parent directory")?;
    let pending = stage(directory, bytes).map_err(|error| error.to_string())?;
    let name = path.file_name().ok_or("The file has no name")?;
    let target = pending.path().with_file_name(name);
    pending
        .persist(target)
        .map_err(|error| error.error.to_string())?;
    Ok(())
}

pub fn persist_save(game: &LibraryGame, json: &str) -> Result<PathBuf, String> {
    let header = inspect_save(json.as_bytes(), &game.manifest)?;
    let directory = game.directory.join("saves");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let mut pending = stage(&directory, json.as_bytes()).map_err(|error| error.to_string())?;
    for serial in 0..u32::MAX {
        let path = directory.join(format!("{}-{serial}.json", header.timestamp));
        if is_initial_state(&game.directory, &game.manifest, &path) {
            continue;
        }
        // The temporary file's directory is canonical so Win32 also accepts long paths.
        let filename = path
            .file_name()
            .ok_or("Could not allocate a save filename")?;
        let target = pending.path().with_file_name(filename);
        match pending.persist_noclobber(target) {
            Ok(_) => return Ok(path),
            Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {
                pending = error.file
            }
            Err(error) => return Err(error.error.to_string()),
        }
    }
    Err("No available save filename".into())
}

pub fn persist_quicksave(game: &LibraryGame, json: &str) -> Result<PathBuf, String> {
    let path =
        persist_save(game, json).map_err(|error| format!("Could not write quicksave: {error}"))?;
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    let directory = game.directory.join("saves");
    let designate = || {
        let _lock = lock_quicksave(&directory)?;
        replace_file(&directory.join(QUICKSAVE_FILE), filename.as_bytes())
    };
    designate().map_err(|error| {
        format!("Saved {filename} but could not update the quicksave designation: {error}")
    })?;
    Ok(path)
}

fn check_user_save(directory: &Path, game_id: &str, path: &Path) -> Result<(), String> {
    let manifest = load_manifest(directory)?;
    if manifest.id != game_id {
        return Err("The game entry changed on disk".into());
    }
    if path.parent() != Some(directory.join("saves").as_path())
        || !is_save_path(path)
        || is_initial_state(directory, &manifest, path)
    {
        return Err("The selected file is not a user save for this game".into());
    }
    Ok(())
}

pub(crate) fn edit_save_description(
    game: &LibraryGame,
    path: &Path,
    text: &str,
) -> Result<(), String> {
    check_user_save(&game.directory, &game.manifest.id, path)?;
    let _lock = lock_quicksave(&game.directory.join("saves"))?;
    let (bytes, _) = read_associated_save(path, &game.manifest)?;
    let mut document: BTreeMap<String, Box<RawValue>> =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    let header = document.get("header").ok_or("The save header is missing")?;
    let mut header: BTreeMap<String, Box<RawValue>> =
        serde_json::from_str(header.get()).map_err(|error| error.to_string())?;
    let description = text.trim();
    if description.is_empty() {
        header.remove("description");
    } else {
        let value = to_raw_value(description).map_err(|error| error.to_string())?;
        header.insert("description".into(), value);
    }
    document.insert(
        "header".into(),
        to_raw_value(&header).map_err(|error| error.to_string())?,
    );
    let updated = serde_json::to_vec(&document).map_err(|error| error.to_string())?;
    if fs::read(path).map_err(|error| error.to_string())? != bytes {
        return Err("The save changed on disk. Reopen the editor and try again.".into());
    }
    replace_file(path, &updated)
}

pub(crate) fn delete_save(directory: &Path, game_id: &str, path: &Path) -> Result<(), String> {
    check_user_save(directory, game_id, path)?;
    let saves_directory = directory.join("saves");
    let _lock = lock_quicksave(&saves_directory)?;
    let designation = current_quicksave(directory);
    let was_quicksave = designation.as_ref().is_ok_and(|selected| {
        selected
            .as_ref()
            .is_some_and(|selected| same_save(selected, path))
    });
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    let cleanup = || {
        designation?;
        if was_quicksave {
            replace_file(&saves_directory.join(QUICKSAVE_FILE), b"")?;
        }
        Ok::<(), String>(())
    };
    cleanup().map_err(|error| {
        format!("The save is gone but its quicksave designation could not be reconciled: {error}")
    })
}
