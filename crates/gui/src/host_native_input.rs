//! Desktop OS and controller event listeners, outside emulated time.

use crate::host_input::HostInputEvent;
use rdev::{Button, EventType, Key};
use std::collections::{HashMap, HashSet};

fn physical_code(key: Key) -> Option<u8> {
    crate::host_keys::amiga_key_code(match key {
        Key::Alt => "AltLeft",
        Key::AltGr => "AltRight",
        Key::Backspace => "Backspace",
        Key::CapsLock => "CapsLock",
        Key::ControlLeft => "ControlLeft",
        Key::ControlRight => "ControlRight",
        Key::Delete => "Delete",
        Key::DownArrow => "ArrowDown",
        Key::Escape => "Escape",
        Key::LeftArrow => "ArrowLeft",
        Key::MetaLeft => "MetaLeft",
        Key::MetaRight => "MetaRight",
        Key::Return => "Enter",
        Key::RightArrow => "ArrowRight",
        Key::ShiftLeft => "ShiftLeft",
        Key::ShiftRight => "ShiftRight",
        Key::Space => "Space",
        Key::Tab => "Tab",
        Key::UpArrow => "ArrowUp",
        Key::BackQuote => "Backquote",
        Key::Minus => "Minus",
        Key::Equal => "Equal",
        Key::Num0 => "Digit0",
        Key::Num1 => "Digit1",
        Key::Num2 => "Digit2",
        Key::Num3 => "Digit3",
        Key::Num4 => "Digit4",
        Key::Num5 => "Digit5",
        Key::Num6 => "Digit6",
        Key::Num7 => "Digit7",
        Key::Num8 => "Digit8",
        Key::Num9 => "Digit9",
        Key::KeyA => "KeyA",
        Key::KeyB => "KeyB",
        Key::KeyC => "KeyC",
        Key::KeyD => "KeyD",
        Key::KeyE => "KeyE",
        Key::KeyF => "KeyF",
        Key::KeyG => "KeyG",
        Key::KeyH => "KeyH",
        Key::KeyI => "KeyI",
        Key::KeyJ => "KeyJ",
        Key::KeyK => "KeyK",
        Key::KeyL => "KeyL",
        Key::KeyM => "KeyM",
        Key::KeyN => "KeyN",
        Key::KeyO => "KeyO",
        Key::KeyP => "KeyP",
        Key::KeyQ => "KeyQ",
        Key::KeyR => "KeyR",
        Key::KeyS => "KeyS",
        Key::KeyT => "KeyT",
        Key::KeyU => "KeyU",
        Key::KeyV => "KeyV",
        Key::KeyW => "KeyW",
        Key::KeyX => "KeyX",
        Key::KeyY => "KeyY",
        Key::KeyZ => "KeyZ",
        Key::LeftBracket => "BracketLeft",
        Key::RightBracket => "BracketRight",
        Key::SemiColon => "Semicolon",
        Key::Quote => "Quote",
        Key::BackSlash => "Backslash",
        Key::IntlBackslash => "IntlBackslash",
        Key::Comma => "Comma",
        Key::Dot => "Period",
        Key::Slash => "Slash",
        Key::Insert => "Insert",
        Key::KpReturn => "NumpadEnter",
        Key::KpMinus => "NumpadSubtract",
        Key::KpPlus => "NumpadAdd",
        Key::KpMultiply => "NumpadMultiply",
        Key::KpDivide => "NumpadDivide",
        Key::Kp0 => "Numpad0",
        Key::Kp1 => "Numpad1",
        Key::Kp2 => "Numpad2",
        Key::Kp3 => "Numpad3",
        Key::Kp4 => "Numpad4",
        Key::Kp5 => "Numpad5",
        Key::Kp6 => "Numpad6",
        Key::Kp7 => "Numpad7",
        Key::Kp8 => "Numpad8",
        Key::Kp9 => "Numpad9",
        Key::KpDelete => "NumpadDecimal",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F7 => "F7",
        Key::F8 => "F8",
        // Existing frontend shortcuts retain their meaning.
        _ => return None,
    })
}

#[derive(Debug, Default)]
pub struct NativeEvents {
    held: HashSet<Key>,
    position: Option<(f64, f64)>,
    buttons: [bool; 3],
    capture: Option<(u64, u64)>,
}

impl NativeEvents {
    pub(crate) fn capture(&mut self, epoch: (u64, u64)) {
        if self.capture != Some(epoch) {
            self.buttons = [false; 3];
            self.capture = Some(epoch);
        }
    }
    pub fn translate(&mut self, event: EventType, enabled: bool) -> Option<HostInputEvent> {
        match event {
            EventType::KeyPress(key) | EventType::KeyRelease(key) => {
                let pressed = matches!(event, EventType::KeyPress(_));
                let changed = if pressed {
                    self.held.insert(key)
                } else {
                    self.held.remove(&key)
                };
                if !enabled || !changed {
                    return None;
                }
                let command = self.held.contains(&Key::ControlLeft)
                    || self.held.contains(&Key::ControlRight)
                    || self.held.contains(&Key::MetaLeft)
                    || self.held.contains(&Key::MetaRight);
                if pressed
                    && command
                    && matches!(
                        key,
                        Key::KeyO
                            | Key::KeyS
                            | Key::KeyL
                            | Key::KeyT
                            | Key::Equal
                            | Key::Minus
                            | Key::Num0
                    )
                {
                    return None;
                }
                let code = physical_code(key)?;
                let pressed = if matches!(key, Key::ControlLeft | Key::ControlRight) {
                    self.held.contains(&Key::ControlLeft) || self.held.contains(&Key::ControlRight)
                } else {
                    pressed
                };
                Some(HostInputEvent::Key { code, pressed })
            }
            EventType::MouseMove { x, y } => {
                let previous = self.position.replace((x, y));
                if !enabled {
                    self.buttons = [false; 3];
                    return None;
                }
                let (old_x, old_y) = previous?;
                let dx = (x - old_x).round() as i32;
                let dy = (y - old_y).round() as i32;
                (dx != 0 || dy != 0).then_some(HostInputEvent::MouseMotion { port: 0, dx, dy })
            }
            EventType::ButtonPress(button) | EventType::ButtonRelease(button) => {
                if !enabled {
                    self.buttons = [false; 3];
                    return None;
                }
                let index = match button {
                    Button::Left => 0,
                    Button::Right => 1,
                    _ => return None,
                };
                self.buttons[index] = matches!(event, EventType::ButtonPress(_));
                Some(HostInputEvent::MouseButtons {
                    port: 0,
                    left: self.buttons[0],
                    right: self.buttons[1],
                    middle: self.buttons[2],
                })
            }
            _ => None,
        }
    }
}

pub(crate) fn listen() {
    let mut events = NativeEvents::default();
    if let Err(error) = rdev::listen(move |event| {
        crate::host_runtime::collect_native_event(&mut events, event);
    }) {
        eprintln!("Native keyboard/mouse input unavailable: {error:?}");
    }
}

#[derive(Debug)]
pub(crate) struct NativeGamepads {
    context: gilrs::Gilrs,
    identities: HashMap<gilrs::GamepadId, u64>,
    serial: u64,
    session: u128,
}

impl NativeGamepads {
    pub(crate) fn new() -> Option<Self> {
        gilrs::GilrsBuilder::new()
            .with_force_feedback(false)
            .build()
            .map(|context| Self {
                context,
                identities: HashMap::new(),
                serial: 0,
                session: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos(),
            })
            .map_err(|error| eprintln!("Native gamepad input unavailable: {error}"))
            .ok()
    }
    pub(crate) fn samples(&mut self) -> Vec<crate::input_mapping::GamepadSample> {
        use crate::player_config::{
            PAD_BUTTON_COUNT, PAD_DOWN, PAD_EAST, PAD_LEFT, PAD_LEFT_SHOULDER, PAD_LEFT_THUMB,
            PAD_LEFT_TRIGGER, PAD_NORTH, PAD_RIGHT, PAD_RIGHT_SHOULDER, PAD_RIGHT_THUMB,
            PAD_RIGHT_TRIGGER, PAD_SELECT, PAD_SOUTH, PAD_START, PAD_UP, PAD_WEST,
        };
        use gilrs::{Axis, Button};
        self.identities
            .retain(|id, _| self.context.gamepad(*id).is_connected());
        for (id, pad) in self.context.gamepads() {
            if pad.is_connected() && !self.identities.contains_key(&id) {
                self.serial = self.serial.wrapping_add(1);
                self.identities.insert(id, self.serial);
            }
        }
        self.context
            .gamepads()
            .filter(|(_, pad)| pad.is_connected())
            .map(|(id, pad)| {
                let mut buttons = vec![false; PAD_BUTTON_COUNT];
                for (index, button) in [
                    (PAD_SOUTH, Button::South),
                    (PAD_EAST, Button::East),
                    (PAD_WEST, Button::West),
                    (PAD_NORTH, Button::North),
                    (PAD_LEFT_SHOULDER, Button::LeftTrigger),
                    (PAD_RIGHT_SHOULDER, Button::RightTrigger),
                    (PAD_LEFT_TRIGGER, Button::LeftTrigger2),
                    (PAD_RIGHT_TRIGGER, Button::RightTrigger2),
                    (PAD_LEFT_THUMB, Button::LeftThumb),
                    (PAD_RIGHT_THUMB, Button::RightThumb),
                    (PAD_SELECT, Button::Select),
                    (PAD_START, Button::Start),
                    (PAD_UP, Button::DPadUp),
                    (PAD_DOWN, Button::DPadDown),
                    (PAD_LEFT, Button::DPadLeft),
                    (PAD_RIGHT, Button::DPadRight),
                ] {
                    buttons[index] = pad.is_pressed(button);
                }
                crate::input_mapping::GamepadSample {
                    identity: format!(
                        "native-{}-{}-{}-{:x?}",
                        self.session,
                        self.identities.get(&id).copied().unwrap_or_default(),
                        usize::from(id),
                        pad.uuid()
                    ),
                    name: pad.name().into(),
                    axes: vec![
                        pad.value(Axis::LeftStickX),
                        -pad.value(Axis::LeftStickY),
                        pad.value(Axis::RightStickX),
                        -pad.value(Axis::RightStickY),
                    ],
                    buttons,
                }
            })
            .collect()
    }
}

pub(crate) fn listen_gamepads() {
    let Some(mut gamepads) = NativeGamepads::new() else {
        return;
    };
    crate::host_runtime::collect_native_gamepad(&mut gamepads, std::time::SystemTime::now());
    while let Some(event) = gamepads.context.next_event_blocking(None) {
        crate::host_runtime::collect_native_gamepad(&mut gamepads, event.time);
    }
}
