//! GUI-side input activation, configuration and browser ingress.

use super::{HostInputEvent, HostInputSender};
use crate::host_input::KeyboardEvent;
use crate::input_mapping::{GamepadCapture, GamepadRecorder, GamepadRecording};

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

    pub(crate) fn take_gamepad_recording(
        &self,
        request: &GamepadRecording,
    ) -> Option<GamepadCapture> {
        let Ok(mut mailboxes) = self.shared.mailboxes.try_lock() else {
            self.repaint.request_repaint_after(super::HOST_RETRY_DELAY);
            return None;
        };
        let recorder = mailboxes
            .gamepad_recorder
            .as_mut()
            .filter(|recorder| &recorder.request == request)?;
        recorder.captured.take()
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

    pub(crate) fn take_frontend_actions(&self) -> Vec<crate::player_config::FrontendAction> {
        self.shared
            .mailboxes
            .try_lock()
            .ok()
            .map(|mut mailboxes| std::mem::take(&mut mailboxes.mapping.actions))
            .unwrap_or_default()
    }

    pub fn try_host_event(&self, event: HostInputEvent, generation: u64) -> bool {
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
        true
    }

    /// GUI-routed keys wait for owner activation instead of being discarded on stale telemetry.
    pub(crate) fn try_keyboard_event(&self, event: &KeyboardEvent, generation: u64) -> bool {
        let Ok(mut mailboxes) = self.shared.mailboxes.try_lock() else {
            return false;
        };
        if generation != mailboxes.status.generation {
            return true;
        }
        match event {
            KeyboardEvent::Key { position, pressed } => {
                if !mailboxes.keyboard_enabled || !mailboxes.gameplay_active() {
                    return false;
                }
                let events = mailboxes
                    .mapping
                    .keyboard(position.clone().into(), *pressed);
                for event in events {
                    mailboxes.push_input(event);
                }
            }
            KeyboardEvent::ReleaseAll => mailboxes.resolve_input(HostInputEvent::ReleaseAll),
        }
        true
    }

    pub fn set_enabled(&self, keyboard: bool, mouse: bool, gamepads: bool) -> bool {
        self.apply_input_activation(keyboard, mouse, gamepads, None, None, None)
    }

    pub(crate) fn apply_input_activation(
        &self,
        keyboard: bool,
        mouse: bool,
        gamepads: bool,
        frontend: Option<bool>,
        view_release_generation: Option<u64>,
        gamepad_recording: Option<&GamepadRecording>,
    ) -> bool {
        let Ok(mut mailboxes) = self.shared.mailboxes.try_lock() else {
            return false;
        };
        match gamepad_recording {
            Some(request)
                if !mailboxes
                    .gamepad_recorder
                    .as_ref()
                    .is_some_and(|recorder| &recorder.request == request) =>
            {
                mailboxes.gamepad_recorder = Some(GamepadRecorder::new(
                    request.clone(),
                    &mailboxes.gamepad_samples,
                ));
            }
            None => mailboxes.gamepad_recorder = None,
            _ => {}
        }
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
        if let Some(frontend) = frontend {
            mailboxes.frontend_enabled = frontend;
            if !frontend {
                mailboxes.mapping.actions.clear();
            }
        }
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
