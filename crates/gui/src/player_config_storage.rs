//! The current flat RON user-settings contract, independent of runtime grouping.

use super::{
    ControlConfiguration, FrontendBindings, GamepadShortcut, HostInputSettings,
    LaunchConfiguration, NamedConfiguration, PortBinding,
};
use config::{A500Preset, VideoStandard};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StoredConfiguration {
    id: String,
    name: String,
    preset: A500Preset,
    video: VideoStandard,
    rom: String,
    drives: [bool; config::FLOPPY_DRIVE_COUNT],
    ports: [PortBinding; 2],
    mouse_sensitivity: f32,
    keyboard: Vec<(u8, u8)>,
    frontend: FrontendBindings,
    gamepad_shortcuts: Vec<GamepadShortcut>,
}

impl From<NamedConfiguration> for StoredConfiguration {
    fn from(configuration: NamedConfiguration) -> Self {
        Self {
            id: configuration.id,
            name: configuration.name,
            preset: configuration.launch.preset,
            video: configuration.launch.video,
            rom: configuration.launch.rom,
            drives: configuration.launch.drives,
            ports: configuration.controls.ports,
            mouse_sensitivity: configuration.controls.host.mouse_sensitivity,
            keyboard: configuration.controls.host.keyboard,
            frontend: configuration.controls.host.frontend,
            gamepad_shortcuts: configuration.controls.host.gamepad_shortcuts,
        }
    }
}

impl From<StoredConfiguration> for NamedConfiguration {
    fn from(stored: StoredConfiguration) -> Self {
        Self {
            id: stored.id,
            name: stored.name,
            launch: LaunchConfiguration {
                preset: stored.preset,
                video: stored.video,
                rom: stored.rom,
                drives: stored.drives,
            },
            controls: ControlConfiguration {
                ports: stored.ports,
                host: HostInputSettings {
                    mouse_sensitivity: stored.mouse_sensitivity,
                    keyboard: stored.keyboard,
                    frontend: stored.frontend,
                    gamepad_shortcuts: stored.gamepad_shortcuts,
                },
            },
        }
    }
}
