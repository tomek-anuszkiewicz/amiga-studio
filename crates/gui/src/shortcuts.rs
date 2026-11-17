//! One keyboard pipeline: platform ingress, GUI command policy, then guest input.

use crate::host_commands::{HistoryCommand, HostCommand};
use crate::host_input::{HostInputEvent, INPUT_QUEUE_CAPACITY};
use crate::keyboard_input::{
    KeyCommand, KeyRoute, KeyboardPolicy, PhysicalKey, WindowKey, WINDOW_KEYS_ID,
};
use crate::{EmulatorApp, ViewMode};

const ZOOM_STEP: f32 = 0.25;
const MIN_ZOOM: f32 = 0.75;
const MAX_ZOOM: f32 = 2.5;

impl EmulatorApp {
    pub(crate) fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        self.refresh_keyboard_policy(ctx);
        let decisions = ctx.data_mut(|data| {
            let raw = data
                .remove_temp::<Vec<WindowKey>>(egui::Id::new(WINDOW_KEYS_ID))
                .unwrap_or_default();
            let inbox = data.get_temp_mut_or_default::<crate::keyboard_input::KeyboardInbox>(
                egui::Id::new(crate::keyboard_input::KEYBOARD_INBOX_ID),
            );
            for key in raw {
                let (key, observed_generation) = PhysicalKey::from_window(key);
                inbox.receive(key, observed_generation);
            }
            inbox.take()
        });
        for decision in decisions {
            let generation = decision.generation;
            match decision.route {
                KeyRoute::Command { action, pressed } => {
                    if pressed {
                        self.apply_key_command(action, ctx);
                    }
                }
                KeyRoute::Amiga { code, pressed } => {
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
                KeyRoute::Rejected(_) => {}
            }
        }
        if !ctx.input(|i| i.focused) {
            self.host.keyboard_pending.clear();
        }
    }

    pub(crate) fn refresh_keyboard_policy(&self, ctx: &egui::Context) {
        let screen_focus = ctx.memory(|memory| {
            memory.has_focus(egui::Id::new(
                crate::layout::main_viewport::amiga_screen::SCREEN_INPUT_ID,
            ))
        });
        let text_focus = ctx
            .memory(|memory| memory.focused())
            .is_some_and(|id| egui::TextEdit::load_state(ctx, id).is_some())
            && !screen_focus;
        let policy = KeyboardPolicy {
            focused: ctx.input(|i| i.focused),
            text_focus,
            session: self.player.screen == crate::player::PlayerScreen::Session,
            pending_launch: self.player.pending_launch.is_some(),
            captured: self.player.captured,
            running: self.host.status.running,
            pause_pending: self.host.status.pause_pending,
            developer: self.view_mode == ViewMode::Developer,
            can_edit: self.host.can_edit(),
            bindings: self
                .player
                .active_configuration
                .as_ref()
                .map(|c| c.controls.host.frontend.clone())
                .unwrap_or_default(),
        };
        ctx.data_mut(|data| {
            data.insert_temp(
                egui::Id::new(crate::keyboard_input::WINDOW_KEY_GENERATION_ID),
                self.host.status.generation,
            );
            data.get_temp_mut_or_default::<crate::keyboard_input::KeyboardInbox>(egui::Id::new(
                crate::keyboard_input::KEYBOARD_INBOX_ID,
            ))
            .set_policy(policy, self.host.status.generation);
        });
    }

    fn apply_key_command(&mut self, action: KeyCommand, ctx: &egui::Context) {
        use KeyCommand::*;
        if matches!(
            action,
            Release | ToggleView | Pause | QuickLoad | LoadBinary | LoadState | SaveState
        ) {
            self.host.keyboard_pending.clear();
        }
        match action {
            Release => self.player.captured = false,
            ToggleView => {
                self.player.captured = false;
                self.set_view_mode(if self.view_mode == ViewMode::ScreenOnly {
                    ViewMode::Developer
                } else {
                    ViewMode::ScreenOnly
                });
            }
            Pause => {
                self.player.captured = false;
                self.toggle_run();
            }
            QuickSave => self.save_player(Some(1)),
            QuickLoad => crate::layout::top_menu_bar::quick_load_slot(self, 1),
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
