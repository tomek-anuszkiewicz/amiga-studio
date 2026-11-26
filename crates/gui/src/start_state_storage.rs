//! Game-owned start-state files and manifest updates.

use crate::game_library::{
    current_quicksave, read_associated_save, same_save, write_manifest_if_current, GameManifest,
    GameStartState, LibraryGame,
};
use std::collections::HashMap;
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

const START_STATES_DIRECTORY: &str = "initial_states";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartStateTarget {
    pub directory: PathBuf,
    pub game_id: String,
    pub id: String,
}

impl StartStateTarget {
    pub(crate) fn new(game: &LibraryGame, id: String) -> Self {
        Self {
            directory: game.directory.clone(),
            game_id: game.manifest.id.clone(),
            id,
        }
    }

    pub(crate) fn resolve(&self) -> Result<(PathBuf, GameManifest), String> {
        let (_, manifest) = read_manifest(&self.directory, &self.game_id)?;
        let path = entry_path(&self.directory, &manifest, &self.id)?;
        Ok((path, manifest))
    }

    pub(crate) fn rename(&self, name: String) -> Result<GameManifest, String> {
        let (before, mut manifest) = read_manifest(&self.directory, &self.game_id)?;
        let index = entry_index(&manifest, &self.id)?;
        manifest.start_states[index].name = name;
        publish(&self.directory, &before, &manifest.start_states)?;
        Ok(manifest)
    }

    pub(crate) fn delete(&self) -> Result<GameManifest, String> {
        let (before, mut manifest) = read_manifest(&self.directory, &self.game_id)?;
        let index = entry_index(&manifest, &self.id)?;
        let path = entry_path(&self.directory, &manifest, &self.id)?;
        let remove_file = match path.metadata() {
            Ok(metadata) if metadata.is_file() => true,
            Ok(_) => return Err("The start-state path does not refer to a file".into()),
            Err(error) if error.kind() == ErrorKind::NotFound => false,
            Err(error) => return Err(error.to_string()),
        };

        manifest.start_states.remove(index);
        let updated = publish(&self.directory, &before, &manifest.start_states)?;
        if remove_file {
            if let Err(error) = std::fs::remove_file(path) {
                if error.kind() != ErrorKind::NotFound {
                    return match write_manifest_if_current(
                        &self.directory,
                        &before,
                        Some(&updated),
                    ) {
                        Ok(()) => Err(error.to_string()),
                        Err(rollback) => Err(format!(
                            "The entry was removed but its file could not be deleted: {error}. Could not restore the entry: {rollback}"
                        )),
                    };
                }
            }
        }
        Ok(manifest)
    }
}

pub fn promote(
    directory: &Path,
    game_id: &str,
    source: &Path,
    quicksave: bool,
    name: String,
) -> Result<GameManifest, String> {
    let (before, mut manifest) = read_manifest(directory, game_id)?;
    if source.parent() != Some(directory.join("saves").as_path()) {
        return Err("Choose a save from this game".into());
    }
    if quicksave
        && !current_quicksave(directory)?
            .as_deref()
            .is_some_and(|selected| same_save(selected, source))
    {
        return Err("The current quicksave changed. Select it again.".into());
    }
    let (bytes, _) = read_associated_save(source, &manifest)?;

    let storage = directory.join(START_STATES_DIRECTORY);
    std::fs::create_dir_all(&storage).map_err(|error| error.to_string())?;
    let mut pending = tempfile::Builder::new()
        .prefix("start-")
        .rand_bytes(16)
        .suffix(".pending")
        .tempfile_in(&storage)
        .map_err(|error| error.to_string())?;
    let id = pending
        .path()
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or("Could not allocate a start-state filename")?
        .to_owned();
    manifest.start_states.push(GameStartState {
        id: id.clone(),
        name,
    });
    let path = entry_path(directory, &manifest, &id)?;
    pending
        .write_all(&bytes)
        .and_then(|()| pending.as_file().sync_all())
        .map_err(|error| error.to_string())?;
    let absolute = std::path::absolute(&path).map_err(|error| error.to_string())?;
    pending
        .persist_noclobber(absolute)
        .map_err(|error| error.error.to_string())?;
    if let Err(error) = publish(directory, &before, &manifest.start_states) {
        return match std::fs::remove_file(path) {
            Ok(()) => Err(error),
            Err(cleanup) => Err(format!(
                "{error}. Could not remove the unpublished start-state file: {cleanup}"
            )),
        };
    }
    Ok(manifest)
}

pub fn unavailable(game: &LibraryGame) -> HashMap<String, String> {
    game.manifest
        .start_states
        .iter()
        .filter_map(|entry| {
            entry_path(&game.directory, &game.manifest, &entry.id)
                .and_then(|path| read_associated_save(&path, &game.manifest))
                .err()
                .map(|error| (entry.id.clone(), error))
        })
        .collect()
}

fn read_manifest(directory: &Path, expected_id: &str) -> Result<(Vec<u8>, GameManifest), String> {
    let bytes = std::fs::read(directory.join("game.json")).map_err(|error| error.to_string())?;
    let manifest: GameManifest =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    manifest.validate()?;
    if manifest.id != expected_id {
        return Err("The game entry changed on disk".into());
    }
    Ok((bytes, manifest))
}

fn publish(directory: &Path, before: &[u8], states: &[GameStartState]) -> Result<Vec<u8>, String> {
    let mut document: serde_json::Value =
        serde_json::from_slice(before).map_err(|error| error.to_string())?;
    document["start_states"] = serde_json::to_value(states).map_err(|error| error.to_string())?;
    let updated = serde_json::to_vec_pretty(&document).map_err(|error| error.to_string())?;
    write_manifest_if_current(directory, &updated, Some(before))?;
    Ok(updated)
}

fn entry_index(manifest: &GameManifest, id: &str) -> Result<usize, String> {
    let mut matches = manifest
        .start_states
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.id.eq_ignore_ascii_case(id));
    match (matches.next(), matches.next()) {
        (Some((index, entry)), None) if entry.id == id => Ok(index),
        (Some(_), Some(_)) => Err("Several start states use the same file identity".into()),
        _ => Err("This start state is no longer in the game library".into()),
    }
}

fn entry_path(directory: &Path, manifest: &GameManifest, id: &str) -> Result<PathBuf, String> {
    entry_index(manifest, id)?;
    if !id.strip_prefix("start-").is_some_and(|suffix| {
        suffix.len() == 16 && suffix.bytes().all(|byte| byte.is_ascii_alphanumeric())
    }) {
        return Err("Invalid start-state file identity".into());
    }
    let name = format!("{id}.json");
    let relative = Path::new(START_STATES_DIRECTORY).join(&name);
    // Managed disks have their own directory and ADF filenames.
    let existing_assets = [
        &manifest.initial_state,
        &manifest.artwork,
        &manifest.description,
    ];
    if existing_assets.into_iter().flatten().any(|asset| {
        let mut parts = Path::new(asset)
            .components()
            .filter(|part| *part != Component::CurDir);
        matches!(
            (parts.next(), parts.next(), parts.next()),
            (Some(Component::Normal(folder)), Some(Component::Normal(file)), None)
                if folder.to_str().is_some_and(|folder| folder.eq_ignore_ascii_case(START_STATES_DIRECTORY))
                    && file.to_str().is_some_and(|file| file.eq_ignore_ascii_case(&name))
        )
    }) {
        return Err("The start-state path is already used by another game asset".into());
    }
    Ok(directory.join(relative))
}
