//! Manual launch preparation and selected media.

use super::{PlayerEffect, SessionDisk};
use crate::app_host::HostEffect;
use crate::host_commands::HostCommand;
use crate::EmulatorApp;

impl EmulatorApp {
    pub(crate) fn manual_ready(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if self.player.library_root.is_some() && !self.player.library_available() {
            return false;
        }
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
        let reference = self.player.manual_kickstart(configuration);
        #[cfg(target_arch = "wasm32")]
        let rom_selected = self
            .player
            .kickstarts
            .iter()
            .any(|entry| entry.reference == reference);
        #[cfg(not(target_arch = "wasm32"))]
        let rom_selected = self.player.native_rom_path(reference).is_ok_and(|path| {
            path.metadata().is_ok_and(|metadata| {
                metadata.is_file() && metadata.len() == physical_memory::KICKSTART_ROM_SIZE as u64
            })
        });
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
        #[cfg(not(target_arch = "wasm32"))]
        if self.player.library_root.is_some() && !self.player.library_available() {
            self.file_error =
                Some("Check the library directory and profiles in Configurations".into());
            return;
        }
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
        let reference = self.player.manual_kickstart(&configuration);
        #[cfg(not(target_arch = "wasm32"))]
        let source = self
            .player
            .native_rom_path(reference)
            .and_then(|path| crate::kickstarts::native::read_image(&path));
        #[cfg(target_arch = "wasm32")]
        let source = self
            .player
            .kickstarts
            .iter()
            .find(|entry| entry.reference == reference)
            .map(crate::kickstarts::Entry::source_image)
            .ok_or_else(|| {
                "Add the Kickstart in Configurations and choose it in Manual setup".to_owned()
            });
        match source {
            Ok(source) => self.launch_manual_with_rom(configuration, source),
            Err(error) => self.file_error = Some(error),
        }
    }

    pub(crate) fn launch_manual_with_rom(
        &mut self,
        configuration: crate::player_config::NamedConfiguration,
        source: debugger::SourceImage,
    ) {
        if let Err(error) = crate::kickstarts::validate_size(
            source.name.as_deref().unwrap_or("ROM"),
            source.bytes.len() as u64,
        ) {
            self.file_error = Some(error);
            return;
        }
        let disks = self.player.setup_disks.clone();
        let mounted = self.player.setup_mounted.clone();
        let selected_disks = mounted.each_ref().map(|id| {
            id.as_ref()
                .and_then(|id| disks.iter().find(|disk| &disk.id == id))
        });
        let media = selected_disks.map(|selected| selected.and_then(|disk| disk.bytes.clone()));
        let disk_names = selected_disks.map(|disk| disk.map(|disk| disk.label.clone()));
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
            rom: source.bytes,
            disks: media,
        };
        self.player.busy = true;
        self.host.queue_effect(
            HostCommand::Launch {
                setup,
                association: association.clone(),
                rom_name: source.name,
                disk_names,
            },
            HostEffect::Player(PlayerEffect::Launch {
                configuration,
                mappings: self.player.catalog.mappings.clone(),
                disks,
                association,
                initial_state: false,
                #[cfg(not(target_arch = "wasm32"))]
                game: None,
                #[cfg(not(target_arch = "wasm32"))]
                library_root: self.player.library_root.clone(),
            }),
        );
    }

    pub(crate) fn select_setup_disk(&mut self, drive: usize, image: crate::disk_import::DiskImage) {
        let Some(mounted) = self.player.setup_mounted.get_mut(drive) else {
            self.file_error = Some("Invalid drive number".into());
            return;
        };
        let id = format!("manual-{}", self.player.setup_disks.len());
        self.player.setup_disks.push(SessionDisk {
            id: id.clone(),
            label: image.filename,
            bytes: Some(image.bytes),
        });
        *mounted = Some(id);
    }
}
