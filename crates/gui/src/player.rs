//! Player navigation and pending operations; one host owner retains the machine.

use crate::app_host::HostEffect;
use crate::host_commands::{HostCommand, HostValue};
use crate::player_config::{ConfigurationCatalog, NamedConfiguration};
use crate::EmulatorApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerScreen {
    Library,
    Setup,
    Configuration,
    #[default]
    Session,
}

#[derive(Debug, Clone)]
pub struct SessionDisk {
    pub id: String,
    pub label: String,
    pub bytes: Option<Vec<u8>>,
}

#[derive(Debug, Default)]
pub struct PlayerState {
    pub screen: PlayerScreen,
    pub catalog: ConfigurationCatalog,
    pub(crate) settings_dirty: bool,
    pub rom: Option<Vec<u8>>,
    pub rom_configuration_id: Option<String>,
    pub rom_reference: Option<String>,
    pub disks: Vec<SessionDisk>,
    pub setup_disks: Vec<SessionDisk>,
    pub setup_mounted: [Option<String>; 4],
    pub mounted: [Option<String>; 4],
    pub active_configuration: Option<NamedConfiguration>,
    pub captured: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) pending_pointer: crate::host_input::InputQueue,
    pub show_saves: bool,
    pub(crate) pending_launch: Option<PendingLaunch>,
    pub initial_error: bool,
    pub busy: bool,
    pub gamepads: Vec<crate::input_mapping::GamepadInfo>,
    #[cfg(not(target_arch = "wasm32"))]
    pub library: crate::game_library::GameLibrary,
    #[cfg(not(target_arch = "wasm32"))]
    pub library_editor: Option<crate::library_edit::GameDraft>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) game_slots: [bool; debugger::session::QUICK_SLOT_COUNT],
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) dialog: Option<crate::file_actions::NativeDialog>,
    #[cfg(not(target_arch = "wasm32"))]
    pub selected_game: Option<usize>,
    #[cfg(not(target_arch = "wasm32"))]
    pub active_game: Option<crate::game_library::LibraryGame>,
    #[cfg(not(target_arch = "wasm32"))]
    pub saves: Vec<crate::game_library::PlayerSave>,
}

#[derive(Debug)]
pub(crate) enum PlayerEffect {
    Launch {
        configuration: NamedConfiguration,
        disks: Vec<SessionDisk>,
        association: debugger::SaveAssociation,
        #[cfg(not(target_arch = "wasm32"))]
        game: Option<crate::game_library::LibraryGame>,
    },
    Mount {
        drive: usize,
        id: Option<String>,
    },
    #[cfg(not(target_arch = "wasm32"))]
    Save {
        game: crate::game_library::LibraryGame,
        slot: Option<usize>,
    },
}

#[derive(Debug)]
pub(crate) enum PendingLaunch {
    Manual,
    Rom {
        bytes: Vec<u8>,
        completion: crate::file_actions::FileCompletion,
    },
    #[cfg(not(target_arch = "wasm32"))]
    Library {
        game: crate::game_library::LibraryGame,
        save: Option<std::path::PathBuf>,
        normal_boot: bool,
    },
}

impl PlayerState {
    pub(crate) fn selected_rom(&self, configuration: &NamedConfiguration) -> Option<&[u8]> {
        self.rom.as_deref().filter(|_| {
            self.rom_configuration_id.as_deref() == Some(&configuration.id)
                && self.rom_reference.as_deref() == Some(&configuration.rom)
        })
    }
}

impl EmulatorApp {
    pub(crate) fn apply_frontend_action(&mut self, action: crate::player_config::FrontendAction) {
        use crate::player_config::FrontendAction;
        self.player.captured = false;
        match action {
            FrontendAction::Release => {}
            FrontendAction::Pause => self.toggle_run(),
            FrontendAction::Save => self.save_player(None),
            FrontendAction::Load => self.load_player(),
            FrontendAction::Debug => self.set_view_mode(crate::ViewMode::Developer),
        }
    }

    pub fn navigate_player(&mut self, screen: PlayerScreen) {
        self.player.captured = false;
        self.host.queue(HostCommand::Pause);
        self.player.screen = if screen == PlayerScreen::Session {
            match self.refresh_active_bindings() {
                Ok(()) => screen,
                Err(error) => {
                    self.file_error = Some(error);
                    PlayerScreen::Configuration
                }
            }
        } else {
            screen
        };
        #[cfg(not(target_arch = "wasm32"))]
        if screen == PlayerScreen::Session {
            self.refresh_player_saves();
        }
        self.set_view_mode(crate::ViewMode::ScreenOnly);
    }

    fn refresh_active_bindings(&mut self) -> Result<(), String> {
        let Some(active) = &self.player.active_configuration else {
            return Ok(());
        };
        let configuration = self
            .player
            .catalog
            .configurations
            .iter()
            .find(|c| c.id == active.id)
            .ok_or("The active input configuration is missing")?
            .clone();
        if configuration.ports.each_ref().map(|port| port.device) != self.host.status.ports {
            return Err("Port devices differ from the current machine; start a new session to change hardware".into());
        }
        self.player.catalog.validate(&configuration)?;
        self.host_sources_available(&configuration)?;
        if let Some(active) = &mut self.player.active_configuration {
            active.ports = configuration.ports;
            active.keyboard = configuration.keyboard;
            active.frontend = configuration.frontend;
            active.mouse_sensitivity = configuration.mouse_sensitivity;
            active.gamepad_shortcuts = configuration.gamepad_shortcuts;
        }
        Ok(())
    }

    pub(crate) fn request_player_launch(&mut self, launch: PendingLaunch) {
        let valid_rom = !matches!(&launch, PendingLaunch::Rom { bytes, .. } if bytes.len() != physical_memory::KICKSTART_ROM_SIZE);
        if valid_rom
            && (self.player.active_configuration.is_some()
                || self.loaded_rom_name.is_some()
                || self.host.status.cck != 0
                || self.host.status.running)
        {
            self.player.captured = false;
            self.host.queue(HostCommand::Pause);
            self.player.pending_launch = Some(launch);
        } else {
            self.confirm_player_launch(launch);
        }
    }

    pub(crate) fn confirm_player_launch(&mut self, launch: PendingLaunch) {
        match launch {
            PendingLaunch::Manual => self.start_manual(),
            PendingLaunch::Rom { bytes, completion } => self
                .host
                .queue_effect(HostCommand::LoadRom(bytes), HostEffect::File(completion)),
            #[cfg(not(target_arch = "wasm32"))]
            PendingLaunch::Library {
                game,
                save,
                normal_boot,
            } => self.start_library_game(game, save, normal_boot),
        }
    }

    pub(crate) fn complete_player(
        &mut self,
        effect: PlayerEffect,
        value: Result<HostValue, String>,
    ) {
        self.player.busy = false;
        self.player.captured = false;
        let value = match value {
            Ok(value) => value,
            Err(error) => {
                self.file_error = Some(error);
                self.player.initial_error = true;
                return;
            }
        };
        self.file_error = None;
        match effect {
            PlayerEffect::Launch {
                configuration,
                disks,
                mut association,
                #[cfg(not(target_arch = "wasm32"))]
                game,
            } => {
                if let HostValue::PlayerRestored(restored) = value {
                    association = restored;
                }
                self.player.active_configuration = Some(configuration);
                self.player.disks = disks;
                self.player.mounted = association.mounted;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.player.active_game = game;
                }
                self.player.screen = PlayerScreen::Session;
                self.player.initial_error = false;
                self.loaded_rom_name = Some("player session".into());
                self.set_view_mode(crate::ViewMode::ScreenOnly);
                self.inspection.ready = false;
                #[cfg(not(target_arch = "wasm32"))]
                self.refresh_player_saves();
            }
            PlayerEffect::Mount { drive, id } => self.player.mounted[drive] = id,
            #[cfg(not(target_arch = "wasm32"))]
            PlayerEffect::Save { game, slot } => {
                if let HostValue::Snapshot(json) = value {
                    match crate::game_library::persist_save(&game, &json, slot) {
                        Ok(_) => {
                            self.toast_message = Some((
                                "Progress saved".into(),
                                crate::layout::top_menu_bar::TOAST_DURATION_FRAMES,
                            ));
                            self.refresh_player_saves();
                        }
                        Err(error) => {
                            self.file_error = Some(format!("Could not persist progress: {error}"))
                        }
                    }
                }
            }
        }
    }

    fn host_sources_available(&self, configuration: &NamedConfiguration) -> Result<(), String> {
        for port in &configuration.ports {
            if let crate::player_config::InputSource::Gamepad { identity, .. } = &port.source {
                if self
                    .player
                    .gamepads
                    .iter()
                    .filter(|pad| &pad.identity == identity)
                    .count()
                    != 1
                {
                    return Err("A selected controller is unavailable or ambiguous. Reassign its current connection in Configurations.".into());
                }
            }
        }
        Ok(())
    }

    pub(crate) fn manual_ready(&self) -> bool {
        let Some(configuration) = self.player.catalog.selected() else {
            return false;
        };
        if self.player.catalog.validate(configuration).is_err()
            || self.host_sources_available(configuration).is_err()
        {
            return false;
        }
        let rom_selected = self
            .player
            .selected_rom(configuration)
            .is_some_and(|bytes| bytes.len() == physical_memory::KICKSTART_ROM_SIZE);
        #[cfg(not(target_arch = "wasm32"))]
        let rom_selected = rom_selected
            || std::fs::metadata(&configuration.rom)
                .is_ok_and(|metadata| metadata.len() == physical_memory::KICKSTART_ROM_SIZE as u64);
        rom_selected
            && self
                .player
                .setup_mounted
                .iter()
                .enumerate()
                .all(|(drive, id)| {
                    id.as_ref().is_none_or(|id| {
                        configuration.drives[drive]
                            && self.player.setup_disks.iter().any(|disk| {
                                &disk.id == id
                                    && disk.bytes.as_ref().is_some_and(|bytes| {
                                        debugger::launch::disk_image_is_supported(bytes)
                                    })
                            })
                    })
                })
    }

    pub fn start_manual(&mut self) {
        let Some(configuration) = self.player.catalog.selected().cloned() else {
            self.file_error = Some("Select a configuration".into());
            return;
        };
        if let Err(error) = self
            .player
            .catalog
            .validate(&configuration)
            .and_then(|_| self.host_sources_available(&configuration))
        {
            self.file_error = Some(error);
            return;
        }
        let rom = match self.player.selected_rom(&configuration) {
            Some(bytes) => bytes.to_vec(),
            None => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    match std::fs::read(&configuration.rom) {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            self.file_error =
                                Some(format!("Select a readable Kickstart ROM: {error}"));
                            return;
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    self.file_error = Some(
                        "Select Kickstart again; stored names do not grant file access".into(),
                    );
                    return;
                }
            }
        };
        let disks = self.player.setup_disks.clone();
        let mounted = self.player.setup_mounted.clone();
        let media = std::array::from_fn(|drive| {
            mounted[drive]
                .as_ref()
                .and_then(|id| disks.iter().find(|disk| &disk.id == id))
                .and_then(|disk| disk.bytes.clone())
        });
        let association = debugger::SaveAssociation {
            game_id: None,
            mounted,
        };
        let setup = debugger::launch::MachineSetup {
            config: config::A500Config::from_preset(configuration.preset, configuration.video),
            ports: configuration.ports.each_ref().map(|p| p.device),
            drives: configuration.drives,
            rom,
            disks: media,
        };
        self.player.busy = true;
        self.host.queue_effect(
            HostCommand::Launch {
                setup,
                association: association.clone(),
            },
            HostEffect::Player(PlayerEffect::Launch {
                configuration,
                disks,
                association,
                #[cfg(not(target_arch = "wasm32"))]
                game: None,
            }),
        );
    }

    pub(crate) fn select_setup_asset(
        &mut self,
        drive: Option<usize>,
        name: String,
        bytes: Vec<u8>,
    ) {
        if let Some(drive) = drive {
            if !debugger::launch::disk_image_is_supported(&bytes) {
                self.file_error = Some(format!(
                    "Unsupported ADF ({} bytes). Select a standard 880 KiB DD image.",
                    bytes.len()
                ));
                return;
            }
            let id = format!("manual-{}", self.player.setup_disks.len());
            self.player.setup_disks.push(SessionDisk {
                id: id.clone(),
                label: name,
                bytes: Some(bytes),
            });
            self.player.setup_mounted[drive] = Some(id);
        } else {
            if bytes.len() != physical_memory::KICKSTART_ROM_SIZE {
                self.file_error = Some(format!(
                    "Unsupported Kickstart ({} bytes). Select a 256 KiB image.",
                    bytes.len()
                ));
                return;
            }
            self.player.settings_dirty = true;
            self.player.rom_reference = Some(name.clone());
            if let Some(configuration) = self
                .player
                .catalog
                .configurations
                .iter_mut()
                .find(|c| c.id == self.player.catalog.selected)
            {
                configuration.rom = name;
            }
            self.player.rom = Some(bytes);
            self.player.rom_configuration_id = Some(self.player.catalog.selected.clone());
        }
    }

    pub(crate) fn mount_player_disk(&mut self, drive: usize, id: Option<String>) {
        let bytes = id
            .as_ref()
            .and_then(|id| self.player.disks.iter().find(|disk| &disk.id == id))
            .and_then(|disk| disk.bytes.clone());
        if id.is_some() && bytes.is_none() {
            self.file_error = Some("Disk is unavailable; return to setup".into());
            return;
        }
        self.player.captured = false;
        self.player.busy = true;
        self.host.queue_effect(
            HostCommand::MountDisk {
                drive,
                bytes,
                disk_id: id.clone(),
            },
            HostEffect::Player(PlayerEffect::Mount { drive, id }),
        );
    }

    pub(crate) fn save_player(&mut self, slot: Option<usize>) {
        self.player.captured = false;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(game) = self.player.active_game.clone() {
            self.player.busy = true;
            self.host.queue_effect(
                HostCommand::SaveSnapshot,
                HostEffect::Player(PlayerEffect::Save { game, slot }),
            );
            return;
        }
        if let Some(slot) = slot {
            crate::layout::top_menu_bar::quick_save_slot(self, slot);
        } else {
            self.open_save_state_dialog();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn load_player_quick(&mut self, slot: usize) {
        if let Some(game) = self.player.active_game.clone() {
            let slot_path = game.directory.join(format!("saves/slot-{slot}.txt"));
            match std::fs::read_to_string(slot_path)
                .map_err(|e| e.to_string())
                .and_then(|name| {
                    crate::game_library::relative_asset(&game.directory.join("saves"), name.trim())
                }) {
                Ok(path) => self.request_player_launch(PendingLaunch::Library {
                    game,
                    save: Some(path),
                    normal_boot: false,
                }),
                Err(error) => self.file_error = Some(format!("Quick slot unavailable: {error}")),
            }
        }
    }

    pub(crate) fn load_player(&mut self) {
        self.player.captured = false;
        #[cfg(not(target_arch = "wasm32"))]
        if self.player.active_game.is_some() {
            self.refresh_player_saves();
            self.player.show_saves = true;
            return;
        }
        self.open_load_state_dialog();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn refresh_library(&mut self) {
        match crate::game_library::GameLibrary::discover(std::path::Path::new(
            &self.player.catalog.library_root,
        )) {
            Ok(library) => {
                self.player.library = library;
                self.player.selected_game = None;
            }
            Err(error) => self.file_error = Some(format!("Could not read library: {error}")),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn refresh_player_saves(&mut self) {
        let game = self.player.active_game.as_ref().or_else(|| {
            self.player
                .selected_game
                .and_then(|index| self.player.library.games.get(index))
        });
        if let Some(game) = game {
            match crate::game_library::saves(game) {
                Ok(saves) => {
                    self.player.game_slots = std::array::from_fn(|index| {
                        self.player.active_game.as_ref().is_some_and(|active| {
                            std::fs::read_to_string(
                                active
                                    .directory
                                    .join(format!("saves/slot-{}.txt", index + 1)),
                            )
                            .ok()
                            .is_some_and(|name| {
                                saves.iter().any(|save| {
                                    save.path
                                        .file_name()
                                        .is_some_and(|file| file == name.trim())
                                })
                            })
                        })
                    });
                    self.player.saves = saves;
                }
                Err(error) => self.file_error = Some(error),
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_library_game(
        &mut self,
        game: crate::game_library::LibraryGame,
        save: Option<std::path::PathBuf>,
        normal_boot: bool,
    ) {
        let prepared = (|| {
            let configuration = self
                .player
                .catalog
                .configurations
                .iter()
                .find(|c| c.id == game.manifest.configuration)
                .cloned()
                .ok_or("Select an existing named configuration for this game")?;
            self.player.catalog.validate(&configuration)?;
            self.host_sources_available(&configuration)?;
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
            let disks =
                game.manifest
                    .disks
                    .iter()
                    .map(|disk| {
                        let source = std::fs::read(crate::game_library::relative_asset(
                            &game.directory,
                            &disk.path,
                        )?);
                        let bytes = if state_path.is_some() {
                            source.ok()
                        } else {
                            Some(source.map_err(|e| {
                                format!("Select a readable disk {}: {e}", disk.label)
                            })?)
                        };
                        Ok(SessionDisk {
                            id: disk.id.clone(),
                            label: disk.label.clone(),
                            bytes,
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
                HostCommand::RestorePlayer {
                    json,
                    game_id: Some(game.manifest.id.clone()),
                    ports: configuration.ports.each_ref().map(|p| p.device),
                }
            } else {
                let rom = std::fs::read(&configuration.rom).map_err(|e| {
                    format!("Select a readable Kickstart ROM in configuration: {e}")
                })?;
                let media = std::array::from_fn(|drive| {
                    association.mounted[drive]
                        .as_ref()
                        .and_then(|id| disks.iter().find(|disk| &disk.id == id))
                        .and_then(|disk| disk.bytes.clone())
                });
                HostCommand::Launch {
                    setup: debugger::launch::MachineSetup {
                        config: config::A500Config::from_preset(
                            configuration.preset,
                            configuration.video,
                        ),
                        ports: configuration.ports.each_ref().map(|p| p.device),
                        drives: configuration.drives,
                        rom,
                        disks: media,
                    },
                    association: association.clone(),
                }
            };
            Ok((command, configuration, disks, association))
        })();
        match prepared {
            Ok((command, configuration, disks, association)) => {
                self.player.busy = true;
                self.host.queue_effect(
                    command,
                    HostEffect::Player(PlayerEffect::Launch {
                        configuration,
                        disks,
                        association,
                        game: Some(game),
                    }),
                );
            }
            Err(error) => {
                self.file_error = Some(error);
                self.player.initial_error = true;
            }
        }
    }
}
