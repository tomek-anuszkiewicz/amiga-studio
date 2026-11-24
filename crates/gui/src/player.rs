//! Player navigation and pending operations; one host owner retains the machine.

use crate::app_host::HostEffect;
use crate::host_commands::{HostCommand, HostValue};
use crate::player_config::{ConfigurationCatalog, NamedConfiguration};
use crate::{EmulatorApp, ViewMode};

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
    pub(crate) drive_activity: [crate::player_drive::DriveActivity; config::FLOPPY_DRIVE_COUNT],
    pub screen: PlayerScreen,
    pub catalog: ConfigurationCatalog,
    pub editor: Option<crate::configuration_edit::ConfigurationEdit>,
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
    pub show_saves: bool,
    pub(crate) pending_launch: Option<PendingLaunch>,
    pub initial_error: bool,
    pub busy: bool,
    pub gamepads: Vec<crate::input_mapping::GamepadInfo>,
    #[cfg(not(target_arch = "wasm32"))]
    pub library: crate::game_library::GameLibrary,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) library_sync: Option<crate::library_refresh::NativeLibrary>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) watched_setting: String,
    #[cfg(not(target_arch = "wasm32"))]
    pub library_editor: Option<crate::library_edit::GameDraft>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) game_slots: [bool; debugger::session::QUICK_SLOT_COUNT],
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) dialog: Option<crate::file_actions::NativeDialog>,
    #[cfg(not(target_arch = "wasm32"))]
    pub selected_game: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub library_filter: String,
    #[cfg(not(target_arch = "wasm32"))]
    pub selected_saves: Vec<crate::game_library::PlayerSave>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) selected_save_error: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub active_game: Option<crate::game_library::LibraryGame>,
    #[cfg(not(target_arch = "wasm32"))]
    pub saves: Vec<crate::game_library::PlayerSave>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) session_save_error: Option<String>,
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
    Reset,
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
    #[cfg(not(target_arch = "wasm32"))]
    LibraryQuick {
        game: crate::game_library::LibraryGame,
        save: std::path::PathBuf,
    },
}

impl PlayerState {
    pub(crate) fn selected_rom(&self, configuration: &NamedConfiguration) -> Option<&[u8]> {
        self.rom.as_deref().filter(|_| {
            self.rom_configuration_id.as_deref() == Some(&configuration.id)
                && self.rom_reference.as_deref() == Some(&configuration.launch.rom)
        })
    }
}

impl EmulatorApp {
    /// Changes presentation and connects debugger work only in Developer view.
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        if mode == ViewMode::Developer && self.player.screen != PlayerScreen::Session {
            if self.guard_configuration_navigation(PlayerScreen::Session) {
                if let Some(editor) = &mut self.player.editor {
                    editor.destination_developer = true;
                }
                return;
            }
            self.enter_player_screen(PlayerScreen::Session, mode);
            return;
        }
        if self.view_mode == mode {
            return;
        }
        if self.player.screen == PlayerScreen::Session {
            self.player.captured = false;
            self.host.release_input_for_view_change();
        }
        if mode == ViewMode::Developer {
            self.host.queue(HostCommand::Pause);
        }
        self.view_mode = mode;
        self.host
            .queue(HostCommand::SetDebuggerEnabled(mode == ViewMode::Developer));
    }

    pub(crate) fn apply_frontend_action(&mut self, action: crate::player_config::FrontendAction) {
        use crate::player_config::FrontendAction;
        if !matches!(action, FrontendAction::Save | FrontendAction::Debug) {
            self.player.captured = false;
        }
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
        if self.guard_configuration_navigation(screen) {
            return;
        }
        self.enter_player_screen(screen, ViewMode::ScreenOnly);
    }

    pub(crate) fn enter_player_screen(&mut self, screen: PlayerScreen, mode: ViewMode) {
        if screen == PlayerScreen::Session && self.player.active_configuration.is_some() {
            if let Err(error) = self.validate_current_input() {
                self.file_error = Some(error);
                if self.player.editor.is_none() {
                    self.begin_configuration_edit();
                }
                self.player.screen = PlayerScreen::Configuration;
                self.set_view_mode(crate::ViewMode::ScreenOnly);
                return;
            }
        }
        if screen == PlayerScreen::Configuration && self.player.editor.is_none() {
            self.begin_configuration_edit();
        }
        if screen != PlayerScreen::Configuration {
            self.player.editor = None;
        }
        self.player.screen = screen;
        #[cfg(not(target_arch = "wasm32"))]
        if screen == PlayerScreen::Session {
            self.refresh_player_saves();
        }
        self.set_view_mode(mode);
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
            #[cfg(not(target_arch = "wasm32"))]
            PendingLaunch::LibraryQuick { game, save } => {
                self.open_library_game(game, Some(save), false, true);
            }
        }
    }

    pub(crate) fn complete_player(
        &mut self,
        effect: PlayerEffect,
        value: Result<HostValue, String>,
    ) {
        self.player.busy = false;
        let release_capture = match &effect {
            PlayerEffect::Reset => value.is_ok(),
            #[cfg(not(target_arch = "wasm32"))]
            PlayerEffect::Save { .. } => false,
            _ => true,
        };
        self.player.captured &= !release_capture;
        let value = match value {
            Ok(value) => value,
            Err(error) => {
                self.file_error = Some(error);
                if !matches!(&effect, PlayerEffect::Reset) {
                    self.player.initial_error = true;
                }
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
                self.associate_library_game(game);
                self.player.screen = PlayerScreen::Session;
                self.player.initial_error = false;
                self.loaded_rom_name = Some("player session".into());
                self.set_view_mode(crate::ViewMode::ScreenOnly);
                self.inspection.ready = false;
                #[cfg(not(target_arch = "wasm32"))]
                self.refresh_player_saves();
            }
            PlayerEffect::Mount { drive, id } => self.player.mounted[drive] = id,
            PlayerEffect::Reset => {
                if let HostValue::PlayerReset(mounted) = value {
                    self.player.mounted = mounted;
                }
            }
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
                            if let Some(sync) = &mut self.player.library_sync {
                                if sync.owns_game(&game) {
                                    sync.changed_locally();
                                }
                            }
                        }
                        Err(error) => {
                            self.file_error = Some(format!("Could not persist progress: {error}"))
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn host_sources_available(
        &self,
        configuration: &crate::player_config::ControlConfiguration,
    ) -> Result<(), String> {
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
            || self
                .host_sources_available(&configuration.controls)
                .is_err()
        {
            return false;
        }
        let rom_selected = self
            .player
            .selected_rom(configuration)
            .is_some_and(|bytes| bytes.len() == physical_memory::KICKSTART_ROM_SIZE);
        #[cfg(not(target_arch = "wasm32"))]
        let rom_selected = rom_selected
            || std::fs::metadata(&configuration.launch.rom)
                .is_ok_and(|metadata| metadata.len() == physical_memory::KICKSTART_ROM_SIZE as u64);
        rom_selected
            && self
                .player
                .setup_mounted
                .iter()
                .enumerate()
                .all(|(drive, id)| {
                    id.as_ref().is_none_or(|id| {
                        configuration.launch.drives[drive]
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
            .and_then(|_| self.host_sources_available(&configuration.controls))
        {
            self.file_error = Some(error);
            return;
        }
        let rom = match self.player.selected_rom(&configuration) {
            Some(bytes) => bytes.to_vec(),
            None => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    match std::fs::read(&configuration.launch.rom) {
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
            config: config::A500Config::from_preset(
                configuration.launch.preset,
                configuration.launch.video,
            ),
            ports: configuration.controls.ports.each_ref().map(|p| p.device),
            drives: configuration.launch.drives,
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
                configuration.launch.rom = name;
            }
            self.player.rom = Some(bytes);
            self.player.rom_configuration_id = Some(self.player.catalog.selected.clone());
        }
    }

    pub(crate) fn mount_player_disk(&mut self, drive: usize, id: Option<String>) {
        let bytes = if let Some(id) = &id {
            let Some(disk) = self.player.disks.iter().find(|disk| &disk.id == id) else {
                self.file_error = Some("Disk is unavailable; return to setup".into());
                return;
            };
            disk.bytes.clone()
        } else {
            None
        };
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

    pub(crate) fn reset_player(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        let startup = if let Some(game) = &self.player.active_game {
            if let Err(error) = game.manifest.validate() {
                self.file_error = Some(error);
                return;
            }
            Some(crate::host_commands::GameStartup {
                game_id: game.manifest.id.clone(),
                disks: game.manifest.mounted.each_ref().map(|id| {
                    id.as_ref().map(|id| debugger::launch::StartupDisk {
                        id: id.clone(),
                        bytes: self
                            .player
                            .disks
                            .iter()
                            .find(|disk| &disk.id == id)
                            .and_then(|disk| disk.bytes.clone()),
                    })
                }),
            })
        } else {
            None
        };
        #[cfg(target_arch = "wasm32")]
        let startup = None;
        self.player.busy = true;
        self.host.queue_effect(
            HostCommand::ResetPlayer(startup),
            HostEffect::Player(PlayerEffect::Reset),
        );
    }

    pub(crate) fn save_player(&mut self, slot: Option<usize>) {
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
                Ok(path) => {
                    self.request_player_launch(PendingLaunch::LibraryQuick { game, save: path })
                }
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
    pub fn start_library_game(
        &mut self,
        game: crate::game_library::LibraryGame,
        save: Option<std::path::PathBuf>,
        normal_boot: bool,
    ) {
        self.open_library_game(game, save, normal_boot, false);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_library_game(
        &mut self,
        game: crate::game_library::LibraryGame,
        save: Option<std::path::PathBuf>,
        normal_boot: bool,
        quick_load: bool,
    ) {
        let prepared = (|| {
            game.manifest.validate()?;
            let continuing =
                save.is_some() || (!normal_boot && game.manifest.initial_state.is_some());
            let configuration_id = if continuing
                && self
                    .player
                    .active_game
                    .as_ref()
                    .is_some_and(|active| active.same_entry(&game))
            {
                self.player
                    .active_configuration
                    .as_ref()
                    .map_or(&game.manifest.configuration, |active| &active.id)
            } else {
                &game.manifest.configuration
            };
            let configuration = self
                .player
                .catalog
                .configurations
                .iter()
                .find(|c| &c.id == configuration_id)
                .cloned()
                .ok_or("Select an existing named configuration for this game")?;
            self.player.catalog.validate(&configuration)?;
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
                let rom = std::fs::read(&configuration.launch.rom).map_err(|e| {
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
