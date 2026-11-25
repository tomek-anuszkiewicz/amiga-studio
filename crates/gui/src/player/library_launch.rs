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
        if self.prompt_for_missing_configuration(&game, save.as_deref(), normal_boot) {
            return;
        }
        let prepared = (|| {
            game.manifest.validate()?;
            let (configuration, mappings, library_root) =
                if self.continuing_game(&game, save.is_some(), normal_boot) {
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
            let disks = game
                .manifest
                .disks
                .iter()
                .map(|disk| {
                    let path = crate::game_library::relative_asset(&game.directory, &disk.path)?;
                    Ok(SessionDisk {
                        id: disk.id.clone(),
                        label: disk.display_name().into(),
                        bytes: std::fs::read(path).ok(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let association = debugger::SaveAssociation {
                game_id: Some(game.manifest.id.clone()),
                mounted: game.manifest.mounted.clone(),
            };

            let command = if let Some(path) = state_path {
                let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
                let saved = debugger::SaveState::from_json(&json).map_err(|e| e.to_string())?;
                game.manifest
                    .validate_association(&saved.header.association)?;
                let ports = configuration.controls.ports.each_ref().map(|p| p.device);
                if quick_load {
                    HostCommand::LoadPlayerQuick {
                        json,
                        game_id: game.manifest.id.clone(),
                        ports,
                    }
                } else {
                    HostCommand::RestorePlayer {
                        json,
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
                let rom =
                    std::fs::read(self.player.native_rom_path(configuration)?).map_err(|e| {
                        format!("Select a readable Kickstart ROM in configuration: {e}")
                    })?;
                HostCommand::Launch {
                    setup: debugger::launch::MachineSetup {
                        config: config::A500Config::from_preset(
                            configuration.launch.preset,
                            configuration.launch.video,
                        ),
                        ports: configuration.controls.ports.each_ref().map(|p| p.device),
                        drives: configuration.launch.drives,
                        rom,
                        disks: media,
                    },
                    association: association.clone(),
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
                        game: Some(game),
                        library_root,
                    }),
                );
            }
            Err(error) => {
                self.file_error = Some(error);
                self.player.initial_error = true;
            }
        }
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
                self.request_player_launch(PendingLaunch::Library {
                    game: repair.game,
                    save: repair.save,
                    normal_boot: repair.normal_boot,
                });
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
