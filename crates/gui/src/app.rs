//! Central EmulatorApp Orchestrator (Pure View & Presentation Layer)
//!
//! Owns the UI state, themes, and views, delegating machine execution orchestration
//! entirely to headless `debugger::DebuggerSession`.

use serde::{Deserialize, Serialize};

use crate::app_host::AppHost;
use crate::host_commands::{HistoryCommand, HostCommand};
use crate::layout::left_dock::disassembly::DisasmEditState;
use crate::layout::left_dock::registers::EditRegister;
use crate::layout::main_viewport::amiga_screen::{ScreenScale, ScreenTexture};
use crate::layout::right_dock::breakpoints_panel::BreakpointFormState;
use crate::layout::right_dock::memory_search::MemorySearchState;
use crate::theme::AppTheme;
use debugger::temporal::DEFAULT_TEMPORAL_CAPACITY;
use debugger::DebuggerSession;

mod frame;
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
        #[cfg(target_arch = "wasm32")]
        if let Some(storage) = cc.storage {
            if let Some(mut catalog) = eframe::get_value::<crate::player_config::ConfigurationCatalog>(
                storage,
                crate::player_config::CONFIG_STORAGE_KEY,
            ) {
                app.player.settings_dirty |= catalog.introduce_keyboard_presets();
                app.player.catalog = catalog;
            }
        }
        app.player.screen = if cfg!(target_arch = "wasm32") {
            crate::player::PlayerScreen::Setup
        } else {
            crate::player::PlayerScreen::Library
        };
        app.set_view_mode(ViewMode::ScreenOnly);
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(storage) = cc.storage {
                app.restore_native_player_settings(storage);
            }
            app.reload_library_catalog();
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
        self.host
            .queue(HostCommand::History(HistoryCommand::Capacity(
                prefs.temporal_capacity,
            )));
    }
}
