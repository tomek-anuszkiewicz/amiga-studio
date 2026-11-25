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
        let rom_selected = self
            .player
            .selected_rom(configuration)
            .is_some_and(|bytes| bytes.len() == physical_memory::KICKSTART_ROM_SIZE);
        #[cfg(not(target_arch = "wasm32"))]
        let rom_selected = rom_selected
            || self
                .player
                .native_rom_path(configuration)
                .is_ok_and(|path| {
                    path.metadata().is_ok_and(|metadata| {
                        metadata.is_file()
                            && metadata.len() == physical_memory::KICKSTART_ROM_SIZE as u64
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
        let rom = match self.player.selected_rom(&configuration) {
            Some(bytes) => bytes.to_vec(),
            None => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    match self
                        .player
                        .native_rom_path(&configuration)
                        .and_then(|path| std::fs::read(path).map_err(|error| error.to_string()))
                    {
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
        let selected_disks = mounted.each_ref().map(|id| {
            id.as_ref()
                .and_then(|id| disks.iter().find(|disk| &disk.id == id))
        });
        let media = selected_disks.map(|selected| selected.and_then(|disk| disk.bytes.clone()));
        let disk_names = selected_disks.map(|disk| disk.map(|disk| disk.label.clone()));
        let rom_name = self
            .player
            .selected_file_rom()
            .and_then(|image| image.name)
            .or_else(|| Some(configuration.launch.rom.clone()));
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
                rom_name,
                disk_names,
            },
            HostEffect::Player(PlayerEffect::Launch {
                configuration,
                mappings: self.player.catalog.mappings.clone(),
                disks,
                association,
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

    pub(crate) fn select_setup_rom(&mut self, name: String, bytes: Vec<u8>) {
        if bytes.len() != physical_memory::KICKSTART_ROM_SIZE {
            self.file_error = Some(format!(
                "Unsupported Kickstart ({} bytes). Select a 256 KiB image.",
                bytes.len()
            ));
            return;
        }
        let Some(configuration) = self
            .player
            .catalog
            .configurations
            .iter_mut()
            .find(|c| c.id == self.player.catalog.selected)
        else {
            self.file_error = Some("Select a configuration".into());
            return;
        };
        #[cfg(target_arch = "wasm32")]
        {
            configuration.launch.rom = name;
            self.player.settings_dirty = true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.player.rom_name = Some(name);
        }
        self.player.rom_reference = Some(configuration.launch.rom.clone());
        self.player.rom = Some(bytes);
        self.player.rom_configuration_id = Some(self.player.catalog.selected.clone());
    }
}
