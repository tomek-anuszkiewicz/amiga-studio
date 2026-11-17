//! One host action policy shared by native and browser event adapters.

use crate::host_input::HostInputEvent;
use crate::player_config::{ControlConfiguration, InputSource, JoystickMapping};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GamepadInfo {
    pub identity: String,
    pub name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct GamepadSample {
    pub identity: String,
    pub name: String,
    pub axes: Vec<f32>,
    pub buttons: Vec<bool>,
}

#[derive(Debug, Default)]
pub struct InputMapping {
    configuration: Option<(ControlConfiguration, Vec<JoystickMapping>)>,
    held: HashSet<u8>,
    pads: HashMap<String, [bool; 6]>,
    blocked_pads: HashSet<String>,
    pub available: Vec<GamepadInfo>,
    action_buttons: HashMap<(String, usize), bool>,
    pub actions: Vec<crate::player_config::FrontendAction>,
}

impl InputMapping {
    pub fn configure(
        &mut self,
        configuration: &ControlConfiguration,
        mappings: &[JoystickMapping],
    ) -> bool {
        if self
            .configuration
            .as_ref()
            .is_some_and(|(c, all)| c == configuration && all == mappings)
        {
            return false;
        }
        self.release();
        self.action_buttons.clear();
        self.actions.clear();
        self.configuration = Some((configuration.clone(), mappings.to_vec()));
        true
    }
    pub fn release(&mut self) {
        self.held.clear();
        self.blocked_pads
            .extend(self.available.iter().map(|p| p.identity.clone()));
        self.pads.clear();
    }
    pub fn release_capture(&mut self) -> HostInputEvent {
        self.held.clear();
        let mut joysticks = [None; 2];
        if let Some((configuration, _)) = &self.configuration {
            for (port, binding) in configuration.ports.iter().enumerate() {
                if let InputSource::Gamepad { identity, .. } = &binding.source {
                    joysticks[port] = self.pads.get(identity).copied();
                }
            }
        }
        HostInputEvent::ReleaseCapture { joysticks }
    }
    pub fn keyboard(&mut self, code: u8, pressed: bool) -> Vec<HostInputEvent> {
        if pressed {
            if !self.held.insert(code) {
                return Vec::new();
            }
        } else {
            self.held.remove(&code);
        }
        let Some((configuration, catalog)) = &self.configuration else {
            return vec![HostInputEvent::Key { code, pressed }];
        };
        let shortcuts = [
            &configuration.host.frontend.release,
            &configuration.host.frontend.pause,
            &configuration.host.frontend.save,
            &configuration.host.frontend.load,
        ];
        if shortcuts
            .iter()
            .any(|key| crate::host_keys::frontend_key_code(key) == Some(code))
        {
            return Vec::new();
        }
        let mut events = Vec::new();
        for (port, binding) in configuration.ports.iter().enumerate() {
            let InputSource::Keyboard(id) = &binding.source else {
                continue;
            };
            let Some(mapping) = catalog.iter().find(|mapping| &mapping.id == id) else {
                continue;
            };
            if mapping.keys.contains(&code) {
                let contacts = mapping.keys.map(|key| self.held.contains(&key));
                events.push(joystick(port, contacts));
            }
        }
        if !events.is_empty() {
            return events;
        }
        let guest = configuration
            .host
            .keyboard
            .iter()
            .find(|(key, _)| *key == code)
            .map_or(code, |(_, guest)| *guest);
        // Both physical Ctrl keys share one Amiga position; retain it until both release.
        let pressed = pressed
            || self.held.iter().any(|key| {
                configuration
                    .host
                    .keyboard
                    .iter()
                    .find(|(host, _)| host == key)
                    .map_or(*key, |(_, guest)| *guest)
                    == guest
            });
        events.push(HostInputEvent::Key {
            code: guest,
            pressed,
        });
        events
    }
    pub fn pointer(&self, event: HostInputEvent) -> Vec<HostInputEvent> {
        let Some((configuration, _)) = &self.configuration else {
            return vec![event];
        };
        let Some(port) = configuration
            .ports
            .iter()
            .position(|p| p.source == InputSource::SystemMouse)
        else {
            return Vec::new();
        };
        vec![match event {
            HostInputEvent::MouseMotion { dx, dy, .. } => HostInputEvent::MouseMotion {
                port,
                dx: (dx as f32 * configuration.host.mouse_sensitivity).round() as i32,
                dy: (dy as f32 * configuration.host.mouse_sensitivity).round() as i32,
            },
            HostInputEvent::MouseButtons { left, right, .. } => HostInputEvent::MouseButtons {
                port,
                left,
                right,
                middle: false,
            },
            _ => return Vec::new(),
        }]
    }
    pub fn update_devices(&mut self, devices: Vec<GamepadInfo>) -> Vec<HostInputEvent> {
        let lost = self.available.iter().any(|old| !devices.contains(old));
        for new in &devices {
            if !self.available.contains(new) {
                self.blocked_pads.insert(new.identity.clone());
            }
        }
        self.available = devices;
        if lost {
            self.action_buttons
                .retain(|(id, _), _| self.available.iter().any(|p| &p.identity == id));
            self.release();
            vec![HostInputEvent::ReleaseAll]
        } else {
            Vec::new()
        }
    }
    pub fn frontend_gamepad(&mut self, identity: &str, buttons: &[bool], enabled: bool) {
        let Some((configuration, _)) = &self.configuration else {
            return;
        };
        for shortcut in &configuration.host.gamepad_shortcuts {
            if shortcut.identity != identity {
                continue;
            }
            let pressed = buttons.get(shortcut.button).copied().unwrap_or(false);
            let previous = self
                .action_buttons
                .entry((identity.into(), shortcut.button))
                .or_insert(true);
            if enabled && pressed && !*previous {
                self.actions.push(shortcut.action);
            }
            *previous = pressed;
        }
    }

    pub fn gamepad(
        &mut self,
        identity: &str,
        axes: &[f32],
        buttons: &[bool],
    ) -> Vec<HostInputEvent> {
        let Some((configuration, catalog)) = &self.configuration else {
            return Vec::new();
        };
        let mut events = Vec::new();
        for (port, binding) in configuration.ports.iter().enumerate() {
            let InputSource::Gamepad {
                identity: selected,
                mapping,
            } = &binding.source
            else {
                continue;
            };
            if selected != identity
                || self
                    .available
                    .iter()
                    .filter(|p| p.identity == identity)
                    .count()
                    != 1
            {
                continue;
            }
            let Some(mapping) = catalog.iter().find(|candidate| &candidate.id == mapping) else {
                continue;
            };
            let mut contacts = mapping
                .buttons
                .map(|index| buttons.get(index).copied().unwrap_or(false));
            let x = axes.get(mapping.axes[0]).copied().unwrap_or(0.0);
            let y = axes.get(mapping.axes[1]).copied().unwrap_or(0.0);
            contacts[0] |= y < -mapping.threshold;
            contacts[1] |= y > mapping.threshold;
            contacts[2] |= x < -mapping.threshold;
            contacts[3] |= x > mapping.threshold;
            if self.blocked_pads.contains(identity) {
                if contacts.iter().all(|c| !c) {
                    self.blocked_pads.remove(identity);
                }
                continue;
            }
            if self.pads.get(identity) != Some(&contacts) {
                self.pads.insert(identity.to_owned(), contacts);
                events.push(joystick(port, contacts));
            }
        }
        events
    }
}

fn joystick(port: usize, contacts: [bool; 6]) -> HostInputEvent {
    let [up, down, left, right, fire1, fire2] = contacts;
    HostInputEvent::Joystick {
        port,
        up,
        down,
        left,
        right,
        fire1,
        fire2,
    }
}
