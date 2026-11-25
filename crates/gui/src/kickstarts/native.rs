//! File-backed Kickstart entries in a native Game Library.

use super::{validate_size, Entry};
use crate::library_catalog::{rom_path, CatalogWrite};
use crate::player_config::ConfigurationCatalog;
use debugger::SourceImage;
use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

const DIRECTORY: &str = "kickstarts";
const LEGACY_REFERENCE: &str = "_shared/kickstart-1.3.rom";
const ADOPTED_NAME: &str = "kickstart-1.3.rom";

pub(crate) fn read_image(path: &Path) -> Result<SourceImage, String> {
    let result = || -> io::Result<_> {
        let file = fs::File::open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(io::Error::other("expected a file"));
        }
        Ok((file, metadata.len()))
    };
    let location = path.display().to_string();
    let (file, size) = result().map_err(|error| format!("Could not read {location}: {error}"))?;
    validate_size(&location, size)?;
    let mut bytes = Vec::with_capacity(physical_memory::KICKSTART_ROM_SIZE);
    file.take(size + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read {location}: {error}"))?;
    validate_size(&location, bytes.len() as u64)?;
    Ok(SourceImage {
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()),
        bytes,
    })
}

fn managed_paths(root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let directory = root.join(DIRECTORY);
    let items = match fs::read_dir(&directory) {
        Ok(items) => items,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not read {}: {error}", directory.display())),
    };
    let mut paths = Vec::new();
    for item in items {
        let item =
            item.map_err(|error| format!("Could not read {}: {error}", directory.display()))?;
        let filename = item.file_name();
        if let Some(name) = filename.to_str().filter(|name| managed_name(name)) {
            let kind = item
                .file_type()
                .map_err(|error| format!("Could not inspect {}: {error}", item.path().display()))?;
            if kind.is_file() {
                paths.push((format!("{DIRECTORY}/{name}"), item.path()));
            }
        }
    }
    paths.sort_unstable();
    Ok(paths)
}

fn managed_name(name: &str) -> bool {
    !name.contains(['\\', ':']) && !(name.starts_with(".kickstart-") && name.ends_with(".pending"))
}

pub(super) fn entries(root: &Path) -> Result<(Vec<Entry>, Vec<String>), String> {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    for (reference, path) in managed_paths(root)? {
        match entry(reference, &path) {
            Ok(entry) => entries.push(entry),
            Err(error) => errors.push(error),
        }
    }
    Ok((entries, errors))
}

pub(super) fn entry(reference: String, path: &Path) -> Result<Entry, String> {
    let image = read_image(path)?;
    Ok(Entry::native(
        reference,
        image.name.unwrap_or_default(),
        &image.bytes,
    ))
}

pub(crate) fn extend_restore_roms(root: &Path, roms: &mut Vec<SourceImage>) {
    if let Ok(paths) = managed_paths(root) {
        roms.extend(
            paths
                .into_iter()
                .filter_map(|(_, path)| read_image(&path).ok()),
        );
    }
}

pub(super) fn import(root: &Path, source: &Path) -> Result<(), String> {
    let image = read_image(source)?;
    let filename = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| managed_name(name))
        .ok_or("The Kickstart filename cannot be used in the library")?;
    let directory = prepare_directory(root)?;
    let (stem, suffix) = filename
        .rsplit_once('.')
        .map_or((filename, String::new()), |(stem, suffix)| {
            (stem, format!(".{suffix}"))
        });
    for number in 1..=u32::MAX {
        let destination = if number == 1 {
            directory.join(filename)
        } else {
            directory.join(format!("{stem} ({number}){suffix}"))
        };
        match publish_copy(&destination, &image.bytes) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                if same_image(&destination, &image.bytes) {
                    return Ok(());
                }
            }
            Err(error) => {
                return Err(format!(
                    "Could not copy Kickstart to {}: {error}",
                    destination.display()
                ))
            }
        }
    }
    Err("No available Kickstart filename".into())
}

pub(crate) fn adopt_shared(
    root: &Path,
    catalog: &mut ConfigurationCatalog,
    original: Option<&[u8]>,
) -> Result<Option<Vec<u8>>, String> {
    let source = root.join(LEGACY_REFERENCE);
    if !catalog
        .configurations
        .iter()
        .any(|profile| references(root, &profile.launch.rom, &source))
    {
        return Ok(None);
    }
    let image = read_image(&source)?;
    let destination = prepare_directory(root)?.join(ADOPTED_NAME);
    if let Err(error) = publish_copy(&destination, &image.bytes) {
        if error.kind() != ErrorKind::AlreadyExists || !same_image(&destination, &image.bytes) {
            return Err(format!(
                "Could not adopt {LEGACY_REFERENCE} at {}: {error}",
                destination.display()
            ));
        }
    }
    if !same_image(&destination, &image.bytes) {
        return Err(format!(
            "Managed copy differs from {LEGACY_REFERENCE}: {}",
            destination.display()
        ));
    }
    let mut updated = catalog.clone();
    for profile in &mut updated.configurations {
        if references(root, &profile.launch.rom, &source) {
            profile.launch.rom = format!("{DIRECTORY}/{ADOPTED_NAME}");
        }
    }
    let published = CatalogWrite::prepare(root, original, &updated)
        .and_then(CatalogWrite::publish)
        .map_err(|error| {
            format!("Could not update configurations using {LEGACY_REFERENCE}: {error}")
        })?;
    *catalog = updated;
    Ok(Some(published))
}

fn prepare_directory(root: &Path) -> Result<PathBuf, String> {
    let directory = root.join(DIRECTORY);
    match fs::create_dir(&directory) {
        Ok(()) => Ok(directory),
        Err(error) if error.kind() == ErrorKind::AlreadyExists && directory.is_dir() => {
            Ok(directory)
        }
        Err(error) => Err(format!("Could not create {}: {error}", directory.display())),
    }
}

fn same_image(path: &Path, bytes: &[u8]) -> bool {
    read_image(path).is_ok_and(|image| image.bytes == bytes)
}

fn publish_copy(path: &Path, bytes: &[u8]) -> io::Result<()> {
    // Publishing a complete temporary file keeps interrupted imports out of the list.
    let Some(directory) = path.parent() else {
        return Err(io::Error::other("Kickstart directory is missing"));
    };
    if path.try_exists()? {
        return Err(ErrorKind::AlreadyExists.into());
    }
    let mut pending = tempfile::Builder::new()
        .prefix(".kickstart-")
        .suffix(".pending")
        .tempfile_in(directory)?;
    pending.write_all(bytes)?;
    pending.as_file().sync_all()?;
    pending
        .persist_noclobber(path)
        .map_err(|error| error.error)?;
    Ok(())
}

pub(crate) fn managed_path(root: &Path, reference: &str) -> Result<PathBuf, String> {
    let path = rom_path(root, reference)?;
    if path.parent() == Some(root.join(DIRECTORY).as_path()) {
        Ok(path)
    } else {
        Err("Only managed Kickstarts can be deleted".into())
    }
}

pub(super) fn references(root: &Path, reference: &str, path: &Path) -> bool {
    rom_path(root, reference).is_ok_and(|candidate| {
        candidate == path
            || cfg!(windows) && candidate.as_os_str().eq_ignore_ascii_case(path.as_os_str())
    })
}
