//! Desktop library drafts and explicit import/rename operations.

use crate::disk_import::{read_disk_path, DiskImage};
use crate::game_library::{
    load_game, persist_numbered_png, relative_asset, validate_disk_filename, write_manifest,
    write_rgba_png, GameDisk, GameManifest, LibraryGame,
};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ImportedDisk {
    key: u64,
    image: DiskImage,
}

impl ImportedDisk {
    pub fn key(&self) -> u64 {
        self.key
    }

    pub(crate) fn filename(&self) -> &str {
        &self.image.filename
    }

    pub(crate) fn source(&self) -> std::borrow::Cow<'_, str> {
        self.image.description()
    }
}

#[derive(Debug)]
pub struct SavedGame {
    pub game: LibraryGame,
    pub import_issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupDisk {
    Existing(String),
    Pending(u64),
}

#[derive(Debug, Clone)]
pub(crate) struct DraftArtwork {
    pub(crate) image: image::RgbaImage,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone)]
pub struct GameDraft {
    pub original: Option<LibraryGame>,
    pub name: String,
    pub description: String,
    pub configuration: String,
    pub(crate) artwork: Option<DraftArtwork>,
    pub(crate) artwork_error: Option<String>,
    imports: Vec<ImportedDisk>,
    next_import: u64,
    startup: [Option<StartupDisk>; config::FLOPPY_DRIVE_COUNT],
}

impl GameDraft {
    pub fn new(configuration: String) -> Self {
        Self {
            original: None,
            name: String::new(),
            description: String::new(),
            configuration,
            artwork: None,
            artwork_error: None,
            imports: Vec::new(),
            next_import: 0,
            startup: Default::default(),
        }
    }

    pub fn edit(game: LibraryGame) -> Self {
        Self {
            name: game.manifest.title.clone(),
            description: game.description.clone(),
            configuration: game.manifest.configuration.clone(),
            startup: game
                .manifest
                .mounted
                .clone()
                .map(|id| id.map(StartupDisk::Existing)),
            original: Some(game),
            artwork: None,
            artwork_error: None,
            imports: Vec::new(),
            next_import: 0,
        }
    }

    pub fn imports(&self) -> &[ImportedDisk] {
        &self.imports
    }

    pub fn choose_artwork(&mut self, path: &Path) -> Result<(), String> {
        let bytes =
            std::fs::read(path).map_err(|error| format!("Could not open image: {error}"))?;
        self.set_artwork(decode_artwork(&bytes)?)
    }

    pub(crate) fn set_artwork(&mut self, image: image::RgbaImage) -> Result<(), String> {
        let (width, height) = image.dimensions();
        if width == 0 || height == 0 {
            return Err("The image has no pixels".into());
        }
        let revision = egui::util::hash((width, height, image.as_raw()));
        self.artwork = Some(DraftArtwork { image, revision });
        Ok(())
    }

    /// Inspect all selections and keep usable images even when other inputs fail.
    pub fn add_disks(&mut self, paths: &[PathBuf]) -> Vec<String> {
        let mut issues = Vec::new();
        for path in paths {
            let selection = read_disk_path(path);
            issues.extend(selection.issues);
            for image in selection.images {
                if let Err(issue) = self.check_import(&image) {
                    issues.push(format!("{}: {issue}", image.description()));
                    continue;
                }
                self.imports.push(ImportedDisk {
                    key: self.next_import,
                    image,
                });
                self.next_import += 1;
            }
        }
        issues
    }

    fn check_import(&self, image: &DiskImage) -> Result<(), String> {
        validate_disk_filename(&image.filename)?;
        if self
            .imports
            .iter()
            .any(|disk| disk.image.bytes == image.bytes)
        {
            return Err("Disk already added".into());
        }
        if let Some(game) = &self.original {
            if duplicate_id(&game.directory, &game.manifest, &image.bytes)?.is_some() {
                return Err("Disk already added".into());
            }
            if filename_is_occupied(&game.directory, &game.manifest, &image.filename)? {
                return Err(filename_collision(&image.filename));
            }
        }
        if self
            .imports
            .iter()
            .any(|disk| disk.filename() == image.filename)
        {
            return Err(filename_collision(&image.filename));
        }
        Ok(())
    }

    pub fn remove_import(&mut self, key: u64) {
        self.imports.retain(|disk| disk.key != key);
        for selection in &mut self.startup {
            if *selection == Some(StartupDisk::Pending(key)) {
                *selection = None;
            }
        }
    }

    pub fn startup_choices(&self) -> impl Iterator<Item = (StartupDisk, &str)> {
        self.original
            .iter()
            .flat_map(|game| &game.manifest.disks)
            .map(|disk| {
                (
                    StartupDisk::Existing(disk.id.clone()),
                    disk.filename.as_str(),
                )
            })
            .chain(
                self.imports
                    .iter()
                    .map(|disk| (StartupDisk::Pending(disk.key), disk.filename())),
            )
    }

    pub fn startup_disk(&self, drive: usize) -> Option<StartupDisk> {
        self.startup[drive].clone()
    }

    pub fn select_startup_disk(&mut self, drive: usize, disk: Option<StartupDisk>) {
        self.startup[drive] = disk;
    }

    pub fn save(
        &self,
        root: &Path,
        connected: [bool; config::FLOPPY_DRIVE_COUNT],
    ) -> Result<SavedGame, String> {
        self.save_with_rename_allowed(root, connected, true)
    }

    pub(crate) fn save_with_rename_allowed(
        &self,
        root: &Path,
        connected: [bool; config::FLOPPY_DRIVE_COUNT],
        rename_allowed: bool,
    ) -> Result<SavedGame, String> {
        validate_game_name(&self.name)?;
        if self.configuration.trim().is_empty() {
            return Err("Select a named configuration".into());
        }
        for (drive, disk) in self.startup.iter().enumerate() {
            if let Some(disk) = disk {
                if !connected[drive] {
                    return Err(format!("DF{drive} is disconnected in the selected configuration. Choose Empty or another configuration."));
                }
                if !self.startup_choices().any(|(choice, _)| choice == *disk) {
                    return Err(format!(
                        "The startup disk selected for DF{drive} is unavailable"
                    ));
                }
            }
        }
        let root = root
            .canonicalize()
            .map_err(|e| format!("Choose an existing library directory: {e}"))?;
        let original_directory = self
            .original
            .as_ref()
            .map(|game| game.directory.canonicalize())
            .transpose()
            .map_err(|e| e.to_string())?;
        if let Some(directory) = &original_directory {
            if directory.parent() != Some(root.as_path()) {
                return Err("The edited game must belong to this library".into());
            }
        }
        let destination = match self.original.as_ref().zip(original_directory.as_ref()) {
            Some((game, directory)) if !rename_allowed => {
                if self.name != game.manifest.title {
                    return Err(
                        "Undo pending save deletions or wait for them before renaming this game"
                            .into(),
                    );
                }
                directory.clone()
            }
            _ => root.join(&self.name),
        };
        if original_directory.as_ref() != Some(&destination) && destination.exists() {
            return Err("A folder with this game name already exists; choose another name".into());
        }
        let mut manifest = if let Some(game) = &self.original {
            let current = load_game(&game.directory)?;
            if current.manifest != game.manifest || current.description != game.description {
                return Err("Game files changed on disk; reopen Edit game before saving".into());
            }
            game.manifest.clone()
        } else {
            let library = crate::game_library::GameLibrary::discover(&root)?;
            let mut serial = 1u64;
            let id = loop {
                let id = format!("game-{serial}");
                if !library.games.iter().any(|game| game.manifest.id == id) {
                    break id;
                }
                serial += 1;
            };
            GameManifest {
                id,
                title: self.name.clone(),
                configuration: self.configuration.clone(),
                disks: Vec::new(),
                mounted: [None, None, None, None],
                initial_state: None,
                start_states: Vec::new(),
                artwork: None,
                description: None,
            }
        };
        manifest.title.clone_from(&self.name);
        manifest.configuration.clone_from(&self.configuration);
        let import_issues = if let Some(directory) = original_directory {
            let issues = self.write_assets(&directory, &mut manifest)?;
            if directory != destination {
                std::fs::rename(&directory, &destination).map_err(|e| format!("Changes saved, but the folder could not be renamed: {e}. The library will pick up the saved changes."))?;
            }
            issues
        } else {
            // The library only discovers direct children. Publish the complete game by renaming it out of the temporary container.
            let temporary = tempfile::Builder::new()
                .prefix(".game-import-")
                .tempdir_in(&root)
                .map_err(|error| error.to_string())?;
            let staging = temporary.path().join("game");
            std::fs::create_dir(&staging).map_err(|e| e.to_string())?;
            let result = self
                .write_assets(&staging, &mut manifest)
                .and_then(|issues| {
                    std::fs::rename(&staging, &destination).map_err(|e| e.to_string())?;
                    Ok(issues)
                });
            match result {
                Ok(issues) => issues,
                Err(error) => {
                    if let Err(cleanup) = temporary.close() {
                        return Err(format!(
                            "{error}; temporary game folder could not be removed: {cleanup}"
                        ));
                    }
                    return Err(error);
                }
            }
        };
        Ok(SavedGame {
            game: load_game(&destination)?,
            import_issues,
        })
    }

    fn write_assets(
        &self,
        directory: &Path,
        manifest: &mut GameManifest,
    ) -> Result<Vec<String>, String> {
        let mut new_files = Vec::new();
        let result = self.write_game(directory, manifest, &mut new_files);
        if let Err(mut error) = result {
            for path in new_files {
                if let Err(cleanup) = std::fs::remove_file(&path) {
                    error.push_str(&format!(". Could not remove {}: {cleanup}", path.display()));
                }
            }
            return Err(error);
        }
        result
    }

    fn write_game(
        &self,
        directory: &Path,
        manifest: &mut GameManifest,
        new_files: &mut Vec<PathBuf>,
    ) -> Result<Vec<String>, String> {
        manifest.mounted = self.startup.each_ref().map(|disk| match disk {
            Some(StartupDisk::Existing(id)) => Some(id.clone()),
            _ => None,
        });
        let mut issues = Vec::new();
        for disk in &self.imports {
            let id = if let Some(id) = duplicate_id(directory, manifest, &disk.image.bytes)? {
                issues.push(format!("{}: Disk already added", disk.source()));
                id.to_owned()
            } else {
                match publish_disk(directory, manifest, disk, new_files)? {
                    Some(id) => id,
                    None => {
                        let issue =
                            format!("{}: {}", disk.source(), filename_collision(disk.filename()));
                        if self.startup.contains(&Some(StartupDisk::Pending(disk.key))) {
                            return Err(format!(
                                "{issue}. Remove the conflicting startup selection before saving."
                            ));
                        }
                        issues.push(issue);
                        continue;
                    }
                }
            };
            for (drive, selection) in self.startup.iter().enumerate() {
                if *selection == Some(StartupDisk::Pending(disk.key)) {
                    manifest.mounted[drive] = Some(id.clone());
                }
            }
        }
        let changed_description = self
            .original
            .as_ref()
            .is_none_or(|game| game.description != self.description);
        if changed_description && (!self.description.is_empty() || manifest.description.is_some()) {
            let path = manifest
                .description
                .clone()
                .unwrap_or_else(|| "description.md".into());
            let target = relative_asset(directory, &path)?;
            if manifest.description.is_none() && target.exists() {
                return Err("An unassigned description.md already exists; preserve or rename it before saving".into());
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(target, &self.description).map_err(|e| e.to_string())?;
            manifest.description = Some(path);
        }
        manifest.validate()?;
        let new_artwork = self
            .artwork
            .as_ref()
            .map(|artwork| {
                let image = &artwork.image;
                persist_numbered_png(directory, "artwork", |file| {
                    write_rgba_png(file, image.as_raw(), image.width(), image.height())
                })
                .map_err(|error| format!("Could not store the cover image: {error}"))
            })
            .transpose()?;
        if let Some(path) = &new_artwork {
            manifest.artwork = Some(path.clone());
        }
        let saved = serde_json::to_vec_pretty(manifest)
            .map_err(|error| error.to_string())
            .and_then(|bytes| write_manifest(directory, &bytes));
        if let Err(error) = saved {
            if let Some(path) = new_artwork {
                std::fs::remove_file(directory.join(path)).map_err(|cleanup| {
                    format!("{error}. Could not remove the unselected cover image: {cleanup}")
                })?;
            }
            return Err(error);
        }
        Ok(issues)
    }
}

fn duplicate_id<'a>(
    directory: &Path,
    manifest: &'a GameManifest,
    bytes: &[u8],
) -> Result<Option<&'a str>, String> {
    let mut buffer = vec![0; bytes.len()];
    for disk in &manifest.disks {
        let path = disk.asset_path(directory)?;
        let mut compare = || -> std::io::Result<bool> {
            let mut file = std::fs::File::open(&path)?;
            let metadata = file.metadata()?;
            if !metadata.is_file() || metadata.len() != bytes.len() as u64 {
                return Ok(false);
            }
            file.read_exact(&mut buffer)?;
            let mut trailing = [0];
            Ok(buffer == bytes && file.read(&mut trailing)? == 0)
        };
        match compare() {
            Ok(true) => return Ok(Some(&disk.id)),
            Ok(false) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(format!("Could not compare {}: {error}", path.display())),
        }
    }
    Ok(None)
}

fn filename_collision(filename: &str) -> String {
    format!("Disk filename collision: {filename}")
}

fn filename_is_occupied(
    directory: &Path,
    manifest: &GameManifest,
    filename: &str,
) -> Result<bool, String> {
    if manifest.disks.iter().any(|disk| disk.filename == filename) {
        return Ok(true);
    }
    directory
        .join("disks")
        .join(filename)
        .try_exists()
        .map_err(|error| format!("Could not check destination for {filename}: {error}"))
}

fn publish_disk(
    directory: &Path,
    manifest: &mut GameManifest,
    disk: &ImportedDisk,
    new_files: &mut Vec<PathBuf>,
) -> Result<Option<String>, String> {
    if filename_is_occupied(directory, manifest, disk.filename())? {
        return Ok(None);
    }
    let id = (1u64..)
        .map(|serial| format!("disk-{serial}"))
        .find(|candidate| manifest.disks.iter().all(|disk| &disk.id != candidate))
        .ok_or("Could not allocate a disk ID")?;
    let asset = GameDisk {
        id: id.clone(),
        filename: disk.filename().into(),
    };
    let path = asset.asset_path(directory)?;
    let folder = directory.join("disks");
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let pending = tempfile::NamedTempFile::new_in(&folder)
        .and_then(|mut pending| {
            pending.write_all(&disk.image.bytes)?;
            Ok(pending)
        })
        .map_err(|error| format!("Could not write {}: {error}", path.display()))?;
    match pending.persist_noclobber(&path) {
        Ok(_) => {
            new_files.push(path);
            manifest.disks.push(asset);
            Ok(Some(id))
        }
        Err(error) if error.error.kind() == ErrorKind::AlreadyExists => Ok(None),
        Err(error) => Err(format!("Could not store {}: {error}", path.display())),
    }
}

pub(crate) fn decode_artwork(bytes: &[u8]) -> Result<image::RgbaImage, String> {
    let format = match image::guess_format(bytes) {
        Ok(format @ (image::ImageFormat::Jpeg | image::ImageFormat::Png)) => format,
        _ => return Err("Choose a JPEG or PNG image".into()),
    };
    image::load_from_memory_with_format(bytes, format)
        .map(image::DynamicImage::into_rgba8)
        .map_err(|error| format!("Could not read image: {error}"))
}

/// Preserve the entered name exactly while rejecting unsafe Windows folder names.
pub fn validate_game_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.trim() != name
        || name.ends_with('.')
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || matches!(name, "." | "..")
    {
        return Err("Enter a folder name without path separators, reserved characters, or surrounding spaces/dots".into());
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.len() == 1 && suffix.as_bytes()[0].is_ascii_digit() && suffix != "0"
            })
        })
    {
        return Err("This folder name is reserved by Windows; choose another name".into());
    }
    Ok(())
}
