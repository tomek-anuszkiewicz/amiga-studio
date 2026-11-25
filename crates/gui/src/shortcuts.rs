//! One keyboard pipeline: platform ingress, GUI command policy, then guest input.

use crate::host_commands::{HistoryCommand, HostCommand};
use crate::host_input::{HostInputEvent, INPUT_QUEUE_CAPACITY};
use crate::host_keys::{frontend_binding_name, frontend_key_position, host_key_name};
use crate::keyboard_input::{with_inbox, KeyCommand, KeyRoute, KeyboardPolicy};
use crate::player_config::FrontendAction;
use crate::{EmulatorApp, ViewMode};

impl From<FrontendAction> for KeyCommand {
    fn from(action: FrontendAction) -> Self {
        match action {
            FrontendAction::Release => Self::Release,
            FrontendAction::Pause => Self::Pause,
            FrontendAction::Save => Self::QuickSave,
            FrontendAction::Load => Self::QuickLoad,
            FrontendAction::Debug => Self::ToggleView,
        }
    }
}

#[derive(Clone, Copy)]
pub enum ShortcutModifier {
    None,
    Command,
    Debugger,
    DebuggerUnshifted,
    DebuggerShift,
    DebuggerAlt,
}

pub struct FixedShortcut {
    pub(crate) action: KeyCommand,
    pub(crate) position: &'static str,
    pub(crate) modifier: ShortcutModifier,
}

impl FixedShortcut {
    const fn new(action: KeyCommand, position: &'static str, modifier: ShortcutModifier) -> Self {
        Self {
            action,
            position,
            modifier,
        }
    }
}

pub const FIXED_SHORTCUTS: &[FixedShortcut] = {
    use crate::keyboard_input::VIEW_SWITCH_KEY;
    use KeyCommand::*;
    use ShortcutModifier::*;
    &[
        FixedShortcut::new(ToggleView, VIEW_SWITCH_KEY, None),
        FixedShortcut::new(LoadBinary, "KeyO", Command),
        FixedShortcut::new(SaveState, "KeyS", Command),
        FixedShortcut::new(LoadState, "KeyL", Command),
        FixedShortcut::new(ZoomIn, "Equal", Command),
        FixedShortcut::new(ZoomIn, "NumpadAdd", Command),
        FixedShortcut::new(ZoomOut, "Minus", Command),
        FixedShortcut::new(ZoomOut, "NumpadSubtract", Command),
        FixedShortcut::new(NormalZoom, "Digit0", Command),
        FixedShortcut::new(NormalZoom, "Numpad0", Command),
        FixedShortcut::new(StepInstruction, "F10", DebuggerUnshifted),
        FixedShortcut::new(HistoryBack, "F10", DebuggerShift),
        FixedShortcut::new(StepCck, "F11", DebuggerUnshifted),
        FixedShortcut::new(StepField, "F11", DebuggerShift),
        FixedShortcut::new(Microcode, "F8", Debugger),
        FixedShortcut::new(Recording, "KeyT", DebuggerAlt),
    ]
};

const ZOOM_STEP: f32 = 0.25;
const MIN_ZOOM: f32 = 0.75;
const MAX_ZOOM: f32 = 2.5;

impl EmulatorApp {
    pub(crate) fn execution_help(&self) -> &'static str {
        if self.host.status.running {
            "Pause after the current video field completes."
        } else {
            "Resume the same machine and mounted disks at the selected speed."
        }
    }

    pub(crate) fn shortcut_tooltip(
        &self,
        response: egui::Response,
        action: KeyCommand,
        description: &'static str,
    ) -> egui::Response {
        response
            .on_hover_ui(|ui| {
                ui.label(description);
                let keys = self.shortcut_names(action);
                if !keys.is_empty() {
                    ui.label(format!("Keyboard: {}", keys.join(", ")));
                }
            })
            .on_disabled_hover_text(description)
    }

    fn shortcut_names(&self, action: KeyCommand) -> Vec<String> {
        let bindings = self.active_frontend_bindings();
        let debugger_shortcuts = self.view_mode == ViewMode::Developer
            && !self.host.status.running
            && self.host.can_edit();
        let mut names = Vec::new();
        for fixed in FIXED_SHORTCUTS
            .iter()
            .filter(|fixed| fixed.action == action)
        {
            let key = host_key_name(fixed.position);
            match fixed.modifier {
                ShortcutModifier::None => names.push(key),
                ShortcutModifier::Command => {
                    names.extend([format!("Ctrl+{key}"), format!("Command+{key}")]);
                }
                modifier if debugger_shortcuts => {
                    if FrontendAction::EDITABLE_KEYS.into_iter().any(|action| {
                        frontend_key_position(bindings.binding(action)) == Some(fixed.position)
                    }) {
                        continue;
                    }
                    names.push(match modifier {
                        ShortcutModifier::DebuggerShift => format!("Shift+{key}"),
                        ShortcutModifier::DebuggerAlt => format!("Alt+{key}"),
                        _ => key,
                    });
                }
                _ => {}
            }
        }
        if let Some(frontend) = FrontendAction::EDITABLE_KEYS
            .into_iter()
            .find(|frontend| KeyCommand::from(*frontend) == action)
        {
            let binding = bindings.binding(frontend);
            if frontend_key_position(binding).is_some() {
                names.push(frontend_binding_name(binding));
            }
        }
        names
    }

    fn active_frontend_bindings(&self) -> crate::player_config::FrontendBindings {
        self.player
            .active_configuration
            .as_ref()
            .map(|configuration| configuration.controls.host.frontend.clone())
            .unwrap_or_default()
    }

    pub(crate) fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        self.refresh_keyboard_policy(ctx);
        let decisions = with_inbox(ctx, |inbox| inbox.take());
        for decision in decisions {
            let generation = decision.generation;
            match decision.route {
                KeyRoute::Command { action, pressed } => {
                    if pressed && !self.editing_save_description() {
                        self.apply_key_command(action, ctx);
                    }
                }
                KeyRoute::Amiga { code, pressed } if !self.editing_save_description() => {
                    if self.host.keyboard_pending.len() == INPUT_QUEUE_CAPACITY {
                        self.host.keyboard_pending.clear();
                        self.host
                            .keyboard_pending
                            .push_back((HostInputEvent::ReleaseAll, generation));
                    } else {
                        self.host
                            .keyboard_pending
                            .push_back((HostInputEvent::Key { code, pressed }, generation));
                    }
                }
                KeyRoute::ReleaseAll => {
                    self.player.captured = false;
                    self.host.keyboard_pending.clear();
                    self.host
                        .keyboard_pending
                        .push_back((HostInputEvent::ReleaseAll, generation));
                }
                KeyRoute::Amiga { .. } | KeyRoute::Consumed | KeyRoute::Rejected(_) => {}
            }
        }
    }

    pub(crate) fn keyboard_policy(&self, ctx: &egui::Context) -> KeyboardPolicy {
        let focused_text = ctx
            .memory(|memory| memory.focused())
            .is_some_and(|id| egui::TextEdit::load_state(ctx, id).is_some());
        let dialog = self.editing_save_description()
            || self.load_binary_modal_open
            || self.file_error.is_some()
            || self.file_picker_open()
            || ctx.memory(|memory| {
                // egui reports the previous pass's layer. The editor's state
                // determines ingress ownership on the opening and closing passes.
                memory.top_modal_layer().is_some_and(|layer| {
                    layer.id != egui::Id::unique(crate::player_view::SAVE_DESCRIPTION_DIALOG)
                })
            });
        #[cfg(not(target_arch = "wasm32"))]
        let focused = ctx.input(|input| input.focused);
        #[cfg(target_arch = "wasm32")]
        let focused = crate::host_runtime::host_window_active();
        KeyboardPolicy {
            focused,
            gui_owns_input: focused_text || dialog,
            session: self.player.screen == crate::player::PlayerScreen::Session
                && self.host.error.is_none()
                && !self.host.status.shutdown,
            pending_launch: self.player.pending_launch.is_some() || {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.player.missing_configuration.is_some()
                }
                #[cfg(target_arch = "wasm32")]
                {
                    false
                }
            },
            running: self.host.status.running,
            pause_pending: self.host.pause_requested(),
            developer: self.view_mode == ViewMode::Developer,
            can_edit: self.host.can_edit(),
            bindings: self.active_frontend_bindings(),
        }
    }

    fn file_picker_open(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            self.browser_file_task.is_some()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.player.dialog.is_some()
        }
    }

    fn refresh_keyboard_policy(&self, ctx: &egui::Context) {
        let policy = self.keyboard_policy(ctx);
        with_inbox(ctx, |inbox| {
            inbox.set_policy(policy, self.host.status.generation)
        });
    }

    pub(crate) fn apply_key_command(&mut self, action: KeyCommand, ctx: &egui::Context) {
        use KeyCommand::*;
        if matches!(action, Pause | LoadBinary | LoadState) {
            self.host.keyboard_pending.clear();
        }
        match action {
            Release => self.player.captured = false,
            ToggleView => self.set_view_mode(if self.view_mode == ViewMode::ScreenOnly {
                ViewMode::Developer
            } else {
                ViewMode::ScreenOnly
            }),
            Pause => {
                self.player.captured = false;
                self.toggle_run();
            }
            QuickSave => self.quick_save(),
            QuickLoad => self.quick_load(),
            StepInstruction => {
                self.disassembly_view_addr = None;
                self.host.queue(HostCommand::StepInstruction);
            }
            HistoryBack => {
                self.disassembly_view_addr = None;
                self.host
                    .queue(HostCommand::History(HistoryCommand::Backward));
            }
            StepCck => {
                self.disassembly_view_addr = None;
                self.host.queue(HostCommand::StepCck);
            }
            StepField => {
                self.disassembly_view_addr = None;
                self.host.queue(HostCommand::StepField);
            }
            Microcode => self.show_microcode = !self.show_microcode,
            Recording => self
                .host
                .queue(HostCommand::History(HistoryCommand::ToggleRecording)),
            LoadBinary => self.open_load_binary_dialog(),
            SaveState => self.open_save_state_dialog(),
            LoadState => self.open_load_state_dialog(),
            ZoomIn => ctx.set_zoom_factor((ctx.zoom_factor() + ZOOM_STEP).min(MAX_ZOOM)),
            ZoomOut => ctx.set_zoom_factor((ctx.zoom_factor() - ZOOM_STEP).max(MIN_ZOOM)),
            NormalZoom => ctx.set_zoom_factor(1.0),
        }
    }
}
