//! Shared physical keyboard policy for window and browser events.

use crate::host_keys::{amiga_key_code, frontend_key_code};
use crate::player_config::FrontendBindings;
use std::collections::{HashMap, HashSet};

/// Physical window events carry the session generation observed at receipt.
pub(crate) type WindowKey = (String, String, bool, bool, bool, u64);
pub(crate) const WINDOW_KEYS_ID: &str = "window_keyboard_events";
pub(crate) const WINDOW_KEY_GENERATION_ID: &str = "window_keyboard_generation";

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PhysicalKey {
    pub physical: String,
    pub logical: String,
    pub pressed: bool,
    pub repeat: bool,
    pub synthetic: bool,
}

impl PhysicalKey {
    pub(crate) fn from_window(
        (physical, logical, pressed, repeat, synthetic, generation): WindowKey,
    ) -> (Self, u64) {
        (
            Self {
                physical: match physical.as_str() {
                    "SuperLeft" => "MetaLeft".into(),
                    "SuperRight" => "MetaRight".into(),
                    _ => physical,
                },
                logical,
                pressed,
                repeat,
                synthetic,
            },
            generation,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCommand {
    Release,
    ToggleView,
    Pause,
    QuickSave,
    QuickLoad,
    StepInstruction,
    HistoryBack,
    StepCck,
    StepField,
    Microcode,
    Recording,
    LoadBinary,
    SaveState,
    LoadState,
    ZoomIn,
    ZoomOut,
    NormalZoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyRoute {
    Command { action: KeyCommand, pressed: bool },
    Amiga { code: u8, pressed: bool },
    Rejected(&'static str),
}

#[derive(Debug, Clone)]
pub struct KeyboardPolicy {
    pub focused: bool,
    pub text_focus: bool,
    pub session: bool,
    pub pending_launch: bool,
    pub captured: bool,
    pub running: bool,
    pub pause_pending: bool,
    pub developer: bool,
    pub can_edit: bool,
    pub bindings: FrontendBindings,
}

#[derive(Debug, Default, Clone)]
pub struct KeyboardRouter {
    held: HashSet<String>,
    commands: HashMap<String, KeyCommand>,
}

#[derive(Debug, Clone)]
pub(crate) struct KeyboardDecision {
    pub route: KeyRoute,
    pub generation: u64,
}

/// GUI-owned routing state; browser callbacks can decide default handling immediately.
#[derive(Debug, Default, Clone)]
pub(crate) struct KeyboardInbox {
    router: KeyboardRouter,
    policy: Option<KeyboardPolicy>,
    generation: u64,
    pending: Vec<KeyboardDecision>,
}

pub(crate) const KEYBOARD_INBOX_ID: &str = "keyboard_routing_inbox";

impl KeyboardInbox {
    pub fn set_policy(&mut self, policy: KeyboardPolicy, generation: u64) {
        if self.generation != generation {
            self.router.release();
            self.pending.clear();
            self.generation = generation;
        }
        if !policy.focused {
            self.router.release();
        }
        self.policy = Some(policy);
    }

    pub fn receive(&mut self, key: PhysicalKey, generation: u64) -> bool {
        let Some(policy) = &mut self.policy else {
            return false;
        };
        if generation != self.generation {
            return false;
        }
        let route = if matches!(key.physical.as_str(), "__focus_lost__" | "__overflow__") {
            self.router.release();
            policy.captured = false;
            KeyRoute::Command {
                action: KeyCommand::Release,
                pressed: true,
            }
        } else {
            self.router.route(&key, policy)
        };
        let prevent_default = !matches!(route, KeyRoute::Rejected(_))
            || (policy.focused && policy.captured && !policy.text_focus);
        if matches!(
            route,
            KeyRoute::Command {
                action: KeyCommand::Release
                    | KeyCommand::ToggleView
                    | KeyCommand::Pause
                    | KeyCommand::QuickLoad
                    | KeyCommand::LoadBinary
                    | KeyCommand::LoadState
                    | KeyCommand::SaveState,
                pressed: true
            }
        ) {
            policy.captured = false;
        }
        if self.pending.len() >= crate::host_input::INPUT_QUEUE_CAPACITY {
            self.pending.clear();
            self.router.release();
            self.pending.push(KeyboardDecision {
                route: KeyRoute::Command {
                    action: KeyCommand::Release,
                    pressed: true,
                },
                generation,
            });
        } else {
            self.pending.push(KeyboardDecision { route, generation });
        }
        prevent_default
    }

    pub fn take(&mut self) -> Vec<KeyboardDecision> {
        std::mem::take(&mut self.pending)
    }
}

impl KeyboardRouter {
    pub fn release(&mut self) {
        self.held.clear();
        self.commands.clear();
    }

    pub fn route(&mut self, key: &PhysicalKey, policy: &KeyboardPolicy) -> KeyRoute {
        if key.synthetic && key.pressed {
            return KeyRoute::Rejected("synthetic_focus_press");
        }
        let changed = if key.pressed {
            self.held.insert(key.physical.clone())
        } else {
            self.held.remove(&key.physical)
        };
        if !key.pressed {
            if let Some(action) = self.commands.remove(&key.physical) {
                return KeyRoute::Command {
                    action,
                    pressed: false,
                };
            }
        }
        if !policy.focused {
            return KeyRoute::Rejected("window_inactive");
        }
        if !policy.session {
            return KeyRoute::Rejected("not_session_screen");
        }
        if policy.pending_launch {
            return KeyRoute::Rejected("pending_launch");
        }
        if policy.text_focus {
            return KeyRoute::Rejected("gui_text_focus");
        }
        if key.repeat || !changed {
            return KeyRoute::Rejected("duplicate_edge_or_repeat");
        }
        if key.pressed {
            if let Some(action) = self.command(&key.physical, policy) {
                self.commands.insert(key.physical.clone(), action);
                return KeyRoute::Command {
                    action,
                    pressed: true,
                };
            }
        }
        if !policy.captured {
            return KeyRoute::Rejected("capture_disabled");
        }
        if !policy.running {
            return KeyRoute::Rejected("machine_not_running");
        }
        if policy.pause_pending {
            return KeyRoute::Rejected("pause_pending");
        }
        let Some(code) = amiga_key_code(&key.physical) else {
            return KeyRoute::Rejected("unmapped_physical_key");
        };
        let pressed = if matches!(key.physical.as_str(), "ControlLeft" | "ControlRight") {
            self.held.contains("ControlLeft") || self.held.contains("ControlRight")
        } else {
            key.pressed
        };
        KeyRoute::Amiga { code, pressed }
    }

    fn command(&self, physical: &str, policy: &KeyboardPolicy) -> Option<KeyCommand> {
        use KeyCommand::*;
        if physical == "F12" {
            return Some(ToggleView);
        }
        let command = ["ControlLeft", "ControlRight", "MetaLeft", "MetaRight"]
            .into_iter()
            .any(|key| self.held.contains(key));
        if command {
            let action = match physical {
                "KeyO" => Some(LoadBinary),
                "KeyS" => Some(SaveState),
                "KeyL" => Some(LoadState),
                "Equal" | "NumpadAdd" => Some(ZoomIn),
                "Minus" | "NumpadSubtract" => Some(ZoomOut),
                "Digit0" | "Numpad0" => Some(NormalZoom),
                _ => None,
            };
            if action.is_some() {
                return action;
            }
        }
        for (binding, action) in [
            (&policy.bindings.release, Release),
            (&policy.bindings.pause, Pause),
            (&policy.bindings.save, QuickSave),
            (&policy.bindings.load, QuickLoad),
        ] {
            if amiga_key_code(physical).is_some_and(|code| frontend_key_code(binding) == Some(code))
            {
                return Some(action);
            }
        }
        if policy.developer && !policy.running && policy.can_edit {
            let shift = self.held.contains("ShiftLeft") || self.held.contains("ShiftRight");
            return match physical {
                "F10" => Some(if shift { HistoryBack } else { StepInstruction }),
                "F11" => Some(if shift { StepField } else { StepCck }),
                "F8" => Some(Microcode),
                "KeyT" if self.held.contains("AltLeft") || self.held.contains("AltRight") => {
                    Some(Recording)
                }
                _ => None,
            };
        }
        None
    }
}
