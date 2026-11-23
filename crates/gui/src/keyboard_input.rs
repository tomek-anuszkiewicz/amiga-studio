//! Shared physical keyboard policy for window and browser events.

use crate::host_keys::{amiga_key_code, frontend_key_position};
use crate::player_config::FrontendBindings;
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PhysicalKey {
    pub physical: String,
    pub logical: String,
    pub pressed: bool,
    pub repeat: bool,
    pub synthetic: bool,
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
    Consumed,
    ReleaseAll,
    Rejected(&'static str),
}

#[derive(Debug, Clone)]
pub struct KeyboardPolicy {
    pub focused: bool,
    pub gui_owns_input: bool,
    pub session: bool,
    pub pending_launch: bool,
    pub running: bool,
    pub pause_pending: bool,
    pub developer: bool,
    pub can_edit: bool,
    pub bindings: FrontendBindings,
}

impl KeyboardPolicy {
    pub(crate) fn frontend_active(&self) -> bool {
        self.focused && self.session && !self.pending_launch && !self.gui_owns_input
    }

    pub(crate) fn gameplay_active(&self) -> bool {
        self.frontend_active() && self.running && !self.pause_pending
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeldKey {
    Available,
    Guest,
    Command(KeyCommand),
    Blocked,
}

#[derive(Debug, Default, Clone)]
pub struct KeyboardRouter {
    held: HashMap<String, HeldKey>,
}

#[derive(Debug, Clone)]
pub(crate) struct KeyboardDecision {
    pub route: KeyRoute,
    pub generation: u64,
}

/// Both platforms use this inbox before sending a key to their egui adapter.
#[derive(Debug, Default, Clone)]
pub(crate) struct KeyboardInbox {
    router: KeyboardRouter,
    policy: Option<KeyboardPolicy>,
    generation: u64,
    pending: Vec<KeyboardDecision>,
}

const KEYBOARD_INBOX_ID: &str = "keyboard_routing_inbox";

pub(crate) fn with_inbox<T>(ctx: &egui::Context, f: impl FnOnce(&mut KeyboardInbox) -> T) -> T {
    ctx.data_mut(|data| {
        f(data.get_temp_mut_or_default::<KeyboardInbox>(egui::Id::new(KEYBOARD_INBOX_ID)))
    })
}

/// Returns whether the key belongs to gameplay or an application command.
pub fn receive_key(ctx: &egui::Context, mut key: PhysicalKey, generation: Option<u64>) -> bool {
    match key.physical.as_str() {
        "SuperLeft" => key.physical = "MetaLeft".into(),
        "SuperRight" => key.physical = "MetaRight".into(),
        _ => {}
    }
    with_inbox(ctx, |inbox| inbox.receive(key, generation))
}

/// Updates ownership immediately, including between application frames.
pub fn receive_focus(ctx: &egui::Context, focused: bool, generation: Option<u64>) {
    with_inbox(ctx, |inbox| inbox.set_focus(focused, generation));
}

const TEXT_EDITORS_ID: &str = "session_text_editors";

pub(crate) fn begin_editor_collection(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove_temp::<Vec<EditorRegion>>(egui::Id::new(TEXT_EDITORS_ID)));
}

#[derive(Clone)]
struct EditorRegion {
    rect: egui::Rect,
    layer: egui::LayerId,
}

pub(crate) fn text_edit(ui: &mut egui::Ui, edit: egui::TextEdit<'_>) -> egui::Response {
    let response = ui.add(edit);
    if response.enabled() && response.interact_rect.is_positive() {
        ui.ctx().data_mut(|data| {
            data.get_temp_mut_or_default::<Vec<EditorRegion>>(egui::Id::new(TEXT_EDITORS_ID))
                .push(EditorRegion {
                    rect: response.interact_rect,
                    layer: response.layer_id,
                });
        });
    }
    response
}

/// Give a visible editor the first key even when the click and key precede the next frame.
pub fn receive_pointer_press(ctx: &egui::Context, position: egui::Pos2) {
    let layer = ctx.layer_id_at(position);
    let editor = ctx.data(|data| {
        data.get_temp::<Vec<EditorRegion>>(egui::Id::new(TEXT_EDITORS_ID))
            .is_some_and(|editors| {
                editors
                    .iter()
                    .any(|editor| Some(editor.layer) == layer && editor.rect.contains(position))
            })
    });
    if editor {
        with_inbox(ctx, |inbox| {
            if let Some(policy) = &mut inbox.policy {
                policy.gui_owns_input = true;
                inbox.router.suspend();
                inbox
                    .pending
                    .retain(|decision| !matches!(decision.route, KeyRoute::Amiga { .. }));
            }
        });
    }
}

impl KeyboardInbox {
    pub fn set_policy(&mut self, mut policy: KeyboardPolicy, generation: u64) {
        if self.generation != generation {
            self.router.suspend();
            self.pending.clear();
            self.generation = generation;
        }
        if let Some(previous) = &self.policy {
            // At ingress F12 already changed routing for the rest of the batch.
            // Keep that view until the GUI processes the queued command.
            if previous.developer != policy.developer
                && self.pending.iter().any(|decision| {
                    matches!(
                        decision.route,
                        KeyRoute::Command {
                            action: KeyCommand::ToggleView,
                            pressed: true
                        }
                    )
                })
            {
                policy.developer = previous.developer;
            }
            if previous.focused && !policy.focused {
                self.set_focus(false, None);
            } else if previous.developer != policy.developer
                || (previous.frontend_active() && !policy.frontend_active())
                || (previous.gameplay_active() && !policy.gameplay_active())
            {
                self.router.suspend();
                self.pending
                    .retain(|decision| !matches!(decision.route, KeyRoute::Amiga { .. }));
            }
        }
        self.policy = Some(policy);
    }

    fn set_focus(&mut self, focused: bool, generation: Option<u64>) {
        if generation.is_some_and(|generation| generation != self.generation) {
            return;
        }
        if let Some(policy) = &mut self.policy {
            if policy.focused == focused {
                return;
            }
            policy.focused = focused;
            // The platform may never report releases outside our window. Repeats
            // and synthesized presses cannot establish fresh gameplay input.
            self.router.release();
            if !focused {
                self.release_pending();
            }
        }
    }

    fn receive(&mut self, key: PhysicalKey, generation: Option<u64>) -> bool {
        if generation.is_some_and(|generation| generation != self.generation) {
            return false;
        }
        let Some(policy) = &mut self.policy else {
            return false;
        };
        let route = self.router.route(&key, policy);
        match route {
            KeyRoute::Consumed => return true,
            KeyRoute::Rejected(_) => return false,
            KeyRoute::Command {
                pressed: true,
                action,
            } => {
                use KeyCommand::*;
                match action {
                    ToggleView => {
                        if !policy.developer {
                            policy.pause_pending = true;
                        }
                        self.router.suspend();
                        policy.developer = !policy.developer;
                    }
                    Pause | QuickLoad => {
                        policy.pause_pending = true;
                        self.router.suspend();
                    }
                    LoadBinary | LoadState => {
                        policy.gui_owns_input = true;
                        self.router.suspend();
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    SaveState => {
                        policy.gui_owns_input = true;
                        self.router.suspend();
                    }
                    _ => {}
                }
            }
            KeyRoute::Command { pressed: false, .. } => return true,
            _ => {}
        }
        if self.pending.len() >= crate::host_input::INPUT_QUEUE_CAPACITY {
            self.router.suspend();
            self.release_pending();
        } else {
            self.pending.push(KeyboardDecision {
                route,
                generation: self.generation,
            });
        }
        true
    }

    fn release_pending(&mut self) {
        self.pending.clear();
        self.pending.push(KeyboardDecision {
            route: KeyRoute::ReleaseAll,
            generation: self.generation,
        });
    }

    pub fn take(&mut self) -> Vec<KeyboardDecision> {
        std::mem::take(&mut self.pending)
    }
}

impl KeyboardRouter {
    pub fn release(&mut self) {
        self.held.clear();
    }

    fn suspend(&mut self) {
        for state in self.held.values_mut() {
            if !matches!(state, HeldKey::Command(_)) {
                *state = HeldKey::Blocked;
            }
        }
    }

    pub fn route(&mut self, key: &PhysicalKey, policy: &KeyboardPolicy) -> KeyRoute {
        if key.synthetic && key.pressed {
            return KeyRoute::Rejected("synthetic_focus_press");
        }
        let code = amiga_key_code(&key.physical);
        let previous = if key.pressed {
            self.held.get(&key.physical).copied()
        } else {
            self.held.remove(&key.physical)
        };
        if let Some(HeldKey::Command(action)) = previous {
            return if key.pressed {
                KeyRoute::Consumed
            } else {
                KeyRoute::Command {
                    action,
                    pressed: false,
                }
            };
        }
        if !policy.frontend_active() {
            if key.pressed && !key.repeat {
                self.held.insert(key.physical.clone(), HeldKey::Blocked);
            }
            return KeyRoute::Rejected(if !policy.focused {
                "window_inactive"
            } else if !policy.session {
                "not_session_screen"
            } else if policy.pending_launch {
                "pending_launch"
            } else {
                "gui_owns_input"
            });
        }
        if key.repeat || (key.pressed && previous.is_some()) {
            return if policy.gameplay_active() && code.is_some() {
                KeyRoute::Consumed
            } else {
                KeyRoute::Rejected("duplicate_edge_or_repeat")
            };
        }
        if key.pressed {
            if let Some(action) = self.command(&key.physical, policy) {
                self.held
                    .insert(key.physical.clone(), HeldKey::Command(action));
                return KeyRoute::Command {
                    action,
                    pressed: true,
                };
            }
            let held = if code.is_some() && policy.gameplay_active() {
                HeldKey::Guest
            } else {
                HeldKey::Available
            };
            self.held.insert(key.physical.clone(), held);
        }
        if !policy.running {
            return KeyRoute::Rejected("machine_not_running");
        }
        if policy.pause_pending {
            return KeyRoute::Rejected("pause_pending");
        }
        let Some(code) = code else {
            return KeyRoute::Rejected("unmapped_physical_key");
        };
        if !key.pressed && previous != Some(HeldKey::Guest) {
            return KeyRoute::Consumed;
        }
        let pressed = key.pressed
            || matches!(key.physical.as_str(), "ControlLeft" | "ControlRight")
                && ["ControlLeft", "ControlRight"]
                    .iter()
                    .any(|side| self.held.get(*side) == Some(&HeldKey::Guest));
        KeyRoute::Amiga { code, pressed }
    }

    fn held(&self, physical: &str) -> bool {
        self.held
            .get(physical)
            .is_some_and(|key| *key != HeldKey::Blocked)
    }

    fn command(&self, physical: &str, policy: &KeyboardPolicy) -> Option<KeyCommand> {
        use KeyCommand::*;
        if physical == "F12" {
            return Some(ToggleView);
        }
        let command = ["ControlLeft", "ControlRight", "MetaLeft", "MetaRight"]
            .into_iter()
            .any(|key| self.held(key));
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
            if frontend_key_position(binding) == Some(physical) {
                return Some(action);
            }
        }
        if policy.developer && !policy.running && policy.can_edit {
            let shift = self.held("ShiftLeft") || self.held("ShiftRight");
            return match physical {
                "F10" => Some(if shift { HistoryBack } else { StepInstruction }),
                "F11" => Some(if shift { StepField } else { StepCck }),
                "F8" => Some(Microcode),
                "KeyT" if self.held("AltLeft") || self.held("AltRight") => Some(Recording),
                _ => None,
            };
        }
        None
    }
}
