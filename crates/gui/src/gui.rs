//! Amiga 500 Developer Studio & Interactive Debugger GUI
//!
//! Immediate-mode user interface using eframe and egui for native desktop and WebAssembly.
//! Acts purely as the visual presentation and interaction layer (View), delegating
//! execution orchestration to headless `debugger::DebuggerSession`.

pub mod app;
pub mod app_host;
mod app_lifecycle;
mod configuration_bindings;
#[cfg(not(target_arch = "wasm32"))]
mod configuration_delete;
pub mod configuration_edit;
mod configuration_view;
mod disk_import;
pub mod file_actions;
#[cfg(feature = "game-fps-benchmark")]
pub mod game_fps;
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
mod kickstarts;
pub mod layout;
#[cfg(not(target_arch = "wasm32"))]
mod library_catalog;
#[cfg(not(target_arch = "wasm32"))]
pub mod library_edit;
#[cfg(not(target_arch = "wasm32"))]
mod library_editor;
#[cfg(not(target_arch = "wasm32"))]
pub mod library_refresh;
pub mod playback;
pub mod player;
pub mod player_config;
mod player_drive;
mod player_view;
mod shortcuts;
#[cfg(not(target_arch = "wasm32"))]
mod start_state_storage;
pub mod theme;
#[cfg(not(target_arch = "wasm32"))]
pub mod window_keyboard;
pub mod window_pointer;

pub use app::{EmulatorApp, UserPreferences, ViewMode};
pub use debugger::DebuggerSession;
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

    #[cfg(feature = "game-fps-benchmark")]
    let benchmark_media = game_fps::browser::media().await?;
    let web_options = eframe::WebOptions {
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
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
            Box::new(move |cc| {
                let app = EmulatorApp::new(cc);
                #[cfg(feature = "game-fps-benchmark")]
                let app = game_fps::browser::attach(app, cc, benchmark_media)?;
                Ok(Box::new(app))
            }),
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to start eframe: {:?}", e)))
}
