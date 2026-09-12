//! Central EmulatorApp Orchestrator (Pure View & Presentation Layer)
//!
//! Owns the UI state, themes, and views, delegating machine execution orchestration
//! entirely to headless `debugger::DebuggerSession`.

use std::path::PathBuf;

use crate::layout::left_dock::disassembly::{render_disassembly, DisasmEditState};
use crate::layout::left_dock::microcode::render_microcode;
use crate::layout::left_dock::registers::{render_registers, EditRegister};
use crate::layout::main_viewport::amiga_screen::render_amiga_screen;
use crate::layout::main_viewport::temporal_bar::render_temporal_bar;
use crate::layout::right_dock::breakpoints_panel::{render_breakpoints_panel, BreakpointFormState};
use crate::layout::right_dock::memory_hex::render_memory_hex;
use crate::layout::right_dock::memory_search::{render_memory_search, MemorySearchState};
use crate::layout::right_dock::trace_log::render_trace_log;
use crate::layout::top_menu_bar::render_top_menu_bar;
use crate::theme::AppTheme;
use debugger::temporal::DEFAULT_TEMPORAL_CAPACITY;
use debugger::DebuggerSession;

/// Maximum instructions executed per GUI frame during free-running emulation
pub const MAX_INSTRUCTIONS_PER_FRAME: usize = 5000;

/// App display mode: Developer Studio vs Clean Standalone Game Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Developer,
    ScreenOnly,
}

/// Central Amiga 500 Emulator GUI Application (View Layer)
pub struct EmulatorApp {
    /// Headless machine execution controller & model
    pub session: DebuggerSession,

    // UI Configuration & Themes
    pub theme: AppTheme,
    pub view_mode: ViewMode,

    // Interactive Edit states
    pub active_reg_edit: Option<(EditRegister, String)>,
    pub active_disasm_edit: Option<DisasmEditState>,

    // UI Panel States
    pub show_microcode: bool,
    pub hex_base_addr: u32,
    pub hex_edit_buffer: (u32, String),
    pub memory_search_state: MemorySearchState,
    pub goto_addr_str: String,
    pub target_cck_input: String,
    pub temporal_capacity_selection: usize,
    pub breakpoint_form: BreakpointFormState,

    // File loading modal state
    pub load_binary_modal_open: bool,
    pub pending_binary_path: Option<PathBuf>,
    pub load_target_addr_str: String,
    pub load_auto_prime: bool,
}

impl Default for EmulatorApp {
    fn default() -> Self {
        let view_mode = if cfg!(target_arch = "wasm32") {
            ViewMode::ScreenOnly
        } else {
            match std::env::var("AMIGA_DEV_GUI").as_deref() {
                Ok("0") | Ok("false") | Ok("off") => ViewMode::ScreenOnly,
                _ => ViewMode::Developer,
            }
        };

        Self {
            session: DebuggerSession::new(),
            theme: AppTheme::Dark,
            view_mode,
            active_reg_edit: None,
            active_disasm_edit: None,
            show_microcode: true,
            hex_base_addr: 0x000000,
            hex_edit_buffer: (0, String::new()),
            memory_search_state: MemorySearchState::default(),
            goto_addr_str: "001000".to_string(),
            target_cck_input: String::new(),
            temporal_capacity_selection: DEFAULT_TEMPORAL_CAPACITY,
            breakpoint_form: BreakpointFormState::default(),
            load_binary_modal_open: false,
            pending_binary_path: None,
            load_target_addr_str: "001000".to_string(),
            load_auto_prime: true,
        }
    }
}

impl EmulatorApp {
    /// Initializes a new EmulatorApp instance with headless session
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let app = Self::default();
        app.theme.apply(&cc.egui_ctx);
        app
    }

    /// Opens the native file chooser dialog for binary injection
    pub fn open_load_binary_dialog(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = rfd::FileDialog::new()
                .set_title("Select M68000 Binary Machine Code")
                .add_filter("Raw Binary / ROM", &["bin", "rom", "exe", "dat"])
                .pick_file();

            if let Some(path) = file {
                self.pending_binary_path = Some(path);
                self.load_binary_modal_open = true;
            }
        }
    }

    /// Handles global keyboard shortcuts
    pub fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let input = ctx.input(|i| i.clone());

        let toggle_view_mode = input.key_pressed(egui::Key::F12)
            || input.key_pressed(egui::Key::F2)
            || (self.view_mode == ViewMode::ScreenOnly && input.key_pressed(egui::Key::Escape));

        if toggle_view_mode {
            if self.view_mode == ViewMode::ScreenOnly {
                self.session.is_running = false; // Pause emulation to inspect state
                self.view_mode = ViewMode::Developer;
            } else {
                self.view_mode = ViewMode::ScreenOnly;
            }
        }

        // F5 or Space: Toggle Run / Pause
        if input.key_pressed(egui::Key::F5)
            || (input.key_pressed(egui::Key::Space) && !input.focused)
        {
            self.session.toggle_run();
        }

        // F10: Step Instruction
        if input.key_pressed(egui::Key::F10) && !input.modifiers.shift {
            self.session.step_instruction();
        }

        // Shift + F10: Step Backward (Rewind)
        if input.key_pressed(egui::Key::F10) && input.modifiers.shift {
            self.session.step_backward();
        }

        // F11: Step CCK
        if input.key_pressed(egui::Key::F11) {
            self.session.step_cck();
        }

        // F8: Toggle Microcode Inspector
        if input.key_pressed(egui::Key::F8) {
            self.show_microcode = !self.show_microcode;
        }

        // Alt + T: Toggle Temporal Recording
        if input.modifiers.alt && input.key_pressed(egui::Key::T) {
            self.session.temporal.toggle_recording();
        }

        // Ctrl + R: Reset Cold
        if input.modifiers.command && input.key_pressed(egui::Key::R) {
            self.session.reset_cold();
        }

        // Ctrl + O: Load Binary
        if input.modifiers.command && input.key_pressed(egui::Key::O) {
            self.open_load_binary_dialog();
        }

        // Ctrl + Plus / Equal: Zoom In
        if input.modifiers.command
            && (input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals))
        {
            let current = ctx.zoom_factor();
            ctx.set_zoom_factor((current + 0.25).min(2.5));
        }

        // Ctrl + Minus: Zoom Out
        if input.modifiers.command && input.key_pressed(egui::Key::Minus) {
            let current = ctx.zoom_factor();
            ctx.set_zoom_factor((current - 0.25).max(0.75));
        }

        // Ctrl + 0: Reset Zoom
        if input.modifiers.command && input.key_pressed(egui::Key::Num0) {
            ctx.set_zoom_factor(1.0);
        }
    }

    /// Renders one complete UI frame, handling input events, execution time-slicing, and docked panels
    pub fn update_ui(&mut self, ctx: &egui::Context) {
        self.handle_keyboard_shortcuts(ctx);

        // Check for drag-and-dropped files onto the emulator window
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped_files {
            if let Some(path) = file.path {
                self.pending_binary_path = Some(path);
                self.load_binary_modal_open = true;
            }
        }

        // Bounded Time-Slicing Execution Loop (when free-running)
        if self.session.is_running {
            self.session.run_slice(MAX_INSTRUCTIONS_PER_FRAME);
            if self.session.is_running {
                ctx.request_repaint(); // Request next frame immediately
            }
        }

        // --- RENDER BASED ON ACTIVE VIEW MODE ---
        if self.view_mode == ViewMode::ScreenOnly {
            // Mode A: Clean Standalone Screen / Game View (no floating buttons)
            egui::CentralPanel::default().show(ctx, |ui| {
                render_amiga_screen(ui, false);
            });
        } else {
            // Mode B: Full Developer Studio GUI (3-Column Architecture)
            render_top_menu_bar(self, ctx);

            let screen_rect = ctx.screen_rect();
            let total_w = screen_rect.width();
            let compact = total_w < 1250.0;

            let left_default = if compact { 330.0 } else { 350.0 };
            let right_default = if compact { 390.0 } else { 540.0 };

            // Column 1: Left Dock (Execution & CPU Registers + Microcode)
            egui::SidePanel::left("left_dock")
                .resizable(true)
                .default_width(left_default)
                .width_range(280.0..=450.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("left_dock_scroll")
                        .auto_shrink([false, false])
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                        )
                        .show(ui, |ui| {
                            render_registers(
                                ui,
                                &mut self.session.cpu,
                                &mut self.session.bus,
                                self.session.prev_cpu_state.as_ref(),
                                &mut self.active_reg_edit,
                            );
                            if self.show_microcode {
                                ui.add_space(3.0);
                                render_microcode(
                                    ui,
                                    &self.session.cpu.state,
                                    self.session.bus.is_chip_ram_locked(),
                                );
                            }
                        });
                });

            // Column 3: Right Dock (Memory, Search, Breakpoints & Trace Log)
            egui::SidePanel::right("right_dock")
                .resizable(true)
                .default_width(right_default)
                .width_range(360.0..=650.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("right_dock_scroll")
                        .auto_shrink([false, false])
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                        )
                        .show(ui, |ui| {
                            render_memory_hex(
                                ui,
                                &mut self.session.bus,
                                &mut self.session.debugger.breakpoints,
                                &mut self.hex_base_addr,
                                &mut self.hex_edit_buffer,
                                if self.session.prev_cpu_state.is_some() {
                                    Some((&self.session.prev_hex_bytes, self.session.prev_hex_base))
                                } else {
                                    None
                                },
                            );
                            ui.separator();
                            render_memory_search(
                                ui,
                                &self.session.bus,
                                &mut self.hex_base_addr,
                                &mut self.memory_search_state,
                            );
                            ui.separator();
                            render_breakpoints_panel(
                                &mut self.session.debugger.breakpoints,
                                &mut self.breakpoint_form,
                                ui,
                            );
                            ui.separator();
                            render_trace_log(self, ui);
                        });
                });

            // Column 2: Central Viewport (Amiga CRT Screen + Temporal Bar + Disassembly Stream)
            egui::CentralPanel::default().show(ctx, |ui| {
                let total_h = ui.available_height();
                let screen_h = (total_h * 0.38).clamp(130.0, 260.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), screen_h),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        render_amiga_screen(ui, false);
                    },
                );

                ui.separator();
                render_temporal_bar(self, ui);
                ui.separator();

                egui::ScrollArea::both()
                    .id_salt("center_disasm_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        render_disassembly(
                            ui,
                            &mut self.session.cpu,
                            &mut self.session.bus,
                            &mut self.session.debugger,
                            &mut self.session.temporal,
                            &mut self.goto_addr_str,
                            &mut self.active_disasm_edit,
                        );
                    });
            });
        }

        // --- Modal Dialog: Load Binary into Memory ---
        if self.load_binary_modal_open {
            egui::Window::new("Load Binary into Memory")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    if let Some(path) = &self.pending_binary_path {
                        ui.label(format!("File: {}", path.display()));
                    }

                    ui.horizontal(|ui| {
                        ui.label("Target RAM Address:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.load_target_addr_str)
                                .desired_width(80.0),
                        );
                    });

                    ui.checkbox(&mut self.load_auto_prime, "Auto-set PC & prime prefetch");

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Inject into RAM").clicked() {
                            #[cfg(not(target_arch = "wasm32"))]
                            if let Some(path) = &self.pending_binary_path {
                                if let Ok(data) = std::fs::read(path) {
                                    let clean =
                                        self.load_target_addr_str.trim().trim_start_matches('$');
                                    let target_addr =
                                        u32::from_str_radix(clean, 16).unwrap_or(0x001000);
                                    self.session.load_binary(
                                        target_addr,
                                        &data,
                                        self.load_auto_prime,
                                    );
                                    self.hex_base_addr = target_addr & 0x00FF_FFF0;
                                }
                            }
                            self.load_binary_modal_open = false;
                        }

                        if ui.button("Cancel").clicked() {
                            self.load_binary_modal_open = false;
                        }
                    });
                });
        }
    }
}

impl eframe::App for EmulatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_ui(ctx);
    }
}
