//! Amiga 500 Developer Studio & Interactive Debugger GUI
//!
//! Immediate-mode user interface using eframe and egui for native desktop and WebAssembly.
//! Acts purely as the visual presentation and interaction layer (View), delegating
//! execution orchestration to headless `debugger::DebuggerSession`.

pub mod app;
pub mod app_host;
mod app_lifecycle;
mod configuration_view;
pub mod file_actions;
pub mod game_library;
mod host_clock;
pub mod host_commands;
mod host_failure;
#[cfg(not(target_arch = "wasm32"))]
mod host_gamepads;
pub mod host_input;
pub mod host_inspection;
pub mod host_keys;
pub mod host_owner;
pub mod host_runtime;
pub mod host_video;
pub mod input_mapping;
mod inspection;
pub mod keyboard_input;
pub mod layout;
#[cfg(not(target_arch = "wasm32"))]
pub mod library_edit;
#[cfg(not(target_arch = "wasm32"))]
mod library_editor;
pub mod playback;
pub mod player;
pub mod player_config;
mod player_drive;
mod player_view;
mod shortcuts;
pub mod theme;
pub mod window_pointer;
#[cfg(not(target_arch = "wasm32"))]
pub mod window_keyboard;

pub use app::{EmulatorApp, UserPreferences, ViewMode};
pub use debugger::DebuggerSession;
pub use file_actions::BinarySource;
pub use layout::left_dock::disassembly::DisasmEditState;
pub use layout::left_dock::registers::EditRegister;
pub use layout::main_viewport::amiga_screen::ScreenScale;
pub use theme::AppTheme;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let web_options = eframe::WebOptions::default();
    let runner = eframe::WebRunner::new();

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No global `window` exists"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("Should have a document on window"))?;
    let element = document
        .get_element_by_id("amiga_canvas")
        .ok_or_else(|| JsValue::from_str("Should have #amiga_canvas on document"))?;
    let canvas = element
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("#amiga_canvas is not an HtmlCanvasElement"))?;

    runner
        .start(
            canvas,
            web_options,
            Box::new(|cc| Ok(Box::new(EmulatorApp::new(cc)))),
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to start eframe: {:?}", e)))
}
