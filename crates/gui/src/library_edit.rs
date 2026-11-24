//! Desktop library drafts and explicit import/rename operations.

use crate::game_library::{
    load_game, persist_numbered_png, relative_asset, write_manifest, write_rgba_png, GameDisk,
    GameManifest, LibraryGame,
};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ImportedDisk {
    key: u64,
    pub name: String,
    bytes: Vec<u8>,
}

impl ImportedDisk {
    pub fn key(&self) -> u64 {
        self.key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupDisk {
    Existing(String),
    Pending(u64),
}

#[derive(Debug, Clone)]
pub(crate) struct DraftArtwork {
    pub(crate) image: image::RgbaImage,
    pub(crate) revision: egui::Id,
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
        let revision = egui::Id::new((width, height, image.as_raw()));
        self.artwork = Some(DraftArtwork { image, revision });
        Ok(())
    }

    /// Read and validate before the draft can affect library files.
    pub fn add_disk(&mut self, path: &Path) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        if !debugger::launch::disk_image_is_supported(&bytes) {
            return Err(format!(
                "Unsupported ADF length: {} bytes. Select a standard 880 KiB ADF.",
                bytes.len()
            ));
        }
        self.imports.push(ImportedDisk {
            key: self.next_import,
            name: path
                .file_name()
                .ok_or("Select a disk file")?
                .to_string_lossy()
                .into_owned(),
            bytes,
        });
        self.next_import += 1;
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
            .map(|disk| (StartupDisk::Existing(disk.id.clone()), disk.display_name()))
            .chain(
                self.imports
                    .iter()
                    .map(|disk| (StartupDisk::Pending(disk.key), disk.name.as_str())),
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
    ) -> Result<LibraryGame, String> {
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
        let destination = root.join(&self.name);
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
                artwork: None,
                description: None,
            }
        };
        manifest.title.clone_from(&self.name);
        manifest.configuration.clone_from(&self.configuration);
        if let Some(directory) = original_directory {
            self.write_assets(&directory, &mut manifest)?;
            if directory != destination {
                std::fs::rename(&directory, &destination).map_err(|e| format!("Changes saved, but the folder could not be renamed: {e}. Refresh the library."))?;
            }
        } else {
            // Only the fresh staging directory is disposable; existing game data never are.
            let staging = root.join(format!(".{}.pending", manifest.id));
            std::fs::create_dir(&staging).map_err(|e| e.to_string())?;
            let result = self
                .write_assets(&staging, &mut manifest)
                .and_then(|()| std::fs::rename(&staging, &destination).map_err(|e| e.to_string()));
            if let Err(error) = result {
                // Both paths have the canonical library root as their immediate parent.
                if staging.parent() == Some(root.as_path()) {
                    if let Err(cleanup) = std::fs::remove_dir_all(&staging) {
                        return Err(format!(
                            "{error}; temporary game folder could not be removed: {cleanup}"
                        ));
                    }
                }
                return Err(error);
            }
        }
        load_game(&destination)
    }

    fn write_assets(&self, directory: &Path, manifest: &mut GameManifest) -> Result<(), String> {
        use std::io::Write;
        manifest.mounted = self.startup.each_ref().map(|disk| match disk {
            Some(StartupDisk::Existing(id)) => Some(id.clone()),
            _ => None,
        });
        if !self.imports.is_empty() {
            std::fs::create_dir_all(directory.join("disks")).map_err(|e| e.to_string())?;
        }
        for disk in &self.imports {
            let mut serial = 1u64;
            let (id, path) = loop {
                let id = format!("disk-{serial}");
                let path = format!("disks/{id}.adf");
                if !manifest.disks.iter().any(|disk| disk.id == id)
                    && !directory.join(&path).exists()
                {
                    break (id, path);
                }
                serial += 1;
            };
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join(&path))
                .map_err(|e| e.to_string())?;
            output.write_all(&disk.bytes).map_err(|e| e.to_string())?;
            for (drive, selection) in self.startup.iter().enumerate() {
                if *selection == Some(StartupDisk::Pending(disk.key)) {
                    manifest.mounted[drive] = Some(id.clone());
                }
            }
            manifest.disks.push(GameDisk {
                id,
                label: disk.name.clone(),
                path,
            });
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
        Ok(())
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
