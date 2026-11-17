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
    pub(super) fn browser_key(&self, json: &str, generation: u64) -> bool {
        let Ok(key) = serde_json::from_str::<crate::keyboard_input::PhysicalKey>(json) else {
            return false;
        };
        let prevent_default = self.repaint.data_mut(|data| {
            data.get_temp_mut_or_default::<crate::keyboard_input::KeyboardInbox>(egui::Id::new(
                crate::keyboard_input::KEYBOARD_INBOX_ID,
            ))
            .receive(key, generation)
        });
        self.repaint.request_repaint();
        prevent_default
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
        if generation == mailboxes.status.generation
            && (mailboxes.input_active()
                || matches!(
                    event,
                    HostInputEvent::ReleaseAll | HostInputEvent::ReleaseCapture { .. }
                ))
        {
            mailboxes.resolve_input(event);
        } else if retry_inactive && generation == mailboxes.status.generation {
            return false;
        }
        true
    }

    pub fn set_enabled(&self, captured: bool, gamepads: bool) -> bool {
        let Ok(mut mailboxes) = self.shared.mailboxes.try_lock() else {
            return false;
        };
        if mailboxes.gamepad_enabled != gamepads {
            mailboxes.gamepad_enabled = gamepads;
            mailboxes.mapping.release();
            #[cfg(not(target_arch = "wasm32"))]
            {
                mailboxes.gamepads_started_at = std::time::SystemTime::now();
            }
            mailboxes.push_input(HostInputEvent::ReleaseAll);
        }
        if mailboxes.input_enabled != captured {
            mailboxes.input_enabled = captured;
            let release = mailboxes.mapping.release_capture();
            mailboxes.push_input(release);
        }
        #[cfg(target_arch = "wasm32")]
        super::set_host_input_enabled(
            captured,
            gamepads,
            mailboxes.status.input_poll_seconds * 1000.0,
            mailboxes.status.generation,
        );
        true
    }
}
