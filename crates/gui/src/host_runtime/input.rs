//! GUI-side input activation, configuration and browser ingress.

use super::{HostInputEvent, HostInputSender};

impl HostInputSender {
    pub fn configure(
        &self,
        configuration: &crate::player_config::ControlConfiguration,
        mappings: &[crate::player_config::JoystickMapping],
    ) -> bool {
        let Ok(mut mailboxes) = self.shared.mailboxes.try_lock() else {
            return false;
        };
        if mailboxes.mapping.configure(configuration, mappings) {
            mailboxes.push_input(HostInputEvent::ReleaseAll);
        }
        true
    }

    pub fn gamepads(&self) -> Option<Vec<crate::input_mapping::GamepadInfo>> {
        self.shared
            .mailboxes
            .try_lock()
            .ok()
            .map(|m| m.mapping.available.clone())
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn browser_key(&self, json: &str, generation: u64, focused: bool) -> bool {
        let Ok(key) = serde_json::from_str::<crate::keyboard_input::PhysicalKey>(json) else {
            return false;
        };
        crate::keyboard_input::receive_focus(&self.repaint, focused, Some(generation));
        let handled = crate::keyboard_input::receive_key(&self.repaint, key, Some(generation));
        self.repaint.request_repaint();
        handled
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn browser_focus(&self, generation: u64, focused: bool) {
        crate::keyboard_input::receive_focus(&self.repaint, focused, Some(generation));
        self.repaint.request_repaint();
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn browser_gamepads(&self, json: &str, generation: u64) -> bool {
        #[derive(serde::Deserialize)]
        struct BrowserGamepads {
            devices: Vec<crate::input_mapping::GamepadSample>,
            focused: bool,
        }
        if let Ok(sample) = serde_json::from_str::<BrowserGamepads>(json) {
            return self.try_gamepads(sample.devices, generation, sample.focused);
        }
        true
    }

    pub fn try_gamepads(
        &self,
        samples: Vec<crate::input_mapping::GamepadSample>,
        generation: u64,
        focused: bool,
    ) -> bool {
        let Ok(mut mailboxes) = self.shared.mailboxes.try_lock() else {
            return false;
        };
        if generation != mailboxes.status.generation {
            return true;
        }
        let active = focused && mailboxes.gamepads_active();
        if mailboxes.sample_gamepads(samples, active, focused) {
            self.repaint.request_repaint();
        }
        true
    }

    pub fn frontend_actions(
        &self,
        enabled: bool,
    ) -> Option<Vec<crate::player_config::FrontendAction>> {
        let Ok(mut mailboxes) = self.shared.mailboxes.try_lock() else {
            return None;
        };
        mailboxes.frontend_enabled = enabled;
        let actions = std::mem::take(&mut mailboxes.mapping.actions);
        Some(if enabled { actions } else { Vec::new() })
    }

    pub fn try_host_event(&self, event: HostInputEvent, generation: u64) -> bool {
        self.try_input(event, generation, false)
    }

    /// GUI-routed keys wait for owner activation instead of being discarded on stale telemetry.
    pub(crate) fn try_keyboard_event(&self, event: HostInputEvent, generation: u64) -> bool {
        self.try_input(event, generation, true)
    }

    fn try_input(&self, event: HostInputEvent, generation: u64, retry_inactive: bool) -> bool {
        let Ok(mut mailboxes) = self.shared.mailboxes.try_lock() else {
            return false;
        };
        if generation != mailboxes.status.generation {
            return true;
        }
        let active = match event {
            HostInputEvent::Key { .. } => mailboxes.keyboard_enabled && mailboxes.gameplay_active(),
            HostInputEvent::MouseMotion { .. } | HostInputEvent::MouseButtons { .. } => {
                mailboxes.mouse_enabled && mailboxes.gameplay_active()
            }
            HostInputEvent::Joystick { .. } => mailboxes.gamepads_active(),
            HostInputEvent::ReleaseAll | HostInputEvent::ReleaseMouse { .. } => true,
        };
        if active {
            mailboxes.resolve_input(event);
        }
        active || !retry_inactive
    }

    pub fn set_enabled(&self, keyboard: bool, mouse: bool, gamepads: bool) -> bool {
        self.apply_input_activation(keyboard, mouse, gamepads, None)
    }

    pub(crate) fn apply_input_activation(
        &self,
        keyboard: bool,
        mouse: bool,
        gamepads: bool,
        view_release_generation: Option<u64>,
    ) -> bool {
        let Ok(mut mailboxes) = self.shared.mailboxes.try_lock() else {
            return false;
        };
        let gameplay_lost =
            (mailboxes.keyboard_enabled && !keyboard) || (mailboxes.gamepad_enabled && !gamepads);
        let view_changed = view_release_generation == Some(mailboxes.status.generation);
        #[cfg(not(target_arch = "wasm32"))]
        if view_changed || mailboxes.gamepad_enabled != gamepads {
            mailboxes.gamepads_started_at = std::time::SystemTime::now();
        }
        #[cfg(target_arch = "wasm32")]
        if view_changed {
            let snapshot = super::browser::discard_buffered_host_controls();
            if !gameplay_lost {
                let samples: Vec<crate::input_mapping::GamepadSample> =
                    serde_json::from_str(&snapshot).unwrap_or_default();
                // Observe physical contacts at the switch without delivering their edges.
                for pad in samples {
                    mailboxes
                        .mapping
                        .sample_gamepad(&pad.identity, &pad.axes, &pad.buttons, true);
                }
            }
        }
        if gameplay_lost {
            mailboxes.resolve_input(HostInputEvent::ReleaseAll);
        } else if view_changed {
            mailboxes.mapping.release_for_view_change();
            mailboxes.push_input(HostInputEvent::ReleaseAll);
        } else if mailboxes.mouse_enabled && !mouse {
            mailboxes.resolve_input(HostInputEvent::ReleaseMouse { port: 0 });
        }
        mailboxes.keyboard_enabled = keyboard;
        mailboxes.mouse_enabled = mouse;
        mailboxes.gamepad_enabled = gamepads;
        #[cfg(target_arch = "wasm32")]
        let (interval, generation) = (
            mailboxes.status.input_poll_seconds * 1000.0,
            mailboxes.status.generation,
        );
        drop(mailboxes);
        #[cfg(target_arch = "wasm32")]
        super::set_host_input_enabled(mouse, gamepads, interval, generation);
        true
    }
}
