//! Amiga 500 Developer Studio & Interactive Debugger GUI
//!
//! Immediate-mode user interface using eframe and egui for native desktop and WebAssembly.
//! Acts purely as the visual presentation and interaction layer (View), delegating
//! execution orchestration to headless `debugger::DebuggerSession`.

pub mod app;
pub mod layout;
pub mod theme;

pub use app::{EmulatorApp, ViewMode};
pub use debugger::DebuggerSession;
pub use layout::left_dock::disassembly::DisasmEditState;
pub use layout::left_dock::registers::EditRegister;
pub use theme::AppTheme;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
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
