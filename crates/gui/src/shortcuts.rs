//! Global application shortcuts, sharing execution policy with the toolbar.

use crate::{EmulatorApp, ViewMode};

const ZOOM_STEP: f32 = 0.25;
const MIN_ZOOM: f32 = 0.75;
const MAX_ZOOM: f32 = 2.5;

impl EmulatorApp {
    /// Handles global keyboard shortcuts
    pub(crate) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let input = ctx.input(|i| i.clone());

        let toggle_view_mode = input.key_pressed(egui::Key::F12)
            || input.key_pressed(egui::Key::F2)
            || (self.view_mode == ViewMode::ScreenOnly && input.key_pressed(egui::Key::Escape));

        if toggle_view_mode {
            if self.view_mode == ViewMode::ScreenOnly {
                self.set_view_mode(ViewMode::Developer);
            } else {
                self.set_view_mode(ViewMode::ScreenOnly);
            }
        }

        // F5 or Space: Toggle Run / Pause
        let should_toggle_run = input.key_pressed(egui::Key::F5)
            || (input.key_pressed(egui::Key::Space) && !input.focused);
        if should_toggle_run {
            self.session.toggle_run();
        }

        if self.view_mode == ViewMode::Developer {
            // F10: Step Instruction
            if !self.session.is_running
                && input.key_pressed(egui::Key::F10)
                && !input.modifiers.shift
            {
                self.disassembly_view_addr = None;
                self.session.step_instruction();
            }

            // Shift + F10: Step Backward (Rewind)
            if !self.session.is_running
                && input.key_pressed(egui::Key::F10)
                && input.modifiers.shift
            {
                self.disassembly_view_addr = None;
                self.session.step_backward();
            }

            // F11: Step CCK
            if !self.session.is_running
                && input.key_pressed(egui::Key::F11)
                && !input.modifiers.shift
            {
                self.disassembly_view_addr = None;
                self.session.step_cck();
            }

            // Shift + F11: Step to the next completed video field.
            if !self.session.is_running
                && input.key_pressed(egui::Key::F11)
                && input.modifiers.shift
            {
                self.disassembly_view_addr = None;
                self.session.step_frame();
            }

            // F8: Toggle Microcode Inspector
            if !self.session.is_running && input.key_pressed(egui::Key::F8) {
                self.show_microcode = !self.show_microcode;
            }

            // Alt + T: Toggle Temporal Recording
            if !self.session.is_running && input.modifiers.alt && input.key_pressed(egui::Key::T) {
                self.session.temporal.toggle_recording();
            }
        }

        // Ctrl + R: Reset
        if input.modifiers.command && input.key_pressed(egui::Key::R) {
            self.session.reset();
            self.inspection.ready = false;
        }

        // Ctrl + O: Load Binary
        if input.modifiers.command && input.key_pressed(egui::Key::O) {
            self.open_load_binary_dialog();
        }

        // F6: Quick Save Slot 1
        if input.key_pressed(egui::Key::F6) {
            crate::layout::top_menu_bar::quick_save_slot(self, 1);
        }

        // F9: Quick Load Slot 1
        if input.key_pressed(egui::Key::F9) {
            crate::layout::top_menu_bar::quick_load_slot(self, 1);
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
