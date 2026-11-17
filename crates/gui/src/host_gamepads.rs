//! Native gamepad listener, outside emulated time.

use std::collections::HashMap;

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
