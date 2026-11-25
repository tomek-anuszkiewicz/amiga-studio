//! Native library game launches and configuration reference repair.

use super::{MissingConfiguration, PendingLaunch, PlayerEffect, PlayerState, SessionDisk};
use crate::app_host::HostEffect;
use crate::host_commands::HostCommand;
use crate::EmulatorApp;

impl PlayerState {
    fn library_contains(&self, game: &crate::game_library::LibraryGame) -> bool {
        self.library_root
            .as_deref()
            .zip(game.directory.parent())
            .is_some_and(|(root, parent)| {
                root == parent
                    || root
                        .canonicalize()
                        .ok()
                        .zip(parent.canonicalize().ok())
                        .is_some_and(|(root, parent)| root == parent)
            })
    }
}

impl EmulatorApp {
    pub fn start_library_game(
        &mut self,
        game: crate::game_library::LibraryGame,
        save: Option<std::path::PathBuf>,
        normal_boot: bool,
    ) {
        self.open_library_game(game, save, normal_boot, false);
    }

    pub(super) fn open_library_game(
        &mut self,
        game: crate::game_library::LibraryGame,
        save: Option<std::path::PathBuf>,
        normal_boot: bool,
        quick_load: bool,
    ) {
        let explicit_save = save.is_some();
        let initial_state = !explicit_save && !normal_boot && game.manifest.initial_state.is_some();
        if self.prompt_for_missing_configuration(&game, save.as_deref(), normal_boot) {
            return;
        }
        let prepared = (|| {
            game.manifest.validate()?;
            let continuing = self.continuing_game(&game, save.is_some(), normal_boot);
            let (configuration, mappings, library_root) = if continuing {
                (
                    self.player
                        .active_configuration
                        .as_ref()
                        .ok_or("The active game has no committed controls")?,
                    &self.player.active_mappings,
                    &self.player.active_library_root,
                )
            } else {
                if !self.player.library_available() || !self.player.library_contains(&game) {
                    return Err(
                        "Open this game's library in Configurations before starting it".into(),
                    );
                }
                (
                    self.player
                        .catalog
                        .configurations
                        .iter()
                        .find(|configuration| configuration.id == game.manifest.configuration)
                        .ok_or("Configuration missing")?,
                    &self.player.catalog.mappings,
                    &self.player.library_root,
                )
            };
            configuration.validate(mappings)?;
            self.host_sources_available(&configuration.controls)?;
            let state_path = save.or_else(|| {
                if normal_boot {
                    None
                } else {
                    game.manifest
                        .initial_state
                        .as_ref()
                        .map(|p| game.directory.join(p))
                }
            });
            let save_document = state_path
                .map(|path| {
                    let (bytes, _) =
                        crate::game_library::read_associated_save(&path, &game.manifest)
                            .map_err(|error| format!("{}: {error}", path.display()))?;
                    String::from_utf8(bytes).map_err(|error| format!("{}: {error}", path.display()))
                })
                .transpose()?;
            let current_session_disks = self.player.disks.iter().filter(|_| continuing);
            let disks = game
                .manifest
                .disks
                .iter()
                .map(|disk| {
                    let path = disk.asset_path(&game.directory)?;
                    Ok(SessionDisk {
                        id: disk.id.clone(),
                        label: disk.filename.clone(),
                        bytes: std::fs::read(path).ok(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let association = debugger::SaveAssociation {
                game_id: Some(game.manifest.id.clone()),
                mounted: game.manifest.mounted.clone(),
            };

            let command = if let Some(json) = save_document {
                // Old session buffers can satisfy a reference while the disk
                // list shows the current availability of its source files.
                let mut media = self.player.restore_media(
                    disks
                        .iter()
                        .chain(current_session_disks)
                        .chain(&self.player.setup_disks),
                    false,
                );
                if let Some(root) = library_root.as_deref() {
                    // The active profile supplies committed controls. The current
                    // library profile can supply a relocated ROM for that session.
                    let current = self
                        .player
                        .library_root
                        .as_deref()
                        .filter(|path| *path == root)
                        .and_then(|_| {
                            self.player
                                .catalog
                                .configurations
                                .iter()
                                .find(|entry| entry.id == game.manifest.configuration)
                        });
                    if let Some(source) =
                        self.read_legacy_save_rom(root, current.unwrap_or(configuration))
                    {
                        media.roms.push(source);
                    }
                    crate::kickstarts::native::extend_restore_roms(root, &mut media.roms);
                }
                if let Some(opened) = self
                    .player
                    .library_root
                    .as_deref()
                    .filter(|opened| Some(*opened) != library_root.as_deref())
                {
                    crate::kickstarts::native::extend_restore_roms(opened, &mut media.roms);
                }
                let ports = configuration.controls.ports.each_ref().map(|p| p.device);
                if quick_load {
                    HostCommand::LoadPlayerQuick {
                        json,
                        media,
                        game_id: game.manifest.id.clone(),
                        ports,
                    }
                } else {
                    HostCommand::RestorePlayer {
                        json,
                        media,
                        game_id: Some(game.manifest.id.clone()),
                        ports,
                        resume: true,
                    }
                }
            } else {
                let mut media = [const { None }; config::FLOPPY_DRIVE_COUNT];
                for (drive, id) in association.mounted.iter().enumerate() {
                    if let Some(id) = id {
                        if !configuration.launch.drives[drive] {
                            return Err(format!(
                                "DF{drive} is disconnected in the game's configuration"
                            ));
                        }
                        let disk = disks
                            .iter()
                            .find(|disk| &disk.id == id)
                            .ok_or_else(|| format!("Unknown startup disk {id} for DF{drive}"))?;
                        let bytes = disk.bytes.as_ref().ok_or_else(|| {
                            format!("Startup disk {} for DF{drive} is unavailable", disk.label)
                        })?;
                        if !debugger::launch::disk_image_is_supported(bytes) {
                            return Err(format!(
                                "Startup disk {} for DF{drive} is an unsupported ADF ({} bytes)",
                                disk.label,
                                bytes.len()
                            ));
                        }
                        media[drive] = Some(bytes.clone());
                    }
                }
                let path = self.player.native_rom_path(&configuration.launch.rom)?;
                let rom = crate::kickstarts::native::read_image(&path)?;
                let disk_names = association.mounted.each_ref().map(|id| {
                    id.as_ref()
                        .and_then(|id| disks.iter().find(|disk| &disk.id == id))
                        .map(|disk| disk.label.clone())
                });
                HostCommand::Launch {
                    setup: debugger::launch::MachineSetup {
                        config: config::A500Config::from_preset(
                            configuration.launch.preset,
                            configuration.launch.video,
                        ),
                        ports: configuration.controls.ports.each_ref().map(|p| p.device),
                        drives: configuration.launch.drives,
                        rom: rom.bytes,
                        disks: media,
                    },
                    association: association.clone(),
                    rom_name: rom.name,
                    disk_names,
                }
            };
            Ok((
                command,
                configuration.clone(),
                mappings.clone(),
                library_root.clone(),
                disks,
                association,
            ))
        })();
        match prepared {
            Ok((command, configuration, mappings, library_root, disks, association)) => {
                self.player.busy = true;
                self.host.queue_effect(
                    command,
                    HostEffect::Player(PlayerEffect::Launch {
                        configuration,
                        mappings,
                        disks,
                        association,
                        initial_state,
                        game: Some(game),
                        library_root,
                    }),
                );
            }
            Err(error) => {
                self.file_error = Some(if explicit_save {
                    format!("Could not load save: {error}")
                } else {
                    error
                });
                self.player.initial_error = initial_state;
            }
        }
    }

    pub(crate) fn read_legacy_save_rom(
        &self,
        root: &std::path::Path,
        configuration: &crate::player_config::NamedConfiguration,
    ) -> Option<debugger::SourceImage> {
        if crate::kickstarts::native::managed_path(root, &configuration.launch.rom).is_ok() {
            return None;
        }
        let path = crate::library_catalog::rom_path(root, &configuration.launch.rom).ok()?;
        Some(debugger::SourceImage {
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned()),
            bytes: std::fs::read(&path).ok()?,
        })
    }

    fn continuing_game(
        &self,
        game: &crate::game_library::LibraryGame,
        has_save: bool,
        normal_boot: bool,
    ) -> bool {
        (has_save || (!normal_boot && game.manifest.initial_state.is_some()))
            && self
                .player
                .active_game
                .as_ref()
                .is_some_and(|active| active.same_entry(game))
            && self.player.active_configuration.is_some()
    }

    pub(super) fn prompt_for_missing_configuration(
        &mut self,
        game: &crate::game_library::LibraryGame,
        save: Option<&std::path::Path>,
        normal_boot: bool,
    ) -> bool {
        if !self.player.library_available()
            || !self.player.library_contains(game)
            || self.continuing_game(game, save.is_some(), normal_boot)
            || self
                .player
                .catalog
                .configurations
                .iter()
                .any(|configuration| configuration.id == game.manifest.configuration)
        {
            return false;
        }
        self.player.missing_configuration = Some(MissingConfiguration {
            game: game.clone(),
            save: save.map(std::path::Path::to_path_buf),
            normal_boot,
            start_state: None,
            replacement: String::new(),
            error: None,
        });
        true
    }

    pub(crate) fn repair_game_configuration(&mut self) {
        let Some(mut repair) = self.player.missing_configuration.take() else {
            return;
        };
        let result = if self.player.library_available()
            && self.player.library_contains(&repair.game)
            && self
                .player
                .catalog
                .configurations
                .iter()
                .any(|configuration| configuration.id == repair.replacement)
        {
            crate::game_library::replace_configuration(
                &repair.game.directory,
                &repair.game.manifest,
                &repair.replacement,
            )
        } else {
            Err("Choose an existing configuration from this game's library".into())
        };
        match result {
            Ok(manifest) => {
                repair.game.manifest.configuration = manifest.configuration;
                if let Some(entry) = self
                    .player
                    .library
                    .games
                    .iter_mut()
                    .find(|entry| entry.same_entry(&repair.game))
                {
                    entry
                        .manifest
                        .configuration
                        .clone_from(&repair.game.manifest.configuration);
                }
                if let Some(sync) = &mut self.player.library_sync {
                    sync.changed_locally();
                }
                self.file_error = None;
                if let Some(id) = repair.start_state {
                    self.start_named_state(repair.game, id);
                } else {
                    self.request_player_launch(PendingLaunch::Library {
                        game: repair.game,
                        save: repair.save,
                        normal_boot: repair.normal_boot,
                    });
                }
            }
            Err(error) => {
                repair.error = Some(format!(
                    "Could not save the configuration reference: {error}"
                ));
                self.player.missing_configuration = Some(repair);
            }
        }
    }
}
