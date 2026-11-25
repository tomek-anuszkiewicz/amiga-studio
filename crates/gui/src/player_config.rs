//! Host-owned named compositions; hardware presets remain the core authority.

use crate::host_keys::{
    amiga_key_code, amiga_key_name, frontend_binding_name, frontend_key_position,
    host_name_for_code, HostKey,
};
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
pub(crate) const PAD_WEST: usize = 2;
pub(crate) const PAD_NORTH: usize = 3;
pub(crate) const PAD_LEFT_SHOULDER: usize = 4;
pub(crate) const PAD_RIGHT_SHOULDER: usize = 5;
pub(crate) const PAD_LEFT_TRIGGER: usize = 6;
pub(crate) const PAD_RIGHT_TRIGGER: usize = 7;
pub(crate) const PAD_SELECT: usize = 8;
pub(crate) const PAD_START: usize = 9;
pub(crate) const PAD_LEFT_THUMB: usize = 10;
pub(crate) const PAD_RIGHT_THUMB: usize = 11;
pub(crate) const PAD_UP: usize = 12;
pub(crate) const PAD_DOWN: usize = 13;
pub(crate) const PAD_LEFT: usize = 14;
pub(crate) const PAD_RIGHT: usize = 15;
pub(crate) const PAD_BUTTON_COUNT: usize = 16;
pub(crate) const JOYSTICK_CONTACTS: [&str; 6] = ["Up", "Down", "Left", "Right", "Fire 1", "Fire 2"];

pub(crate) fn controller_button_name(button: usize) -> String {
    match button {
        PAD_SOUTH => "South button",
        PAD_EAST => "East button",
        PAD_WEST => "West button",
        PAD_NORTH => "North button",
        PAD_LEFT_SHOULDER => "Left shoulder",
        PAD_RIGHT_SHOULDER => "Right shoulder",
        PAD_LEFT_TRIGGER => "Left trigger",
        PAD_RIGHT_TRIGGER => "Right trigger",
        PAD_SELECT => "Select",
        PAD_START => "Start",
        PAD_LEFT_THUMB => "Left stick press",
        PAD_RIGHT_THUMB => "Right stick press",
        PAD_UP => "D-pad Up",
        PAD_DOWN => "D-pad Down",
        PAD_LEFT => "D-pad Left",
        PAD_RIGHT => "D-pad Right",
        _ => return format!("Button {button}"),
    }
    .into()
}

pub(crate) fn controller_axis_name(axis: usize) -> String {
    match axis {
        0 => "Left stick horizontal".into(),
        1 => "Left stick vertical".into(),
        2 => "Right stick horizontal".into(),
        3 => "Right stick vertical".into(),
        _ => format!("Axis {axis}"),
    }
}

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
            SCANCODE_KEY_X, SCANCODE_KEY_Z,
        };
        Self {
            id: "arrows".into(),
            name: "Arrow keys".into(),
            keys: [
                SCANCODE_ARROW_UP,
                SCANCODE_ARROW_DOWN,
                SCANCODE_ARROW_LEFT,
                SCANCODE_ARROW_RIGHT,
                SCANCODE_KEY_Z,
                SCANCODE_KEY_X,
            ],
            buttons: [PAD_UP, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_SOUTH, PAD_EAST],
            axes: DEFAULT_PAD_AXES,
            threshold: DEFAULT_AXIS_THRESHOLD,
        }
    }
}

impl JoystickMapping {
    fn wasd(id: String) -> Self {
        use crate::host_keys::{
            SCANCODE_KEY_A, SCANCODE_KEY_D, SCANCODE_KEY_J, SCANCODE_KEY_K, SCANCODE_KEY_S,
            SCANCODE_KEY_W,
        };
        Self {
            id,
            name: "WASD".into(),
            keys: [
                SCANCODE_KEY_W,
                SCANCODE_KEY_S,
                SCANCODE_KEY_A,
                SCANCODE_KEY_D,
                SCANCODE_KEY_J,
                SCANCODE_KEY_K,
            ],
            ..Self::default()
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

impl FrontendAction {
    pub(crate) const ALL: [Self; 5] = [
        Self::Release,
        Self::Pause,
        Self::Save,
        Self::Load,
        Self::Debug,
    ];
    pub(crate) const EDITABLE_KEYS: [Self; 4] =
        [Self::Release, Self::Pause, Self::Save, Self::Load];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Release => "Release mouse",
            Self::Pause => "Pause / Resume",
            Self::Save => "Quick save",
            Self::Load => "Quick load",
            Self::Debug => "Open developer view",
        }
    }
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

impl FrontendBindings {
    pub(crate) fn binding(&self, action: FrontendAction) -> &str {
        match action {
            FrontendAction::Release => &self.release,
            FrontendAction::Pause => &self.pause,
            FrontendAction::Save => &self.save,
            FrontendAction::Load => &self.load,
            FrontendAction::Debug => "",
        }
    }

    pub(crate) fn assign(&mut self, action: FrontendAction, key: String) {
        match action {
            FrontendAction::Release => self.release = key,
            FrontendAction::Pause => self.pause = key,
            FrontendAction::Save => self.save = key,
            FrontendAction::Load => self.load = key,
            FrontendAction::Debug => {}
        }
    }
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

impl ControlConfiguration {
    pub(crate) fn uses_mapping(&self, id: &str) -> bool {
        self.ports.iter().any(|port| match &port.source {
            InputSource::Keyboard(mapping) | InputSource::Gamepad { mapping, .. } => mapping == id,
            InputSource::None | InputSource::SystemMouse => false,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostInputSettings {
    pub mouse_sensitivity: f32,
    /// Host physical key position to guest raw keycode overrides.
    pub keyboard: Vec<(HostKey, u8)>,
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
    // Zero marks catalogs saved before the two current presets were introduced.
    #[serde(default)]
    pub(crate) next_mapping_id: u64,
}

impl Default for ConfigurationCatalog {
    fn default() -> Self {
        Self {
            configurations: vec![NamedConfiguration::default()],
            selected: "standard".into(),
            mappings: vec![
                JoystickMapping::default(),
                JoystickMapping::wasd("wasd".into()),
            ],
            next_mapping_id: 1,
        }
    }
}

impl ConfigurationCatalog {
    fn next_mapping_id(&mut self) -> String {
        loop {
            let id = format!("mapping-{}", self.next_mapping_id);
            self.next_mapping_id = self.next_mapping_id.checked_add(1).unwrap_or(1);
            if self.mapping(&id).is_none() {
                return id;
            }
        }
    }

    pub(crate) fn create_mapping(&mut self) -> String {
        let id = self.next_mapping_id();
        let mut mapping = JoystickMapping::wasd(id.clone());
        mapping.name = "New mapping".into();
        self.mappings.push(mapping);
        id
    }

    pub(crate) fn introduce_keyboard_presets(&mut self) -> bool {
        if self.next_mapping_id != 0 {
            return false;
        }
        self.next_mapping_id = 1;
        let mut arrow_keys = JoystickMapping::default();
        let mut old_default = arrow_keys.clone();
        old_default.name = "Arrow keys and numpad fire".into();
        old_default.keys[4] = crate::host_keys::SCANCODE_NUMPAD0;
        old_default.keys[5] = crate::host_keys::SCANCODE_NUMPAD_DECIMAL;
        let old = self
            .mappings
            .iter()
            .position(|mapping| *mapping == old_default);
        if self.mapping(&arrow_keys.id) != Some(&arrow_keys) {
            if let Some(index) = old {
                let mut proposed = self.clone();
                proposed.mappings[index] = arrow_keys.clone();
                if self.configurations.iter().all(|configuration| {
                    let before = configuration.controls.validate(&self.mappings);
                    let after = configuration.controls.validate(&proposed.mappings);
                    after.is_ok() || after == before
                }) {
                    self.mappings[index] = arrow_keys;
                } else {
                    let id = self.next_mapping_id();
                    self.mappings.push(JoystickMapping { id, ..arrow_keys });
                }
            } else {
                if self.mapping(&arrow_keys.id).is_some() {
                    arrow_keys.id = self.next_mapping_id();
                }
                self.mappings.push(arrow_keys);
            }
        }
        let mut wasd = JoystickMapping::wasd("wasd".into());
        if self.mapping(&wasd.id) != Some(&wasd) {
            if self.mapping(&wasd.id).is_some() {
                wasd.id = self.next_mapping_id();
            }
            self.mappings.push(wasd);
        }
        true
    }

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
        for (index, port) in configuration.ports.iter().enumerate() {
            match (&port.device, &port.source) {
                (DeviceKind::Disconnected, InputSource::None)
                | (DeviceKind::Mouse, InputSource::SystemMouse) => {}
                (DeviceKind::Joystick, InputSource::Keyboard(id)) => {
                    let mapping = mapping(id).ok_or("Select an existing input mapping")?;
                    for (key, contact) in mapping.keys.into_iter().zip(JOYSTICK_CONTACTS) {
                        let assignment =
                            format!("{} {} on port {}", mapping.name, contact, index + 1);
                        if let Some((_, previous)) = used_keys.iter().find(|(used, _)| *used == key)
                        {
                            return Err(format!(
                                "{} has conflicting destinations. Change {previous} or {assignment}.",
                                host_name_for_code(key)
                            ));
                        }
                        used_keys.push((key, assignment));
                    }
                }
                (
                    DeviceKind::Joystick,
                    InputSource::Gamepad {
                        identity,
                        mapping: id,
                    },
                ) => {
                    if identity.is_empty() || used_pads.contains(&identity.as_str()) {
                        return Err("Select a different available controller for each port".into());
                    }
                    used_pads.push(identity.as_str());
                    let mapping = mapping(id).ok_or("Select an existing controller mapping")?;
                    for (button, contact) in mapping.buttons.into_iter().zip(JOYSTICK_CONTACTS) {
                        pad_buttons.push((
                            identity.as_str(),
                            button,
                            format!("{} {contact}", mapping.name),
                        ));
                    }
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
            let key = (shortcut.identity.as_str(), shortcut.button);
            let previous = pad_buttons
                .iter()
                .find(|(identity, button, _)| (*identity, *button) == key)
                .map(|(_, _, assignment)| assignment.as_str())
                .or_else(|| {
                    frontend_buttons
                        .iter()
                        .find_map(|(used, name)| (*used == key).then_some(*name))
                });
            if let Some(previous) = previous {
                return Err(format!(
                    "{} has conflicting frontend/joystick actions. Change {previous} or {}.",
                    controller_button_name(shortcut.button),
                    shortcut.action.label()
                ));
            }
            frontend_buttons.push((key, shortcut.action.label()));
        }
        let overrides = &configuration.host.keyboard;
        for (index, (host, guest)) in overrides.iter().enumerate() {
            if host
                .default_code()
                .is_some_and(|code| code > crate::host_keys::MAX_AMIGA_KEYCODE)
                || *guest > crate::host_keys::MAX_AMIGA_KEYCODE
            {
                return Err(
                    "Keyboard overrides require unique physical positions and 7-bit guest codes"
                        .into(),
                );
            }
            if matches!(host, HostKey::Position(position)
                if !HostKey::valid_position(position)
                    || position == crate::keyboard_input::VIEW_SWITCH_KEY)
            {
                return Err("Choose an available physical key other than F12.".into());
            }
            if let Some((_, previous)) = overrides[..index]
                .iter()
                .find(|(used, _)| used.matches_source(host))
            {
                return Err(format!(
                    "Keyboard overrides require unique physical positions. Change {} assigned to {}.",
                    host.name(), amiga_key_name(*previous)
                ));
            }
            let default = host.default_code();
            if let Some((_, assignment)) = used_keys.iter().find(|(used, _)| Some(*used) == default)
            {
                return Err(format!(
                    "{} is a joystick key. Change {assignment} first or choose another key.",
                    host.name()
                ));
            }
        }
        let mut assigned = Vec::new();
        for action in FrontendAction::EDITABLE_KEYS {
            let key = configuration.host.frontend.binding(action);
            if action == FrontendAction::Release && key.is_empty() {
                continue;
            }
            let position = frontend_key_position(key).ok_or_else(|| {
                format!(
                    "Choose a supported physical frontend key for {}.",
                    action.label()
                )
            })?;
            let name = frontend_binding_name(key);
            if position == crate::keyboard_input::VIEW_SWITCH_KEY {
                return Err(
                    "F12 remains reserved for switching between Game and Developer Studio.".into(),
                );
            }
            if let Some((_, previous)) = assigned.iter().find(|(used, _)| *used == position) {
                return Err(format!(
                    "Emulator shortcuts must use different keys. Change {previous} first or choose a key other than {name}."
                ));
            }
            assigned.push((position, action.label()));
            let source = HostKey::Position(position.into());
            if action != FrontendAction::Release || position != "Escape" {
                if let Some((_, guest)) = overrides
                    .iter()
                    .find(|(used, _)| used.matches_source(&source))
                {
                    return Err(format!(
                        "{name} is assigned to {} by a keyboard override. Change that override or the {} shortcut.",
                        amiga_key_name(*guest), action.label()
                    ));
                }
            }
            if let Some(code) = amiga_key_code(position) {
                if let Some((_, assignment)) = used_keys.iter().find(|(used, _)| *used == code) {
                    return Err(format!(
                        "{name} is used by a joystick mapping. Change {assignment} or the {} shortcut.",
                        action.label()
                    ));
                }
            }
        }
        Ok(())
    }
}
