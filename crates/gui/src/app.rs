//! Central EmulatorApp Orchestrator (Pure View & Presentation Layer)
//!
//! Owns the UI state, themes, and views, delegating machine execution orchestration
//! entirely to headless `debugger::DebuggerSession`.

use serde::{Deserialize, Serialize};

use crate::app_host::AppHost;
use crate::host_commands::{HistoryCommand, HostCommand};
use crate::host_video::VideoRead;
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
use crate::theme::AppTheme;
use debugger::temporal::DEFAULT_TEMPORAL_CAPACITY;
use debugger::DebuggerSession;

#[cfg(not(target_arch = "wasm32"))]
mod screenshot;

pub(crate) const RTC_BATTERY_STORAGE_KEY: &str = "rtc_battery";

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
    /// GUI-owned command handle and retained inspection data.
    pub host: Box<AppHost>,
    screen_texture: ScreenTexture,
    pub(crate) inspection: Box<crate::inspection::Inspection>,
    pub show_fps: bool,
    pub player: Box<crate::player::PlayerState>,
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
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) closing: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) close_ready: bool,
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

        let mut session = Box::new(DebuggerSession::new());
        session
            .machine
            .rtc
            .supply_host_utc(crate::host_clock::utc_now());
        session.machine.rtc.load_battery(rtc::RtcBattery::default());
        Self {
            host: Box::new(AppHost::new(session)),
            screen_texture: ScreenTexture::default(),
            inspection: Box::default(),
            show_fps: true,
            player: Default::default(),
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
            #[cfg(not(target_arch = "wasm32"))]
            closing: false,
            #[cfg(not(target_arch = "wasm32"))]
            close_ready: false,
        }
    }
}

impl EmulatorApp {
    pub fn with_session(session: Box<DebuggerSession>) -> Self {
        Self {
            host: Box::new(AppHost::new(session)),
            ..Self::default()
        }
    }

    pub(crate) fn toggle_run(&mut self) {
        self.host.queue(if self.host.status.running {
            HostCommand::Pause
        } else {
            HostCommand::Run
        });
    }

    /// Initializes a new EmulatorApp instance with headless session
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.host.native_gamepads = true;
        }
        #[cfg(target_arch = "wasm32")]
        app.browser_history.restore();
        if let Some(storage) = cc.storage {
            if let Some(battery) = eframe::get_value(storage, RTC_BATTERY_STORAGE_KEY) {
                app.host.queue(HostCommand::LoadRtcBattery(battery));
            }
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
        if let Some(storage) = cc.storage {
            if let Some(catalog) =
                eframe::get_value(storage, crate::player_config::CONFIG_STORAGE_KEY)
            {
                app.player.catalog = catalog;
            }
        }
        app.player.screen = if cfg!(target_arch = "wasm32") {
            crate::player::PlayerScreen::Setup
        } else {
            crate::player::PlayerScreen::Library
        };
        app.set_view_mode(ViewMode::ScreenOnly);
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
        self.host
            .queue(HostCommand::History(HistoryCommand::Capacity(
                prefs.temporal_capacity,
            )));
    }

    /// Renders one complete UI frame using owner replies, image notifications, and docked panels
    pub fn update_ui(&mut self, ctx: &egui::Context) {
        let first_pass = ctx.output(|output| output.num_completed_passes == 0);
        crate::keyboard_input::begin_editor_collection(ctx);
        self.poll_host(ctx, first_pass);
        self.poll_configuration_save(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        if first_pass {
            self.poll_library(ctx);
        }

        // Check for drag-and-dropped files onto the emulator window
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if first_pass && !self.player.show_saves {
            for file in dropped_files {
                self.accept_dropped_file(file);
            }
        }

        self.screen_texture.set_scale(self.screen_scale);
        let consume_image = first_pass && self.host.error.is_none();
        let mut next_image = None;
        if let Some(runtime) = self.host.runtime.as_mut().filter(|_| consume_image) {
            let result = runtime.try_consume(|info, status, pixels| {
                if !status.running && info.cck != status.cck {
                    return None;
                }
                Some((info, ScreenTexture::convert(info, pixels)))
            });
            let status = runtime.status();
            let previous = self.host.status;
            self.host.accept_status(status);
            if previous != status {
                if previous.generation != status.generation {
                    self.player.captured = false;
                    if self.host.data.is_none() {
                        self.inspection.ready = false;
                    }
                }
                if previous.running && !status.running {
                    self.disassembly_view_addr = None;
                }
                ctx.request_repaint();
            }
            match result {
                Ok(VideoRead::Frame(image)) => next_image = image,
                Ok(_) => {}
                Err(error) => {
                    self.file_error = Some(format!("Could not read machine image: {error}"))
                }
            }
        }
        crate::layout::file_dialogs::limit_dialog_focus(self, ctx);
        let in_session = self.player.screen == crate::player::PlayerScreen::Session;
        if in_session {
            render_top_menu_bar(self, ctx);
            if self.view_mode == ViewMode::ScreenOnly {
                // Handle screenshot clicks before installing the pending image.
                self.render_player_controls(ctx);
            }
        }
        if first_pass {
            // The session controls can open Saved progress in the same pass
            // as keys queued by the window. Dispatch the keys after that click.
            self.handle_keyboard_input(ctx);
        }
        self.render_player_confirmation(ctx);
        if let Some((info, image)) = next_image {
            self.host.presented = Some(info);
            self.screen_texture.update(ctx, image);
        }
        if !in_session {
            self.render_player_screen(ctx);
            self.finish_ui_pass(ctx, first_pass);
            return;
        }
        self.inspection.frozen = (self.host.continuous_requested()
            || (self.host.inspector_revision.is_none() && self.host.data.is_some()))
            && (self.inspection.ready || self.host.status.pause_pending);
        let screen_texture = self.screen_texture.image();

        // --- RENDER BASED ON ACTIVE VIEW MODE ---
        if self.view_mode == ViewMode::ScreenOnly {
            // Game view never invokes inspection panels or their data queries.
            egui::CentralPanel::default().show(ctx, |ui| {
                render_amiga_screen(ui, screen_texture, self.screen_scale, false);
            });
        } else {
            // Mode B: Full Developer Studio GUI (Responsive Multi-Tier Architecture)
            let Some(mut data) = self.host.data.take() else {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label("Preparing debugger inspection...");
                });
                self.finish_ui_pass(ctx, first_pass);
                return;
            };
            let original_breakpoints = data.debugger.breakpoints.clone();
            if !self.inspection.frozen {
                self.inspection.refresh_status(&data);
                self.inspection.engine.refresh(&data);
                if self.show_microcode {
                    self.inspection
                        .microcode
                        .refresh(&data.cpu, data.memory.chip_ram_blocked);
                }
            }
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
                            if self.host.status.running {
                                ui.disable();
                            }
                            render_registers(
                                ui,
                                &mut self.inspection.registers,
                                !self.inspection.frozen,
                                !self.host.status.running && self.host.can_edit(),
                                &tokens,
                                &data.cpu,
                                &mut self.host.commands,
                                data.previous_cpu.as_ref(),
                                &mut self.active_reg_edit,
                            );
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
                    if self.host.status.running {
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
                                                &data.memory,
                                                &mut self.hex_base_addr,
                                                &mut self.memory_search_state,
                                            );
                                            ui.add_space(6.0);
                                            render_breakpoints_panel(
                                                &mut data.debugger.breakpoints,
                                                &mut self.inspection.breakpoints,
                                                !self.inspection.frozen,
                                                &mut self.breakpoint_form,
                                                ui,
                                            );
                                            if layout_tier != LayoutTier::FullHdWide {
                                                ui.add_space(6.0);
                                                render_trace_log(self, ui, &data);
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
                                    !self.host.status.running && self.host.can_edit(),
                                    &data.memory,
                                    &mut self.host.commands,
                                    &mut data.debugger.breakpoints,
                                    &mut self.hex_base_addr,
                                    &mut self.memory_selected_addr,
                                    &mut self.hex_edit_buffer,
                                    if !self.inspection.frozen && data.previous_cpu.is_some() {
                                        Some((&data.previous_bytes, data.previous_base))
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
                                        render_disassembly(
                                            ui,
                                            &mut self.inspection.disassembly,
                                            !self.inspection.frozen,
                                            !self.host.status.running && self.host.can_edit(),
                                            &data.cpu,
                                            &data.memory,
                                            &mut data.debugger,
                                            &data.temporal,
                                            &mut self.host.commands,
                                            &mut self.goto_addr_str,
                                            &mut self.disassembly_view_addr,
                                            &mut self.disassembly_selected_addr,
                                            &mut self.active_disasm_edit,
                                            &tokens,
                                        );
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
                                                render_temporal_bar(self, ui, &data.temporal);
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
                                        render_trace_log(self, ui, &data);
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
                            render_temporal_bar(self, ui, &data.temporal);
                            ui.separator();

                            render_disassembly(
                                ui,
                                &mut self.inspection.disassembly,
                                !self.inspection.frozen,
                                !self.host.status.running && self.host.can_edit(),
                                &data.cpu,
                                &data.memory,
                                &mut data.debugger,
                                &data.temporal,
                                &mut self.host.commands,
                                &mut self.goto_addr_str,
                                &mut self.disassembly_view_addr,
                                &mut self.disassembly_selected_addr,
                                &mut self.active_disasm_edit,
                                &tokens,
                            );
                        }
                    }
                });
            if data.debugger.breakpoints != original_breakpoints {
                self.host.queue(HostCommand::SetBreakpoints(
                    data.debugger.breakpoints.clone(),
                ));
            }
            if !self.host.status.running
                && self.host.can_edit()
                && data.previous_base != self.hex_base_addr
            {
                self.host
                    .queue(HostCommand::SetMemoryBase(self.hex_base_addr));
            }
            self.host.data = Some(data);
        }

        if self.view_mode == ViewMode::Developer && !self.inspection.frozen {
            self.inspection.ready = true;
        }
        self.finish_ui_pass(ctx, first_pass);
    }
}
