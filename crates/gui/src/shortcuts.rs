//! One keyboard pipeline: platform ingress, GUI command policy, then guest input.

use crate::host_commands::{HistoryCommand, HostCommand};
use crate::host_input::{HostInputEvent, INPUT_QUEUE_CAPACITY};
use crate::keyboard_input::{with_inbox, KeyCommand, KeyRoute, KeyboardPolicy};
use crate::{EmulatorApp, ViewMode};

const ZOOM_STEP: f32 = 0.25;
const MIN_ZOOM: f32 = 0.75;
const MAX_ZOOM: f32 = 2.5;

impl EmulatorApp {
    pub(crate) fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        self.refresh_keyboard_policy(ctx);
        let decisions = with_inbox(ctx, |inbox| inbox.take());
        for decision in decisions {
            let generation = decision.generation;
            match decision.route {
                KeyRoute::Command { action, pressed } => {
                    if pressed && !self.player.show_saves {
                        self.apply_key_command(action, ctx);
                    }
                }
                KeyRoute::Amiga { code, pressed } if !self.player.show_saves => {
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
        let dialog = self.player.show_saves
            || self.load_binary_modal_open
            || self.file_error.is_some()
            || self.file_picker_open()
            || ctx.memory(|memory| {
                // egui reports the previous pass's layer. The chooser's state
                // determines ingress ownership on the opening and closing passes.
                memory.top_modal_layer().is_some_and(|layer| {
                    layer.id != egui::Id::new(crate::player_view::SAVED_PROGRESS_DIALOG)
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
            bindings: self
                .player
                .active_configuration
                .as_ref()
                .map(|c| c.controls.host.frontend.clone())
                .unwrap_or_default(),
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

    fn apply_key_command(&mut self, action: KeyCommand, ctx: &egui::Context) {
        use KeyCommand::*;
        if matches!(action, Pause | QuickLoad | LoadBinary | LoadState) {
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
            QuickSave => self.save_player(Some(1)),
            QuickLoad => {
                self.player.captured = false;
                crate::layout::top_menu_bar::quick_load_slot(self, 1);
            }
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
