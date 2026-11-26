//! Player navigation and pending operations; one host owner retains the machine.

use crate::app_host::HostEffect;
use crate::host_commands::{HostCommand, HostValue};
use crate::player_config::{ConfigurationCatalog, JoystickMapping, NamedConfiguration};
use crate::{EmulatorApp, ViewMode};

#[cfg(not(target_arch = "wasm32"))]
mod library_launch;
mod manual_setup;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod save_catalog;
#[cfg(not(target_arch = "wasm32"))]
mod start_states;

fn set_session_title(ctx: &egui::Context, game_title: Option<&str>) {
    let title = game_title.unwrap_or(crate::app::APPLICATION_TITLE);
    #[cfg(not(target_arch = "wasm32"))]
    ctx.send_viewport_cmd_to(
        egui::ViewportId::ROOT,
        egui::ViewportCommand::Title(title.to_owned()),
    );
    #[cfg(target_arch = "wasm32")]
    {
        let _ = ctx;
        // eframe's browser backend handles some viewport commands but leaves document.title alone.
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            document.set_title(title);
        }
    }
}

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
    #[cfg(not(target_arch = "wasm32"))]
    pub restore_kickstart: Option<debugger::SourceImage>,
    pub(crate) kickstarts: Vec<crate::kickstarts::Entry>,
    pub(crate) manual_kickstart: Option<String>,
    pub(crate) manage_kickstarts: bool,
    pub(crate) delete_kickstart: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) legacy_kickstarts: Vec<crate::kickstarts::Entry>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) kickstart_error: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) kickstart_adoption_error: Option<String>,
    pub disks: Vec<SessionDisk>,
    pub setup_disks: Vec<SessionDisk>,
    pub(crate) drive_disk_choice: Option<crate::file_actions::DriveDiskChoice>,
    pub setup_mounted: [Option<String>; 4],
    pub mounted: [Option<String>; 4],
    pub active_configuration: Option<NamedConfiguration>,
    pub active_mappings: Vec<JoystickMapping>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) active_library_root: Option<std::path::PathBuf>,
    pub captured: bool,
    pub(crate) show_instructions: bool,
    pub(crate) pending_launch: Option<PendingLaunch>,
    pub initial_error: bool,
    pub busy: bool,
    pub gamepads: Vec<crate::input_mapping::GamepadInfo>,
    #[cfg(not(target_arch = "wasm32"))]
    pub library: crate::game_library::GameLibrary,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) library_sync: Option<crate::library_refresh::NativeLibrary>,
    #[cfg(not(target_arch = "wasm32"))]
    pub library_root: Option<std::path::PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) library_error: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) migration_error: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) migrated_legacy: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) catalog_source: Option<Vec<u8>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) missing_configuration: Option<MissingConfiguration>,
    #[cfg(not(target_arch = "wasm32"))]
    pub library_editor: Option<crate::library_edit::GameDraft>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) save_catalog: save_catalog::SaveCatalogActions,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) dialog: Option<crate::file_actions::NativeDialog>,
    #[cfg(not(target_arch = "wasm32"))]
    pub selected_game: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    library_filter: String,
    #[cfg(not(target_arch = "wasm32"))]
    normalized_library_filter: String,
    #[cfg(not(target_arch = "wasm32"))]
    pub selected_saves: Vec<crate::game_library::PlayerSave>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) selected_save_error: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) unavailable_start_states: std::collections::HashMap<String, String>,
    #[cfg(not(target_arch = "wasm32"))]
    start_state_name: Option<start_states::NameEdit>,
    #[cfg(not(target_arch = "wasm32"))]
    start_state_deletions: Vec<start_states::PendingDeletion>,
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
        mappings: Vec<JoystickMapping>,
        disks: Vec<SessionDisk>,
        association: debugger::SaveAssociation,
        initial_state: bool,
        #[cfg(not(target_arch = "wasm32"))]
        game: Option<crate::game_library::LibraryGame>,
        #[cfg(not(target_arch = "wasm32"))]
        library_root: Option<std::path::PathBuf>,
    },
    Mount {
        drive: usize,
        id: Option<String>,
    },
    Reset,
    #[cfg(not(target_arch = "wasm32"))]
    QuickSave {
        game: crate::game_library::LibraryGame,
    },
}

#[derive(Debug)]
pub(crate) enum PendingLaunch {
    Manual,
    #[cfg(not(target_arch = "wasm32"))]
    Library {
        game: crate::game_library::LibraryGame,
        save: Option<std::path::PathBuf>,
        normal_boot: bool,
    },
    #[cfg(not(target_arch = "wasm32"))]
    LibraryStartState {
        game: crate::game_library::LibraryGame,
        id: String,
    },
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub(crate) struct MissingConfiguration {
    pub game: crate::game_library::LibraryGame,
    pub save: Option<std::path::PathBuf>,
    pub normal_boot: bool,
    pub start_state: Option<String>,
    pub replacement: String,
    pub error: Option<String>,
}

impl PlayerState {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn render_library_filter(&mut self, ui: &mut egui::Ui) {
        let label = ui.label("Filter games");
        let response = crate::keyboard_input::text_edit(
            ui,
            egui::TextEdit::singleline(&mut self.library_filter)
                .id_salt("library-filter")
                .desired_width(200.0)
                .hint_text("Game name"),
        )
        .labelled_by(label.id)
        .on_hover_text("Keep game names containing this text, ignoring letter case.");
        if response.changed() {
            self.normalized_library_filter = self.library_filter.to_lowercase();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn normalized_library_filter(&self) -> &str {
        &self.normalized_library_filter
    }

    pub(crate) fn manual_kickstart<'a>(&'a self, configuration: &'a NamedConfiguration) -> &'a str {
        self.manual_kickstart
            .as_deref()
            .unwrap_or(&configuration.launch.rom)
    }

    pub(crate) fn restore_media<'a>(
        &self,
        disks: impl IntoIterator<Item = &'a SessionDisk>,
        remap_manual_ids: bool,
    ) -> crate::host_commands::RestoreMedia {
        #[cfg(not(target_arch = "wasm32"))]
        let roms = self.restore_kickstart.iter().cloned().collect();
        #[cfg(target_arch = "wasm32")]
        let roms = self
            .kickstarts
            .iter()
            .map(crate::kickstarts::Entry::source_image)
            .collect();
        crate::host_commands::RestoreMedia {
            roms,
            disks: disks
                .into_iter()
                .filter_map(|disk| {
                    Some((
                        disk.id.clone(),
                        debugger::SourceImage {
                            name: Some(disk.label.clone()),
                            bytes: disk.bytes.clone()?,
                        },
                    ))
                })
                .collect(),
            remap_manual_ids,
        }
    }

    pub(crate) fn current_mappings(&self) -> &[JoystickMapping] {
        if self.active_configuration.is_some() {
            &self.active_mappings
        } else {
            &self.catalog.mappings
        }
    }

    pub(crate) fn is_active_library(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.active_library_root == self.library_root
        }
        #[cfg(target_arch = "wasm32")]
        {
            true
        }
    }

    /// Records the profile, mappings and library used by a new session.
    pub fn activate_selected_configuration(&mut self) {
        self.active_configuration = self.catalog.selected().cloned();
        self.active_mappings = self.catalog.mappings.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.active_library_root = self.library_root.clone();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn library_available(&self) -> bool {
        self.library_error.is_none()
            && self.library_root.as_ref().is_some_and(|root| root.is_dir())
            && self
                .library_sync
                .as_ref()
                .is_none_or(|sync| sync.read_error.is_none())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn native_rom_path(&self, reference: &str) -> Result<std::path::PathBuf, String> {
        let root = self
            .library_root
            .as_deref()
            .ok_or("Choose a library in Configurations and add a Kickstart")?;
        crate::library_catalog::rom_path(root, reference)
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
            #[cfg(not(target_arch = "wasm32"))]
            self.adopt_shared_kickstart();
            self.begin_configuration_edit();
        }
        #[cfg(not(target_arch = "wasm32"))]
        if matches!(screen, PlayerScreen::Setup | PlayerScreen::Configuration) {
            self.refresh_kickstarts();
        }
        if screen != PlayerScreen::Configuration {
            self.player.editor = None;
            self.player.manage_kickstarts = false;
            self.player.delete_kickstart = None;
        }
        self.player.screen = screen;
        #[cfg(not(target_arch = "wasm32"))]
        if screen == PlayerScreen::Session {
            self.refresh_player_saves();
        }
        self.set_view_mode(mode);
    }

    pub(crate) fn request_player_launch(&mut self, launch: PendingLaunch) {
        #[cfg(not(target_arch = "wasm32"))]
        if let PendingLaunch::Library {
            game,
            save,
            normal_boot,
        } = &launch
        {
            if let Some(path) = save {
                self.load_catalog_save(game.clone(), path.clone(), false);
                return;
            }
            if self.prompt_for_missing_configuration(game, None, *normal_boot) {
                return;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let pause_for_confirmation = match &launch {
            PendingLaunch::Library {
                game,
                save,
                normal_boot,
            } => save.is_none() && (*normal_boot || game.manifest.initial_state.is_none()),
            PendingLaunch::LibraryStartState { .. } => false,
            _ => true,
        };
        #[cfg(target_arch = "wasm32")]
        let pause_for_confirmation = true;
        if self.player.active_configuration.is_some()
            || self.file_loaded_machine
            || self.host.status.cck != 0
            || self.host.status.running
        {
            if pause_for_confirmation {
                self.player.captured = false;
                self.host.queue(HostCommand::Pause);
            }
            self.player.pending_launch = Some(launch);
        } else {
            self.confirm_player_launch(launch);
        }
    }

    pub(crate) fn confirm_player_launch(&mut self, launch: PendingLaunch) {
        match launch {
            PendingLaunch::Manual => self.start_manual(),
            #[cfg(not(target_arch = "wasm32"))]
            PendingLaunch::Library {
                game,
                save,
                normal_boot,
            } => self.start_library_game(game, save, normal_boot),
            #[cfg(not(target_arch = "wasm32"))]
            PendingLaunch::LibraryStartState { game, id } => self.start_named_state(game, id),
        }
    }

    pub(crate) fn complete_player(
        &mut self,
        effect: PlayerEffect,
        value: Result<HostValue, String>,
        ctx: &egui::Context,
    ) {
        self.player.busy = false;
        let value = match value {
            Ok(value) => value,
            Err(error) => {
                self.file_error = Some(match effect {
                    #[cfg(not(target_arch = "wasm32"))]
                    PlayerEffect::QuickSave { .. } => {
                        format!("Could not capture quicksave: {error}")
                    }
                    PlayerEffect::Launch { initial_state, .. } => {
                        self.player.initial_error = initial_state;
                        error
                    }
                    _ => error,
                });
                return;
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        let keep_capture = matches!(effect, PlayerEffect::QuickSave { .. });
        #[cfg(target_arch = "wasm32")]
        let keep_capture = false;
        self.player.captured &= keep_capture;
        self.file_error = None;
        match effect {
            PlayerEffect::Launch {
                configuration,
                mappings,
                disks,
                mut association,
                initial_state: _,
                #[cfg(not(target_arch = "wasm32"))]
                game,
                #[cfg(not(target_arch = "wasm32"))]
                library_root,
            } => {
                if let HostValue::PlayerRestored(restored) = value {
                    association = restored;
                }
                self.player.active_configuration = Some(configuration);
                self.player.active_mappings = mappings;
                self.player.disks = disks;
                self.player.mounted = association.mounted;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.player.active_library_root = library_root;
                    self.associate_library_game(game);
                }
                self.player.screen = PlayerScreen::Session;
                self.player.show_instructions = false;
                self.player.initial_error = false;
                self.sync_session_title(ctx);
                self.set_view_mode(crate::ViewMode::ScreenOnly);
                self.inspection.ready = false;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let migration = self
                        .player
                        .active_game
                        .as_ref()
                        .and_then(|game| crate::game_library::initialize_quicksave(game).err());
                    self.refresh_player_saves();
                    if let Some(error) = migration {
                        self.player.session_save_error = Some(error);
                    }
                }
            }
            PlayerEffect::Mount { drive, id } => self.player.mounted[drive] = id,
            PlayerEffect::Reset => {
                if let HostValue::PlayerReset(mounted) = value {
                    self.player.mounted = mounted;
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            PlayerEffect::QuickSave { game } => {
                if let HostValue::Snapshot(json) = value {
                    match crate::game_library::persist_quicksave(&game, &json) {
                        Ok(_) => {
                            self.toast_message = Some((
                                "Progress saved".into(),
                                crate::layout::top_menu_bar::TOAST_DURATION_FRAMES,
                            ))
                        }
                        Err(error) => self.file_error = Some(error),
                    }
                    self.refresh_game_saves(&game.directory, &game.manifest.id);
                }
            }
        }
    }

    pub(crate) fn sync_session_title(&self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        let game_title = self
            .player
            .active_game
            .as_ref()
            .map(|game| game.manifest.title.as_str());
        #[cfg(target_arch = "wasm32")]
        let game_title = None;
        set_session_title(ctx, game_title);
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

    pub(crate) fn mount_player_disk(&mut self, drive: usize, id: Option<String>) {
        let (bytes, name) = if let Some(id) = &id {
            let Some(disk) = self.player.disks.iter().find(|disk| &disk.id == id) else {
                self.file_error = Some("Disk is unavailable; return to setup".into());
                return;
            };
            (disk.bytes.clone(), Some(disk.label.clone()))
        } else {
            (None, None)
        };
        self.player.captured = false;
        self.player.busy = true;
        self.host.queue_effect(
            HostCommand::MountDisk {
                drive,
                bytes,
                disk_id: id.clone(),
                name,
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
                    id.as_ref().map(|id| {
                        let disk = self.player.disks.iter().find(|disk| &disk.id == id);
                        debugger::launch::StartupDisk {
                            id: id.clone(),
                            bytes: disk.and_then(|disk| disk.bytes.clone()),
                            name: disk.map(|disk| disk.label.clone()),
                        }
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

    pub(crate) fn quick_save(&mut self) {
        if self.player.busy {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(game) = self.player.active_game.clone() {
            self.player.busy = true;
            self.host.queue_effect(
                HostCommand::SaveSnapshot,
                HostEffect::Player(PlayerEffect::QuickSave { game }),
            );
            return;
        }
        self.host.queue_effect(
            HostCommand::QuickSave,
            HostEffect::File(crate::file_actions::FileCompletion::QuickSave),
        );
    }

    pub(crate) fn quick_load(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(game) = self.player.active_game.clone() {
            self.quick_load_library_save(game);
            return;
        }
        if !self.quick_load_available() {
            return;
        }
        if let Err(error) = self.validate_current_input() {
            self.file_error = Some(error);
            return;
        }
        self.host.queue_effect(
            HostCommand::QuickLoad {
                ports: self.current_control_ports(),
            },
            HostEffect::File(crate::file_actions::FileCompletion::State(
                "Quicksave restored".into(),
            )),
        );
    }

    pub(crate) fn quick_load_available(&self) -> bool {
        if self.player.busy {
            return false;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(game) = &self.player.active_game {
            return self.player.saves.iter().any(|save| {
                save.quicksave
                    && save.header.is_some()
                    && self
                        .player
                        .save_catalog
                        .deadline(game, &save.path)
                        .is_none()
            });
        }
        self.host.status.has_quicksave
    }

    pub(crate) fn editing_save_description(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.player.save_catalog.description.is_some()
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }
}
