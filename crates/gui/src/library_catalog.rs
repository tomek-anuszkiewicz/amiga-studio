//! Native configuration storage and one-time transfer of the previous catalog.

use crate::player_config::{ConfigurationCatalog, JoystickMapping, NamedConfiguration};
use serde::{Deserialize, Serialize};
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

const CATALOG_NAME: &str = "configurations.json";
pub const SETTINGS_KEY: &str = "native_player_settings";
pub type CatalogRead = Result<(ConfigurationCatalog, Option<Vec<u8>>), String>;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct NativePlayerSettings {
    pub root: Option<PathBuf>,
    pub selected: String,
    pub migrated_legacy: bool,
}

#[derive(Deserialize)]
pub struct LegacyCatalog {
    configurations: Vec<NamedConfiguration>,
    mappings: Vec<JoystickMapping>,
    pub selected: String,
    pub library_root: PathBuf,
    #[serde(default)]
    next_mapping_id: u64,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Contents {
    configurations: Vec<NamedConfiguration>,
    mappings: Vec<JoystickMapping>,
    #[serde(default)]
    next_mapping_id: u64,
}

impl Contents {
    fn catalog(self, selected: &str) -> ConfigurationCatalog {
        let selected = self
            .configurations
            .iter()
            .find(|configuration| configuration.id == selected)
            .or_else(|| self.configurations.first())
            .map_or_else(String::new, |configuration| configuration.id.clone());
        ConfigurationCatalog {
            configurations: self.configurations,
            mappings: self.mappings,
            selected,
            next_mapping_id: self.next_mapping_id,
        }
    }
}

pub fn same_profiles(a: &ConfigurationCatalog, b: &ConfigurationCatalog) -> bool {
    a.configurations == b.configurations
        && a.mappings == b.mappings
        && a.next_mapping_id == b.next_mapping_id
}

pub fn open(root: &Path, selected: &str) -> CatalogRead {
    std::fs::read_dir(root).map_err(|error| format!("Could not open library: {error}"))?;
    read(root, selected)
}

pub fn read(root: &Path, selected: &str) -> CatalogRead {
    let path = root.join(CATALOG_NAME);
    let source =
        read_optional(&path).map_err(|error| format!("Could not read {CATALOG_NAME}: {error}"))?;
    let Some(bytes) = &source else {
        return Ok((ConfigurationCatalog::default(), None));
    };
    let data: Contents = serde_json::from_slice(bytes)
        .map_err(|error| format!("Could not read {CATALOG_NAME}: {error}"))?;
    let mut catalog = data.catalog(selected);
    validate(&catalog).map_err(|error| format!("Invalid {CATALOG_NAME}: {error}"))?;
    catalog.introduce_keyboard_presets();
    Ok((catalog, source))
}

pub fn empty_catalog() -> ConfigurationCatalog {
    ConfigurationCatalog {
        configurations: Vec::new(),
        mappings: Vec::new(),
        selected: String::new(),
        next_mapping_id: 1,
    }
}

fn validate(catalog: &ConfigurationCatalog) -> Result<(), String> {
    catalog.validate_all()?;
    for configuration in &catalog.configurations {
        if !configuration.launch.rom.is_empty() {
            rom_path(Path::new(""), &configuration.launch.rom)?;
        }
    }
    Ok(())
}

pub fn rom_path(root: &Path, reference: &str) -> Result<PathBuf, String> {
    let path = Path::new(reference);
    if reference.is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || reference.contains(['\\', ':'])
    {
        return Err(
            "Choose a Kickstart path relative to the library directory, using / between folders"
                .into(),
        );
    }
    Ok(root.join(path))
}

#[derive(Debug)]
pub struct CatalogWrite {
    path: PathBuf,
    expected: Option<Vec<u8>>,
    bytes: Vec<u8>,
    pending: tempfile::NamedTempFile,
}

impl CatalogWrite {
    pub(crate) fn prepare(
        root: &Path,
        expected: Option<&[u8]>,
        catalog: &ConfigurationCatalog,
    ) -> Result<Self, String> {
        validate(catalog)?;
        let root = root.canonicalize().map_err(|error| error.to_string())?;
        let path = root.join(CATALOG_NAME);
        check_source(&path, expected)?;
        if path
            .metadata()
            .is_ok_and(|metadata| metadata.permissions().readonly())
        {
            return Err(format!("{CATALOG_NAME} is read-only"));
        }
        let contents = Contents {
            configurations: catalog.configurations.clone(),
            mappings: catalog.mappings.clone(),
            next_mapping_id: catalog.next_mapping_id,
        };
        let bytes = serde_json::to_vec_pretty(&contents).map_err(|error| error.to_string())?;
        let pending = stage(&root, &bytes).map_err(|error| error.to_string())?;
        Ok(Self {
            path,
            expected: expected.map(<[u8]>::to_vec),
            bytes,
            pending,
        })
    }

    pub(crate) fn publish(self) -> Result<Vec<u8>, String> {
        check_source(&self.path, self.expected.as_deref())?;
        let result = if self.expected.is_some() {
            self.pending.persist(&self.path)
        } else {
            self.pending.persist_noclobber(&self.path)
        };
        result.map_err(|error| error.error.to_string())?;
        Ok(self.bytes)
    }
}

fn check_source(path: &Path, expected: Option<&[u8]>) -> Result<(), String> {
    if read_optional(path)
        .map_err(|error| error.to_string())?
        .as_deref()
        != expected
    {
        return Err(format!(
            "{CATALOG_NAME} changed on disk. Reopen Configurations after the library updates."
        ));
    }
    Ok(())
}

fn read_optional(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn stage(directory: &Path, bytes: &[u8]) -> std::io::Result<tempfile::NamedTempFile> {
    let mut pending = tempfile::Builder::new()
        .prefix(".configuration-")
        .suffix(".pending")
        .tempfile_in(directory)?;
    pending.write_all(bytes)?;
    pending.as_file().sync_all()?;
    Ok(pending)
}

impl LegacyCatalog {
    pub(crate) fn migrate(self) -> Result<(), String> {
        let root = self
            .library_root
            .canonicalize()
            .map_err(|error| format!("Could not open the previous library: {error}"))?;
        let mut catalog = ConfigurationCatalog {
            configurations: self.configurations,
            mappings: self.mappings,
            selected: self.selected,
            next_mapping_id: self.next_mapping_id,
        };
        for configuration in &mut catalog.configurations {
            let reference = &mut configuration.launch.rom;
            if reference.is_empty() {
                continue;
            }
            let source = Path::new(reference);
            let path = if source.is_absolute() {
                source.to_path_buf()
            } else {
                root.join(source)
            };
            let path = path.canonicalize().map_err(|error| {
                format!(
                    "Kickstart for {} is unavailable: {error}",
                    configuration.name
                )
            })?;
            let relative = path.strip_prefix(&root).map_err(|_| {
                format!(
                    "Kickstart for {} is outside the previous library",
                    configuration.name
                )
            })?;
            let size = std::fs::metadata(&path).map_err(|error| error.to_string())?;
            if !size.is_file() || size.len() != physical_memory::KICKSTART_ROM_SIZE as u64 {
                return Err(format!(
                    "Kickstart for {} must be a 256 KiB file",
                    configuration.name
                ));
            }
            *reference = relative
                .to_str()
                .ok_or("The Kickstart filename cannot be stored as text")?
                .replace('\\', "/");
        }
        validate(&catalog)?;
        catalog.introduce_keyboard_presets();
        let installed =
            read_optional(&root.join(CATALOG_NAME)).map_err(|error| error.to_string())?;
        if let Some(bytes) = installed {
            // The library write can complete before native preferences are flushed.
            if serde_json::from_slice::<Contents>(&bytes).is_ok_and(|contents| {
                let mut current = contents.catalog(&catalog.selected);
                if validate(&current).is_err() {
                    return false;
                }
                current.introduce_keyboard_presets();
                same_profiles(&current, &catalog)
            }) {
                return Ok(());
            }
            return Err(format!(
                "{CATALOG_NAME} already exists in the previous library"
            ));
        }
        CatalogWrite::prepare(&root, None, &catalog)?.publish()?;
        Ok(())
    }
}

impl crate::EmulatorApp {
    pub(crate) fn restore_native_player_settings(&mut self, storage: &dyn eframe::Storage) {
        let settings = eframe::get_value::<NativePlayerSettings>(storage, SETTINGS_KEY);
        let should_migrate = |legacy: &LegacyCatalog| {
            settings.as_ref().is_none_or(|settings| {
                settings.root.as_ref().is_some_and(|root| {
                    *root == legacy.library_root
                        || root
                            .canonicalize()
                            .ok()
                            .zip(legacy.library_root.canonicalize().ok())
                            .is_some_and(|(root, old)| root == old)
                })
            })
        };
        let key = crate::player_config::CONFIG_STORAGE_KEY;
        let mut migration_error = None;
        let mut previous = None;
        if settings
            .as_ref()
            .is_none_or(|settings| !settings.migrated_legacy)
            && storage
                .get_string(key)
                .is_some_and(|value| !value.is_empty())
        {
            match eframe::get_value::<LegacyCatalog>(storage, key) {
                Some(legacy) if should_migrate(&legacy) => {
                    previous = Some(NativePlayerSettings {
                        root: (!legacy.library_root.as_os_str().is_empty())
                            .then(|| legacy.library_root.clone()),
                        selected: legacy.selected.clone(),
                        migrated_legacy: false,
                    });
                    match legacy.migrate() {
                        Ok(()) => {
                            self.player.migrated_legacy = true;
                            self.player.settings_dirty = true;
                        }
                        Err(error) => migration_error = Some(error),
                    }
                }
                None if settings.is_none() => {
                    migration_error = Some("The previous catalog could not be decoded".into());
                }
                _ => {}
            }
        }
        let settings = previous.or(settings).unwrap_or_default();
        self.player.migrated_legacy |= settings.migrated_legacy;
        self.player.library_root = settings.root;
        self.player.catalog.selected = settings.selected;
        self.player.migration_error = migration_error.map(|error| {
            format!("Previous configurations could not be moved: {error}. The original settings are still available.")
        });
    }

    pub(crate) fn reload_library_catalog(&mut self) {
        let loaded = self.player.library_root.as_ref().map_or_else(
            || Ok((ConfigurationCatalog::default(), None)),
            |root| open(root, &self.player.catalog.selected),
        );
        self.install_library_catalog(loaded);
    }

    pub(crate) fn reconcile_library_catalog(&mut self, loaded: CatalogRead, ctx: &egui::Context) {
        if self.player.migration_error.is_some() {
            return;
        }
        match loaded {
            Ok((catalog, source)) => {
                if self.player.library_error.is_some() || self.player.catalog_source != source {
                    self.install_library_catalog(Ok((catalog, source)));
                    self.refresh_clean_configuration_draft(ctx);
                }
            }
            Err(error) => self.player.library_error = Some(error),
        }
    }

    fn install_library_catalog(&mut self, loaded: CatalogRead) {
        let loaded = self
            .player
            .migration_error
            .as_ref()
            .map_or(loaded, |error| Err(error.clone()));
        match loaded {
            Ok((mut catalog, source)) => {
                let selected = &self.player.catalog.selected;
                if catalog
                    .configurations
                    .iter()
                    .any(|profile| &profile.id == selected)
                {
                    catalog.selected.clone_from(selected);
                }
                self.player.catalog = catalog;
                self.player.catalog_source = source;
                self.player.library_error = None;
            }
            Err(error) => {
                self.player.catalog = empty_catalog();
                self.player.catalog_source = None;
                self.player.library_error = Some(error);
            }
        }
    }

    pub(crate) fn commit_library_root(
        &mut self,
        root: PathBuf,
        catalog: ConfigurationCatalog,
        source: Option<Vec<u8>>,
    ) {
        self.player.library_sync = None;
        self.player.clear_library_listing();
        self.player.library_root = Some(root);
        self.player.catalog = catalog;
        self.player.catalog_source = source;
        self.player.library_error = None;
        self.player.migration_error = None;
        self.player.rom = None;
        self.player.rom_name = None;
        self.player.rom_configuration_id = None;
        self.player.rom_reference = None;
        self.player.settings_dirty = true;
    }
}
