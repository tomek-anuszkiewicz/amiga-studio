//! Central EmulatorApp Orchestrator (Pure View & Presentation Layer)
//!
//! Owns the UI state, themes, and views, delegating machine execution orchestration
//! entirely to headless `debugger::DebuggerSession`.

use serde::{Deserialize, Serialize};

use crate::layout::left_dock::disassembly::{render_disassembly, DisasmEditState};
use crate::layout::left_dock::engine_status::render_engine_status;
use crate::layout::left_dock::microcode::render_microcode;
use crate::layout::left_dock::registers::{render_registers, EditRegister};
use crate::layout::main_viewport::amiga_screen::{render_amiga_screen, ScreenScale, ScreenTexture};
use crate::layout::main_viewport::temporal_bar::render_temporal_bar;
use crate::layout::right_dock::breakpoints_panel::{render_breakpoints_panel, BreakpointFormState};
use crate::layout::right_dock::memory_hex::render_memory_hex;
use crate::layout::right_dock::memory_search::{render_memory_search, MemorySearchState};
use crate::layout::right_dock::trace_log::render_trace_log;
use crate::layout::top_menu_bar::render_top_menu_bar;
use crate::playback::Playback;
use crate::theme::AppTheme;
use debugger::temporal::DEFAULT_TEMPORAL_CAPACITY;
use debugger::DebuggerSession;

const WIDE_LAYOUT_MIN_WIDTH: f32 = 1680.0;
const DESKTOP_LAYOUT_MIN_WIDTH: f32 = 1200.0;

/// Responsive layout tier based on viewport width
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutTier {
    /// Full HD and Ultrawide ($W \ge 1680\text{px}$): 4-pane Studio Workbench layout
    FullHdWide,
    /// Standard desktop ($1200\text{px} \le W < 1680\text{px}$): adaptive 3-column layout
    StandardDesktop,
    /// Compact / Small displays ($W < 1200\text{px}$): dense 3-column layout with scrollbars
    Compact,
}

impl LayoutTier {
    fn from_width(width: f32) -> Self {
        if width >= WIDE_LAYOUT_MIN_WIDTH {
            LayoutTier::FullHdWide
        } else if width >= DESKTOP_LAYOUT_MIN_WIDTH {
            LayoutTier::StandardDesktop
        } else {
            LayoutTier::Compact
        }
    }
}

/// App display mode: Developer Studio vs Clean Standalone Game Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ViewMode {
    #[default]
    Developer,
    ScreenOnly,
}

fn default_disasm_pane_width() -> f32 {
    460.0
}

fn default_crt_pane_height() -> f32 {
    380.0
}

fn default_right_dock_bottom_height() -> f32 {
    260.0
}

/// Persistent high-level user preferences saved across desktop sessions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPreferences {
    pub theme: AppTheme,
    pub view_mode: ViewMode,
    pub screen_scale: ScreenScale,
    pub show_microcode: bool,
    pub temporal_capacity: usize,
    pub disasm_pane_width: f32,
    pub crt_pane_height: f32,
    pub right_dock_bottom_height: f32,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: AppTheme::Dark,
            view_mode: ViewMode::Developer,
            screen_scale: ScreenScale::default(),
            show_microcode: true,
            temporal_capacity: DEFAULT_TEMPORAL_CAPACITY,
            disasm_pane_width: default_disasm_pane_width(),
            crt_pane_height: default_crt_pane_height(),
            right_dock_bottom_height: default_right_dock_bottom_height(),
        }
    }
}

/// Central Amiga 500 Emulator GUI Application (View Layer)
#[derive(Debug)]
pub struct EmulatorApp {
    /// Headless machine execution controller & model
    pub session: Box<DebuggerSession>,
    screen_texture: ScreenTexture,
    pub(crate) inspection: Box<crate::inspection::Inspection>,
    pub playback: Playback,
    pub show_fps: bool,
    pub loaded_rom_name: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub file_history: crate::file_actions::FileHistory,
    #[cfg(target_arch = "wasm32")]
    pub(crate) browser_file_task: Option<crate::file_actions::BrowserFileTask>,
    #[cfg(target_arch = "wasm32")]
    pub(crate) browser_history: crate::file_actions::BrowserHistory,

    // UI Configuration & Themes
    pub theme: AppTheme,
    pub view_mode: ViewMode,
    pub screen_scale: ScreenScale,

    // Interactive Edit states
    pub active_reg_edit: Option<(EditRegister, String)>,
    pub active_disasm_edit: Option<DisasmEditState>,

    // Resizable Splitter Sizes
    pub disasm_pane_width: f32,
    pub crt_pane_height: f32,
    pub right_dock_bottom_height: f32,

    // Selection & Stream Navigation
    pub memory_selected_addr: Option<u32>,
    pub disassembly_view_addr: Option<u32>,
    pub disassembly_selected_addr: Option<u32>,

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
    pub pending_binary: Option<crate::file_actions::BinarySource>,
    pub load_target_addr_str: String,
    pub load_auto_prime: bool,
    pub file_error: Option<String>,
    pub toast_message: Option<(String, usize)>,
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
            session: Box::new(DebuggerSession::new()),
            screen_texture: ScreenTexture::default(),
            inspection: Box::default(),
            playback: Playback::default(),
            show_fps: true,
            loaded_rom_name: None,
            #[cfg(not(target_arch = "wasm32"))]
            file_history: crate::file_actions::FileHistory::default(),
            #[cfg(target_arch = "wasm32")]
            browser_file_task: None,
            #[cfg(target_arch = "wasm32")]
            browser_history: crate::file_actions::BrowserHistory::default(),
            theme: AppTheme::Dark,
            view_mode,
            screen_scale: ScreenScale::default(),
            active_reg_edit: None,
            active_disasm_edit: None,
            disasm_pane_width: default_disasm_pane_width(),
            crt_pane_height: default_crt_pane_height(),
            right_dock_bottom_height: default_right_dock_bottom_height(),
            memory_selected_addr: None,
            disassembly_view_addr: None,
            disassembly_selected_addr: None,
            show_microcode: true,
            hex_base_addr: 0x000000,
            hex_edit_buffer: (0, String::new()),
            memory_search_state: MemorySearchState::default(),
            goto_addr_str: "000000".to_string(),
            target_cck_input: String::new(),
            temporal_capacity_selection: DEFAULT_TEMPORAL_CAPACITY,
            breakpoint_form: BreakpointFormState::default(),
            load_binary_modal_open: false,
            pending_binary: None,
            load_target_addr_str: "001000".to_string(),
            load_auto_prime: true,
            file_error: None,
            toast_message: None,
        }
    }
}

impl EmulatorApp {
    /// Initializes a new EmulatorApp instance with headless session
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        #[cfg(target_arch = "wasm32")]
        app.browser_history.restore();
        if let Some(storage) = cc.storage {
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.file_history =
                    eframe::get_value(storage, crate::file_actions::HISTORY_STORAGE_KEY)
                        .unwrap_or_default();
            }
            if let Some(prefs) = eframe::get_value::<UserPreferences>(storage, eframe::APP_KEY) {
                app.apply_preferences(&prefs);
            }
        }
        app.theme.apply(&cc.egui_ctx);
        app
    }

    /// Exports current persistent user preferences
    pub fn preferences(&self) -> UserPreferences {
        UserPreferences {
            theme: self.theme,
            view_mode: self.view_mode,
            screen_scale: self.screen_scale,
            show_microcode: self.show_microcode,
            temporal_capacity: self.temporal_capacity_selection,
            disasm_pane_width: self.disasm_pane_width,
            crt_pane_height: self.crt_pane_height,
            right_dock_bottom_height: self.right_dock_bottom_height,
        }
    }

    /// Applies loaded user preferences to active app state
    pub fn apply_preferences(&mut self, prefs: &UserPreferences) {
        self.theme = prefs.theme;
        self.view_mode = prefs.view_mode;
        self.screen_scale = prefs.screen_scale;
        self.show_microcode = prefs.show_microcode;
        self.temporal_capacity_selection = prefs.temporal_capacity;
        self.disasm_pane_width = prefs.disasm_pane_width;
        self.crt_pane_height = prefs.crt_pane_height;
        self.right_dock_bottom_height = prefs.right_dock_bottom_height;
        self.session.temporal.set_capacity(prefs.temporal_capacity);
    }

    /// Changes presentation and connects debugger work only in Developer view.
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        if self.view_mode == ViewMode::ScreenOnly && mode == ViewMode::Developer {
            self.session.request_pause();
        }
        self.view_mode = mode;
        self.session
            .set_debugger_enabled(mode == ViewMode::Developer);
    }

    /// Renders one complete UI frame, handling input events, execution time-slicing, and docked panels
    pub fn update_ui(&mut self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        let update_started = std::time::Instant::now();
        #[cfg(target_arch = "wasm32")]
        let update_started = eframe::web::now_sec();
        #[cfg(not(target_arch = "wasm32"))]
        let elapsed = || update_started.elapsed().as_secs_f64();
        #[cfg(target_arch = "wasm32")]
        let elapsed = || eframe::web::now_sec() - update_started;
        let now = ctx.input(|input| input.time);
        let was_running = self.session.is_running;
        let first_pass = ctx.output(|output| output.num_completed_passes == 0);
        if first_pass {
            self.session
                .set_debugger_enabled(self.view_mode == ViewMode::Developer);
            self.handle_keyboard_shortcuts(ctx);
        }

        // Check for drag-and-dropped files onto the emulator window
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if first_pass {
            for file in dropped_files {
                self.accept_dropped_file(file);
            }
        }

        // A Run started before the first UI pass needs one preparation pass before execution.
        let prepare_first = self.view_mode == ViewMode::Developer
            && !self.inspection.ready
            && self.session.continuous_run()
            && !self.session.pause_pending();
        let start_after_preparation = std::mem::take(&mut self.inspection.start_after_preparation);
        let refresh_screen = ((first_pass || start_after_preparation)
            && !prepare_first
            && self.playback.advance(now, &mut self.session, elapsed))
            || self.screen_texture.image().is_none();
        render_top_menu_bar(self, ctx);
        if was_running && !self.session.is_running {
            self.disassembly_view_addr = None;
        }
        self.inspection.frozen = self.session.continuous_run()
            && (self.inspection.ready || self.session.pause_pending());
        if self.view_mode == ViewMode::Developer && !self.inspection.frozen {
            self.inspection.engine.refresh(&self.session);
            if self.show_microcode {
                self.inspection.microcode.refresh(
                    &self.session.machine.cpu.state,
                    self.session.machine.physical_memory.chip_ram_blocked,
                );
            }
        }
        self.screen_texture.set_scale(self.screen_scale);
        if refresh_screen {
            self.screen_texture.update(ctx, &self.session);
        }
        let screen_texture = self.screen_texture.image();

        // --- RENDER BASED ON ACTIVE VIEW MODE ---
        if self.view_mode == ViewMode::ScreenOnly {
            // Game view never invokes inspection panels or their data queries.
            egui::CentralPanel::default().show(ctx, |ui| {
                render_amiga_screen(ui, screen_texture, self.screen_scale, false);
            });
        } else {
            // Mode B: Full Developer Studio GUI (Responsive Multi-Tier Architecture)
            let tokens = self.theme.tokens();

            let screen_rect = ctx.screen_rect();
            let total_w = screen_rect.width();
            let layout_tier = LayoutTier::from_width(total_w);

            let left_default = match layout_tier {
                LayoutTier::FullHdWide => 360.0,
                LayoutTier::StandardDesktop => 340.0,
                LayoutTier::Compact => 290.0,
            };
            let left_min = if layout_tier == LayoutTier::Compact {
                260.0
            } else {
                280.0
            };
            let right_default = match layout_tier {
                LayoutTier::FullHdWide => 540.0,
                LayoutTier::StandardDesktop => 480.0,
                LayoutTier::Compact => 350.0,
            };
            let right_min = if layout_tier == LayoutTier::Compact {
                330.0
            } else {
                360.0
            };

            // Column 1: Left Dock (Execution, CPU Registers, Engine Status & Microcode)
            egui::SidePanel::left("left_dock")
                .resizable(true)
                .default_width(left_default)
                .width_range(left_min..=480.0)
                .frame(
                    egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin {
                        left: 8,
                        right: 4,
                        top: 6,
                        bottom: 6,
                    }),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("left_dock_scroll")
                        .auto_shrink([false, false])
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                        )
                        .show(ui, |ui| {
                            if self.session.is_running {
                                ui.disable();
                            }
                            if render_registers(
                                ui,
                                &mut self.inspection.registers,
                                !self.inspection.frozen,
                                !self.session.is_running,
                                &tokens,
                                &mut self.session.machine.cpu,
                                &mut self.session.machine.physical_memory,
                                self.session.prev_cpu_state.as_ref(),
                                &mut self.active_reg_edit,
                            ) {
                                self.session.invalidate_instruction_tracking();
                            }
                            ui.add_space(3.0);
                            render_engine_status(ui, &tokens, &self.inspection.engine);
                            if self.show_microcode {
                                ui.add_space(3.0);
                                render_microcode(ui, &self.inspection.microcode);
                            }
                        });
                });

            // Column 3: Right Dock (Memory Hex + Tools)
            egui::SidePanel::right("right_dock")
                .resizable(true)
                .default_width(right_default)
                .width_range(right_min..=680.0)
                .frame(
                    egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin {
                        left: 4,
                        right: 8,
                        top: 6,
                        bottom: 6,
                    }),
                )
                .show(ctx, |ui| {
                    if self.session.is_running {
                        ui.disable();
                    }
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        // 1. Bottom Zone: Tool Panels docked to bottom (rendered first in bottom_up)
                        let bottom_h = self.right_dock_bottom_height.clamp(50.0, 420.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), bottom_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("right_dock_tools_scroll")
                                    .auto_shrink([false, false])
                                    .scroll_bar_visibility(
                                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                    )
                                    .show(ui, |ui| {
                                        let tools_resp = ui.vertical(|ui| {
                                            render_memory_search(
                                                ui,
                                                &self.session.machine.physical_memory,
                                                &mut self.hex_base_addr,
                                                &mut self.memory_search_state,
                                            );
                                            ui.add_space(6.0);
                                            render_breakpoints_panel(
                                                &mut self.session.debugger.breakpoints,
                                                &mut self.inspection.breakpoints,
                                                !self.inspection.frozen,
                                                &mut self.breakpoint_form,
                                                ui,
                                            );
                                            if layout_tier != LayoutTier::FullHdWide {
                                                ui.add_space(6.0);
                                                render_trace_log(self, ui);
                                            }
                                        });
                                        let measured_h = tools_resp.response.rect.height();
                                        if measured_h > 30.0 {
                                            self.right_dock_bottom_height = measured_h + 8.0;
                                            let tools_h_id =
                                                egui::Id::new("right_dock_tools_measured_h");
                                            ui.data_mut(|d| {
                                                d.insert_temp(
                                                    tools_h_id,
                                                    self.right_dock_bottom_height,
                                                )
                                            });
                                        }
                                    });
                            },
                        );

                        ui.add_space(4.0);

                        // 2. Top Zone: Memory Hex Editor (takes all remaining vertical space)
                        let remaining_h = ui.available_height();
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), remaining_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                render_memory_hex(
                                    ui,
                                    &mut self.inspection.memory,
                                    !self.inspection.frozen,
                                    !self.session.is_running,
                                    &mut self.session.machine.physical_memory,
                                    &mut self.session.debugger.breakpoints,
                                    &mut self.hex_base_addr,
                                    &mut self.memory_selected_addr,
                                    &mut self.hex_edit_buffer,
                                    if !self.inspection.frozen
                                        && self.session.prev_cpu_state.is_some()
                                    {
                                        Some((
                                            &self.session.prev_hex_bytes,
                                            self.session.prev_hex_base,
                                        ))
                                    } else {
                                        None
                                    },
                                    &tokens,
                                );
                            },
                        );
                    });
                });

            // Column 2: Central Viewport
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::central_panel(&ctx.style()).inner_margin(egui::Margin {
                        left: 4,
                        right: 4,
                        top: 6,
                        bottom: 6,
                    }),
                )
                .show(ctx, |ui| {
                    match layout_tier {
                        LayoutTier::FullHdWide => {
                            // 4-Pane Studio Workbench Layout (Split Central Area)
                            let total_w = ui.available_width();
                            let total_h = ui.available_height();
                            let min_disasm_w = 280.0_f32;
                            let max_disasm_w = (total_w - 380.0).max(min_disasm_w);
                            let disasm_w = self.disasm_pane_width.clamp(min_disasm_w, max_disasm_w);

                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                // Sub-column 0: Dedicated Full-height Disassembly Stream
                                ui.allocate_ui_with_layout(
                                    egui::vec2(disasm_w, total_h),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        if render_disassembly(
                                            ui,
                                            &mut self.inspection.disassembly,
                                            !self.inspection.frozen,
                                            !self.session.is_running,
                                            &mut self.session.machine.cpu,
                                            &mut self.session.machine.physical_memory,
                                            &mut self.session.debugger,
                                            &mut self.session.temporal,
                                            &mut self.goto_addr_str,
                                            &mut self.disassembly_view_addr,
                                            &mut self.disassembly_selected_addr,
                                            &mut self.active_disasm_edit,
                                            &tokens,
                                        ) {
                                            self.session.invalidate_instruction_tracking();
                                        }
                                    },
                                );

                                // Draggable Vertical Splitter between Disassembly and CRT/Trace
                                let splitter_resp = ui.allocate_response(
                                    egui::vec2(6.0, total_h),
                                    egui::Sense::click_and_drag(),
                                );
                                let splitter_hovered =
                                    splitter_resp.hovered() || splitter_resp.dragged();
                                let splitter_color = if splitter_hovered {
                                    tokens.border_active
                                } else {
                                    tokens.border_subtle
                                };
                                ui.painter().vline(
                                    splitter_resp.rect.center().x,
                                    ui.max_rect().y_range(),
                                    egui::Stroke::new(
                                        if splitter_hovered { 2.0_f32 } else { 1.0_f32 },
                                        splitter_color,
                                    ),
                                );
                                if splitter_resp.hovered() || splitter_resp.dragged() {
                                    ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                }
                                if splitter_resp.dragged() {
                                    self.disasm_pane_width = (self.disasm_pane_width
                                        + splitter_resp.drag_delta().x)
                                        .clamp(min_disasm_w, max_disasm_w);
                                }

                                // Sub-column 1: Amiga Screen + Temporal Bar (top) + Trace Log (bottom)
                                let remaining_w = ui.available_width();
                                ui.allocate_ui_with_layout(
                                    egui::vec2(remaining_w, total_h),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        let total_col_h = ui.available_height();
                                        let top_h = self
                                            .crt_pane_height
                                            .clamp(200.0, (total_col_h - 100.0).max(220.0));

                                        ui.allocate_ui_with_layout(
                                            egui::vec2(ui.available_width(), top_h),
                                            egui::Layout::top_down(egui::Align::Center),
                                            |ui| {
                                                let avail_w = ui.available_width();
                                                let avail_h =
                                                    (ui.available_height() - 48.0).max(120.0);
                                                ui.allocate_ui_with_layout(
                                                    egui::vec2(avail_w, avail_h),
                                                    egui::Layout::top_down(egui::Align::Center),
                                                    |ui| {
                                                        render_amiga_screen(
                                                            ui,
                                                            screen_texture,
                                                            self.screen_scale,
                                                            false,
                                                        );
                                                    },
                                                );

                                                ui.separator();
                                                render_temporal_bar(self, ui);
                                            },
                                        );

                                        // Draggable Splitter between CRT Screen/Temporal Bar and Trace Log
                                        let crt_splitter = ui.allocate_response(
                                            egui::vec2(ui.available_width(), 6.0),
                                            egui::Sense::click_and_drag(),
                                        );
                                        let crt_hovered =
                                            crt_splitter.hovered() || crt_splitter.dragged();
                                        let crt_split_color = if crt_hovered {
                                            tokens.border_active
                                        } else {
                                            tokens.border_subtle
                                        };
                                        ui.painter().hline(
                                            crt_splitter.rect.x_range(),
                                            crt_splitter.rect.center().y,
                                            egui::Stroke::new(
                                                if crt_hovered { 2.0_f32 } else { 1.0_f32 },
                                                crt_split_color,
                                            ),
                                        );
                                        if crt_splitter.hovered() || crt_splitter.dragged() {
                                            ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
                                        }
                                        if crt_splitter.dragged() {
                                            self.crt_pane_height = (self.crt_pane_height
                                                + crt_splitter.drag_delta().y)
                                                .clamp(200.0, (total_col_h - 100.0).max(220.0));
                                        }

                                        ui.add_space(2.0);
                                        render_trace_log(self, ui);
                                    },
                                );
                            });
                        }
                        LayoutTier::StandardDesktop | LayoutTier::Compact => {
                            // Stacked Adaptive 3-Column Layout
                            let total_h = ui.available_height();
                            let screen_h = (total_h * 0.38).clamp(130.0, 260.0);

                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), screen_h),
                                egui::Layout::top_down(egui::Align::Center),
                                |ui| {
                                    render_amiga_screen(
                                        ui,
                                        screen_texture,
                                        self.screen_scale,
                                        false,
                                    );
                                },
                            );

                            ui.separator();
                            render_temporal_bar(self, ui);
                            ui.separator();

                            if render_disassembly(
                                ui,
                                &mut self.inspection.disassembly,
                                !self.inspection.frozen,
                                !self.session.is_running,
                                &mut self.session.machine.cpu,
                                &mut self.session.machine.physical_memory,
                                &mut self.session.debugger,
                                &mut self.session.temporal,
                                &mut self.goto_addr_str,
                                &mut self.disassembly_view_addr,
                                &mut self.disassembly_selected_addr,
                                &mut self.active_disasm_edit,
                                &tokens,
                            ) {
                                self.session.invalidate_instruction_tracking();
                            }
                        }
                    }
                });
        }

        if self.view_mode == ViewMode::Developer && !self.inspection.frozen {
            self.inspection.ready = true;
            if prepare_first {
                self.inspection.start_after_preparation = true;
                ctx.request_discard("Prepare inspection before the first Run slice");
            }
        }
        if first_pass {
            self.poll_file_dialog(ctx);
        }
        crate::layout::file_dialogs::render(self, ctx);

        self.playback
            .request_repaint(ctx, &self.session, now + elapsed());
    }
}

impl eframe::App for EmulatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_ui(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.preferences());
        #[cfg(not(target_arch = "wasm32"))]
        eframe::set_value(
            storage,
            crate::file_actions::HISTORY_STORAGE_KEY,
            &self.file_history,
        );
    }

    fn persist_egui_memory(&self) -> bool {
        true
    }
}
