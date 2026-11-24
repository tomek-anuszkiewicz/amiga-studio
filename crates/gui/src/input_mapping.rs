//! One host action policy shared by native and browser event adapters.

use crate::host_input::HostInputEvent;
use crate::player_config::{
    ControlConfiguration, InputSource, JoystickMapping, MAX_CONTROLLER_AXIS, PAD_BUTTON_COUNT,
};
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum GamepadInput {
    Button,
    Axis { threshold: f32 },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GamepadRecording {
    pub(crate) identity: String,
    pub(crate) input: GamepadInput,
    pub(crate) started: u64,
}

#[derive(Debug)]
pub(crate) enum GamepadCapture {
    Input(usize),
    Unavailable,
}

#[derive(Debug)]
pub(crate) struct GamepadRecorder {
    pub(crate) request: GamepadRecording,
    pub(crate) captured: Option<GamepadCapture>,
    neutral: bool,
}

impl GamepadRecorder {
    pub(crate) fn new(request: GamepadRecording, samples: &[GamepadSample]) -> Self {
        let mut recorder = Self {
            request,
            captured: None,
            neutral: false,
        };
        if let Some(sample) = recorder.device(samples) {
            recorder.neutral = recorder.active(sample).is_none();
        } else {
            recorder.captured = Some(GamepadCapture::Unavailable);
        }
        recorder
    }

    fn device<'a>(&self, samples: &'a [GamepadSample]) -> Option<&'a GamepadSample> {
        let mut matches = samples
            .iter()
            .filter(|sample| sample.identity == self.request.identity);
        let sample = matches.next()?;
        matches.next().is_none().then_some(sample)
    }

    fn active(&self, sample: &GamepadSample) -> Option<usize> {
        match self.request.input {
            GamepadInput::Button => sample
                .buttons
                .iter()
                .take(PAD_BUTTON_COUNT)
                .position(|&value| value),
            GamepadInput::Axis { threshold } => sample
                .axes
                .iter()
                .take(MAX_CONTROLLER_AXIS + 1)
                .enumerate()
                .filter(|(_, value)| value.is_finite() && value.abs() > threshold)
                .max_by(|(_, first), (_, second)| first.abs().total_cmp(&second.abs()))
                .map(|(index, _)| index),
        }
    }

    pub(crate) fn sample(&mut self, samples: &[GamepadSample], fresh: bool) -> bool {
        let Some(sample) = self.device(samples) else {
            self.captured = Some(GamepadCapture::Unavailable);
            return true;
        };
        let active = self.active(sample);
        let captured = active.filter(|_| fresh && self.neutral && self.captured.is_none());
        self.neutral = fresh && active.is_none();
        if let Some(index) = captured {
            self.captured = Some(GamepadCapture::Input(index));
            return true;
        }
        false
    }
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
    pub(crate) fn release_for_view_change(&mut self) {
        self.held.clear();
        self.pads.retain(|_, contacts| *contacts == [false; 6]);
        self.blocked_pads.extend(
            self.available
                .iter()
                .filter(|pad| !self.pads.contains_key(&pad.identity))
                .map(|pad| pad.identity.clone()),
        );
    }
    pub(crate) fn mouse_release(&self, default_port: usize) -> Option<HostInputEvent> {
        let port = self.mouse_port(default_port)?;
        Some(HostInputEvent::ReleaseMouse { port })
    }

    fn mouse_port(&self, default_port: usize) -> Option<usize> {
        match &self.configuration {
            Some((configuration, _)) => configuration
                .ports
                .iter()
                .position(|port| port.source == InputSource::SystemMouse),
            None => Some(default_port),
        }
    }

    pub fn keyboard(&mut self, code: u8, pressed: bool) -> Vec<HostInputEvent> {
        let changed = if pressed {
            self.held.insert(code)
        } else {
            self.held.remove(&code)
        };
        if !changed {
            return Vec::new();
        }
        let Some((configuration, catalog)) = &self.configuration else {
            return vec![HostInputEvent::Key { code, pressed }];
        };
        let mut events = Vec::new();
        let keyboard_mappings = || {
            configuration
                .ports
                .iter()
                .enumerate()
                .filter_map(|(port, binding)| {
                    let InputSource::Keyboard(id) = &binding.source else {
                        return None;
                    };
                    Some((port, catalog.iter().find(|mapping| &mapping.id == id)?))
                })
        };
        for (port, mapping) in keyboard_mappings() {
            if mapping.keys.contains(&code) {
                let contacts = mapping.keys.map(|key| self.held.contains(&key));
                events.push(joystick(port, contacts));
            }
        }
        if !events.is_empty() {
            return events;
        }
        let translated = |code| {
            configuration
                .host
                .keyboard
                .iter()
                .find(|(host, _)| *host == code)
                .map_or(code, |(_, guest)| *guest)
        };
        let guest = translated(code);
        let other_source_held = self.held.iter().any(|&key| {
            key != code
                && translated(key) == guest
                && !keyboard_mappings().any(|(_, mapping)| mapping.keys.contains(&key))
        });
        if !other_source_held {
            events.push(HostInputEvent::Key {
                code: guest,
                pressed,
            });
        }
        events
    }
    pub fn pointer(&self, event: HostInputEvent) -> Vec<HostInputEvent> {
        let Some((configuration, _)) = &self.configuration else {
            return vec![event];
        };
        let Some(port) = self.mouse_port(0) else {
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
        self.sample_gamepad(identity, axes, buttons, true)
    }

    pub(crate) fn sample_gamepad(
        &mut self,
        identity: &str,
        axes: &[f32],
        buttons: &[bool],
        fresh: bool,
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
            let neutral = contacts == [false; 6];
            if !fresh {
                // A later native event can sample this old press again.
                if !neutral && self.pads.get(identity) == Some(&[false; 6]) {
                    self.pads.remove(identity);
                    self.blocked_pads.insert(identity.to_owned());
                }
                continue;
            }
            if self.blocked_pads.contains(identity) {
                if neutral {
                    self.blocked_pads.remove(identity);
                    self.pads.insert(identity.to_owned(), contacts);
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
