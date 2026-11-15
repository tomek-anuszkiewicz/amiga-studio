//! Desktop OS and controller event listeners, outside emulated time.

use crate::host_input::HostInputEvent;
use rdev::{Button, EventType, Key};
use std::collections::HashSet;

const AXIS_THRESHOLD: f32 = 0.5;

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
                        Key::KeyR
                            | Key::KeyO
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
                (dx != 0 || dy != 0).then_some(HostInputEvent::MouseMotion { dx, dy })
            }
            EventType::ButtonPress(button) | EventType::ButtonRelease(button) => {
                if !enabled {
                    self.buttons = [false; 3];
                    return None;
                }
                let index = match button {
                    Button::Left => 0,
                    Button::Right => 1,
                    Button::Middle => 2,
                    _ => return None,
                };
                self.buttons[index] = matches!(event, EventType::ButtonPress(_));
                Some(HostInputEvent::MouseButtons {
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
    selected: Option<gilrs::GamepadId>,
    previous: Option<HostInputEvent>,
    capture: Option<(u64, u64)>,
}

impl NativeGamepads {
    pub(crate) fn new() -> Option<Self> {
        match gilrs::GilrsBuilder::new()
            .with_force_feedback(false)
            .build()
        {
            Ok(context) => Some(Self {
                context,
                selected: None,
                previous: None,
                capture: None,
            }),
            Err(error) => {
                eprintln!("Native gamepad input unavailable: {error}");
                None
            }
        }
    }

    pub(crate) fn changed_events(&mut self, capture: (u64, u64)) -> [Option<HostInputEvent>; 2] {
        use gilrs::{Axis, Button};
        if self.capture != Some(capture) {
            self.release();
            self.capture = Some(capture);
        }
        let mut events = [None; 2];
        let current = self.context.gamepads().find(|(_, pad)| pad.is_connected());
        let selected = current.as_ref().map(|(id, _)| *id);
        if selected != self.selected {
            events[0] = Some(HostInputEvent::Joystick {
                up: false,
                down: false,
                left: false,
                right: false,
                fire1: false,
                fire2: false,
            });
            self.previous = None;
            self.selected = selected;
        }
        if let Some((_, pad)) = current {
            let x = pad.value(Axis::LeftStickX);
            let y = pad.value(Axis::LeftStickY);
            let state = HostInputEvent::Joystick {
                up: y > AXIS_THRESHOLD || pad.is_pressed(Button::DPadUp),
                down: y < -AXIS_THRESHOLD || pad.is_pressed(Button::DPadDown),
                left: x < -AXIS_THRESHOLD || pad.is_pressed(Button::DPadLeft),
                right: x > AXIS_THRESHOLD || pad.is_pressed(Button::DPadRight),
                fire1: pad.is_pressed(Button::South),
                fire2: pad.is_pressed(Button::East),
            };
            if self.previous != Some(state) {
                events[1] = Some(state);
                self.previous = Some(state);
            }
        }
        events
    }

    pub(crate) fn release(&mut self) {
        self.previous = None;
        self.selected = None;
        self.capture = None;
    }
}

pub(crate) fn listen_gamepads() {
    let Some(mut gamepads) = NativeGamepads::new() else {
        return;
    };
    while let Some(event) = gamepads.context.next_event_blocking(None) {
        crate::host_runtime::collect_native_gamepad(&mut gamepads, event.time);
    }
}
