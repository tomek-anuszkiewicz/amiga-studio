//! Host-owned named compositions; hardware presets remain the core authority.

use config::{A500Preset, VideoStandard};
use serde::{Deserialize, Serialize};

#[path = "player_config_storage.rs"]
mod storage;
use storage::StoredConfiguration;

pub const CONFIG_STORAGE_KEY: &str = "player_configurations";
pub(crate) const MIN_MOUSE_SENSITIVITY: f32 = 0.1;
pub(crate) const MAX_MOUSE_SENSITIVITY: f32 = 4.0;
pub(crate) const MAX_CONTROLLER_AXIS: usize = 15;
pub(crate) const MIN_CONTROLLER_THRESHOLD: f32 = 0.1;
pub(crate) const MAX_CONTROLLER_THRESHOLD: f32 = 1.0;

pub(crate) const PAD_SOUTH: usize = 0;
pub(crate) const PAD_EAST: usize = 1;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PAD_WEST: usize = 2;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PAD_NORTH: usize = 3;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PAD_LEFT_SHOULDER: usize = 4;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PAD_RIGHT_SHOULDER: usize = 5;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PAD_LEFT_TRIGGER: usize = 6;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PAD_RIGHT_TRIGGER: usize = 7;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PAD_SELECT: usize = 8;
pub(crate) const PAD_START: usize = 9;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PAD_LEFT_THUMB: usize = 10;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PAD_RIGHT_THUMB: usize = 11;
pub(crate) const PAD_UP: usize = 12;
pub(crate) const PAD_DOWN: usize = 13;
pub(crate) const PAD_LEFT: usize = 14;
pub(crate) const PAD_RIGHT: usize = 15;
pub(crate) const PAD_BUTTON_COUNT: usize = 16;
pub(crate) const DEFAULT_PAD_AXES: [usize; 2] = [0, 1];
pub(crate) const DEFAULT_AXIS_THRESHOLD: f32 = 0.5;

use debugger::launch::DeviceKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputSource {
    None,
    SystemMouse,
    Keyboard(String),
    Gamepad { identity: String, mapping: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortBinding {
    pub device: DeviceKind,
    pub source: InputSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoystickMapping {
    pub id: String,
    pub name: String,
    /// Physical Amiga key positions: up, down, left, right, fire 1, fire 2.
    pub keys: [u8; 6],
    /// Standard controller button indices in the same order.
    pub buttons: [usize; 6],
    pub axes: [usize; 2],
    pub threshold: f32,
}

impl Default for JoystickMapping {
    fn default() -> Self {
        use crate::host_keys::{
            SCANCODE_ARROW_DOWN, SCANCODE_ARROW_LEFT, SCANCODE_ARROW_RIGHT, SCANCODE_ARROW_UP,
            SCANCODE_NUMPAD0, SCANCODE_NUMPAD_DECIMAL,
        };
        Self {
            id: "arrows".into(),
            name: "Arrow keys and numpad fire".into(),
            keys: [
                SCANCODE_ARROW_UP,
                SCANCODE_ARROW_DOWN,
                SCANCODE_ARROW_LEFT,
                SCANCODE_ARROW_RIGHT,
                SCANCODE_NUMPAD0,
                SCANCODE_NUMPAD_DECIMAL,
            ],
            buttons: [PAD_UP, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_SOUTH, PAD_EAST],
            axes: DEFAULT_PAD_AXES,
            threshold: DEFAULT_AXIS_THRESHOLD,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrontendAction {
    Release,
    Pause,
    Save,
    Load,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GamepadShortcut {
    pub identity: String,
    pub button: usize,
    pub action: FrontendAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrontendBindings {
    pub release: String,
    pub pause: String,
    pub save: String,
    pub load: String,
}

impl Default for FrontendBindings {
    fn default() -> Self {
        Self {
            release: String::new(),
            pause: "F5".into(),
            save: "F6".into(),
            load: "F9".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "StoredConfiguration", into = "StoredConfiguration")]
pub struct NamedConfiguration {
    pub id: String,
    pub name: String,
    pub launch: LaunchConfiguration,
    pub controls: ControlConfiguration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaunchConfiguration {
    pub preset: A500Preset,
    pub video: VideoStandard,
    /// A library-relative native path or browser display name.
    pub rom: String,
    pub drives: [bool; config::FLOPPY_DRIVE_COUNT],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlConfiguration {
    pub ports: [PortBinding; 2],
    pub host: HostInputSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostInputSettings {
    pub mouse_sensitivity: f32,
    /// Host physical key position to guest raw keycode overrides.
    pub keyboard: Vec<(u8, u8)>,
    pub frontend: FrontendBindings,
    pub gamepad_shortcuts: Vec<GamepadShortcut>,
}

impl Default for NamedConfiguration {
    fn default() -> Self {
        Self {
            id: "standard".into(),
            name: "Standard A500".into(),
            launch: LaunchConfiguration::default(),
            controls: ControlConfiguration::default(),
        }
    }
}

impl Default for LaunchConfiguration {
    fn default() -> Self {
        Self {
            preset: A500Preset::Standard1Mb,
            video: VideoStandard::Pal,
            rom: String::new(),
            drives: [true, false, false, false],
        }
    }
}

impl Default for ControlConfiguration {
    fn default() -> Self {
        Self {
            ports: [
                PortBinding {
                    device: DeviceKind::Mouse,
                    source: InputSource::SystemMouse,
                },
                PortBinding {
                    device: DeviceKind::Joystick,
                    source: InputSource::Keyboard("arrows".into()),
                },
            ],
            host: HostInputSettings::default(),
        }
    }
}

impl Default for HostInputSettings {
    fn default() -> Self {
        Self {
            gamepad_shortcuts: Vec::new(),
            mouse_sensitivity: 1.0,
            keyboard: Vec::new(),
            frontend: FrontendBindings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigurationCatalog {
    pub configurations: Vec<NamedConfiguration>,
    pub mappings: Vec<JoystickMapping>,
    pub selected: String,
}

impl Default for ConfigurationCatalog {
    fn default() -> Self {
        Self {
            configurations: vec![NamedConfiguration::default()],
            selected: "standard".into(),
            mappings: vec![JoystickMapping::default()],
        }
    }
}

impl ConfigurationCatalog {
    pub fn validate_all(&self) -> Result<(), String> {
        if self.selected().is_none()
            && !(self.selected.is_empty() && self.configurations.is_empty())
        {
            return Err("Select an existing configuration".into());
        }
        let mut ids = Vec::new();
        for configuration in &self.configurations {
            if configuration.id.is_empty() || ids.contains(&configuration.id) {
                return Err("Configuration identities must be unique".into());
            }
            ids.push(configuration.id.clone());
            self.validate(configuration)?;
        }
        ids.clear();
        for mapping in &self.mappings {
            if mapping.id.is_empty() || ids.contains(&mapping.id) || mapping.name.trim().is_empty()
            {
                return Err("Mappings require unique identities and a name".into());
            }
            ids.push(mapping.id.clone());
            if mapping
                .keys
                .iter()
                .any(|key| *key > crate::host_keys::MAX_AMIGA_KEYCODE)
                || mapping
                    .buttons
                    .iter()
                    .any(|button| *button >= PAD_BUTTON_COUNT)
                || mapping.axes.iter().any(|axis| *axis > MAX_CONTROLLER_AXIS)
                || !(MIN_CONTROLLER_THRESHOLD..=MAX_CONTROLLER_THRESHOLD)
                    .contains(&mapping.threshold)
            {
                return Err("Choose supported mapping keys, buttons, axes and threshold".into());
            }
        }
        Ok(())
    }

    pub fn selected(&self) -> Option<&NamedConfiguration> {
        self.configurations.iter().find(|c| c.id == self.selected)
    }
    pub fn mapping(&self, id: &str) -> Option<&JoystickMapping> {
        self.mappings.iter().find(|m| m.id == id)
    }
    pub fn validate(&self, configuration: &NamedConfiguration) -> Result<(), String> {
        configuration.validate(&self.mappings)
    }
}

impl NamedConfiguration {
    pub(crate) fn validate(&self, mappings: &[JoystickMapping]) -> Result<(), String> {
        if !self.launch.drives[0] {
            return Err("The internal DF0 drive must remain connected".into());
        }
        if self.name.trim().is_empty() {
            return Err("Enter a configuration name".into());
        }
        self.controls.validate(mappings)
    }
}

impl ControlConfiguration {
    pub(crate) fn validate(&self, mappings: &[JoystickMapping]) -> Result<(), String> {
        let mapping = |id: &str| mappings.iter().find(|mapping| mapping.id == id);
        let configuration = self;
        if !(MIN_MOUSE_SENSITIVITY..=MAX_MOUSE_SENSITIVITY)
            .contains(&configuration.host.mouse_sensitivity)
        {
            return Err("Mouse sensitivity must be between 0.1 and 4".into());
        }
        if configuration
            .ports
            .iter()
            .filter(|p| p.device == DeviceKind::Mouse)
            .count()
            > 1
        {
            return Err("Only one system mouse is available".into());
        }
        let mut used_keys = Vec::new();
        let mut used_pads = Vec::new();
        let mut pad_buttons = Vec::new();
        for port in &configuration.ports {
            match (&port.device, &port.source) {
                (DeviceKind::Disconnected, InputSource::None)
                | (DeviceKind::Mouse, InputSource::SystemMouse) => {}
                (DeviceKind::Joystick, InputSource::Keyboard(id)) => {
                    let mapping = mapping(id).ok_or("Select an existing input mapping")?;
                    for key in mapping.keys {
                        if used_keys.contains(&key) {
                            return Err(format!(
                                "Joystick key {key:02X} has conflicting destinations"
                            ));
                        }
                        used_keys.push(key);
                    }
                }
                (DeviceKind::Joystick, InputSource::Gamepad { identity, mapping }) => {
                    if identity.is_empty() || used_pads.contains(identity) {
                        return Err("Select a different available controller for each port".into());
                    }
                    used_pads.push(identity.clone());
                    let mapping = mappings
                        .iter()
                        .find(|item| item.id == *mapping)
                        .ok_or("Select an existing controller mapping")?;
                    pad_buttons.extend(mapping.buttons.map(|button| (identity.clone(), button)));
                    if !(MIN_CONTROLLER_THRESHOLD..=MAX_CONTROLLER_THRESHOLD)
                        .contains(&mapping.threshold)
                    {
                        return Err("Controller threshold must be between 0.1 and 1".into());
                    }
                }
                _ => return Err("Choose a host source compatible with the emulated device".into()),
            }
        }
        let mut frontend_buttons = Vec::new();
        for shortcut in &configuration.host.gamepad_shortcuts {
            let key = (shortcut.identity.clone(), shortcut.button);
            if pad_buttons.contains(&key) || frontend_buttons.contains(&key) {
                return Err("A controller button has conflicting frontend/joystick actions".into());
            }
            frontend_buttons.push(key);
        }
        let mut overridden = Vec::new();
        for (host, guest) in &configuration.host.keyboard {
            if *host > crate::host_keys::MAX_AMIGA_KEYCODE
                || *guest > crate::host_keys::MAX_AMIGA_KEYCODE
                || overridden.contains(host)
            {
                return Err(
                    "Keyboard overrides require unique physical positions and 7-bit guest codes"
                        .into(),
                );
            }
            if used_keys.contains(host) {
                return Err("A keyboard override conflicts with a joystick key".into());
            }
            overridden.push(*host);
        }
        let shortcuts = [
            &configuration.host.frontend.release,
            &configuration.host.frontend.pause,
            &configuration.host.frontend.save,
            &configuration.host.frontend.load,
        ];
        let mut assigned = Vec::new();
        for (index, key) in shortcuts.iter().enumerate() {
            if index == 0 && key.is_empty() {
                continue;
            }
            let resolved = crate::host_keys::frontend_key(key)
                .ok_or_else(|| format!("Choose a physical frontend key: {key}"))?;
            if resolved == egui::Key::F12 {
                return Err("F12 remains reserved for Debug access".into());
            }
            if assigned.contains(&resolved) {
                return Err("Frontend shortcuts must use different keys".into());
            }
            assigned.push(resolved);
            if let Some(code) = crate::host_keys::frontend_key_code(key) {
                if overridden.contains(&code) {
                    return Err(format!(
                        "Frontend key {key} conflicts with a keyboard override"
                    ));
                }
                if used_keys.contains(&code) {
                    return Err(format!(
                        "Frontend key {key} conflicts with a joystick mapping"
                    ));
                }
            }
        }
        Ok(())
    }
}
