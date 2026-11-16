//! Global application shortcuts, sharing execution policy with the toolbar.

use crate::host_commands::{HistoryCommand, HostCommand};
use crate::{EmulatorApp, ViewMode};

const ZOOM_STEP: f32 = 0.25;
const MIN_ZOOM: f32 = 0.75;
const MAX_ZOOM: f32 = 2.5;

impl EmulatorApp {
    /// Handles global keyboard shortcuts
    pub(crate) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let input = ctx.input(|i| i.clone());

        let screen_focus = ctx.memory(|memory| {
            memory.has_focus(egui::Id::new(
                crate::layout::main_viewport::amiga_screen::SCREEN_INPUT_ID,
            ))
        });
        if self.player.screen != crate::player::PlayerScreen::Session {
            return;
        }
        let host_text_focus = ctx
            .memory(|memory| memory.focused())
            .is_some_and(|id| egui::TextEdit::load_state(ctx, id).is_some());
        if (host_text_focus && !screen_focus) || self.player.pending_launch.is_some() {
            return;
        }
        let bindings = self
            .player
            .active_configuration
            .as_ref()
            .map(|c| c.frontend.clone())
            .unwrap_or_default();
        let pressed = |name: &str| {
            crate::host_keys::frontend_key(name).is_some_and(|key| {
                input.events.iter().any(|event| matches!(event, egui::Event::Key {key: logical, physical_key, pressed: true, repeat: false, ..} if physical_key.unwrap_or(*logical) == key))
                && (!ctx.wants_keyboard_input() || screen_focus || key.name().starts_with('F') || key == egui::Key::Escape)
            })
        };
        if pressed(&bindings.release) {
            self.player.captured = false;
        }
        if pressed(&bindings.debug) || input.key_pressed(egui::Key::F12) {
            self.player.captured = false;
            self.set_view_mode(if self.view_mode == ViewMode::ScreenOnly {
                ViewMode::Developer
            } else {
                ViewMode::ScreenOnly
            });
        }
        if pressed(&bindings.pause) {
            self.player.captured = false;
            self.toggle_run();
        }
        if pressed(&bindings.save) {
            self.save_player(Some(1));
        }
        if pressed(&bindings.load) {
            crate::layout::top_menu_bar::quick_load_slot(self, 1);
        }

        if self.view_mode == ViewMode::Developer {
            // F10: Step Instruction
            if !self.host.status.running
                && self.host.can_edit()
                && input.key_pressed(egui::Key::F10)
                && !input.modifiers.shift
            {
                self.disassembly_view_addr = None;
                self.host.queue(HostCommand::StepInstruction);
            }

            // Shift + F10: Step Backward (Rewind)
            if !self.host.status.running
                && self.host.can_edit()
                && input.key_pressed(egui::Key::F10)
                && input.modifiers.shift
            {
                self.disassembly_view_addr = None;
                self.host
                    .queue(HostCommand::History(HistoryCommand::Backward));
            }

            // F11: Step CCK
            if !self.host.status.running
                && self.host.can_edit()
                && input.key_pressed(egui::Key::F11)
                && !input.modifiers.shift
            {
                self.disassembly_view_addr = None;
                self.host.queue(HostCommand::StepCck);
            }

            // Shift + F11: Step to the next completed video field.
            if !self.host.status.running
                && self.host.can_edit()
                && input.key_pressed(egui::Key::F11)
                && input.modifiers.shift
            {
                self.disassembly_view_addr = None;
                self.host.queue(HostCommand::StepField);
            }

            // F8: Toggle Microcode Inspector
            if !self.host.status.running && self.host.can_edit() && input.key_pressed(egui::Key::F8)
            {
                self.show_microcode = !self.show_microcode;
            }

            // Alt + T: Toggle Temporal Recording
            if !self.host.status.running
                && self.host.can_edit()
                && input.modifiers.alt
                && input.key_pressed(egui::Key::T)
            {
                self.host
                    .queue(HostCommand::History(HistoryCommand::ToggleRecording));
            }
        }

        // Ctrl + R: Reset
        if input.modifiers.command && input.key_pressed(egui::Key::R) {
            self.host.queue(HostCommand::Reset);
            self.inspection.ready = false;
        }

        // Ctrl + O: Load Binary
        if input.modifiers.command && input.key_pressed(egui::Key::O) {
            self.open_load_binary_dialog();
        }

        // Ctrl + S: Save State to File
        if input.modifiers.command && input.key_pressed(egui::Key::S) {
            self.open_save_state_dialog();
        }

        // Ctrl + L: Load State from File
        if input.modifiers.command && input.key_pressed(egui::Key::L) {
            self.open_load_state_dialog();
        }

        // Ctrl + Plus / Equal: Zoom In
        let should_zoom_in = input.modifiers.command
            && (input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals));
        if should_zoom_in {
            let current = ctx.zoom_factor();
            ctx.set_zoom_factor((current + ZOOM_STEP).min(MAX_ZOOM));
        }

        // Ctrl + Minus: Zoom Out
        if input.modifiers.command && input.key_pressed(egui::Key::Minus) {
            let current = ctx.zoom_factor();
            ctx.set_zoom_factor((current - ZOOM_STEP).max(MIN_ZOOM));
        }

        // Ctrl + 0: Reset Zoom
        if input.modifiers.command && input.key_pressed(egui::Key::Num0) {
            ctx.set_zoom_factor(1.0);
        }
    }
}
