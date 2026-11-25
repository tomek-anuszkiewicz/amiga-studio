//! Read benchmark inputs supplied by the local browser driver.

use super::{focus, GameFps, Media};
use crate::EmulatorApp;
use js_sys::{Promise, Reflect, Uint8Array};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub(crate) async fn media() -> Result<Option<Media>, JsValue> {
    let input = Reflect::get(&js_sys::global(), &JsValue::from_str("amigaGameFpsMedia"))?;
    if input.is_undefined() {
        return Ok(None);
    }
    let promise = input
        .dyn_into::<Promise>()
        .map_err(|_| JsValue::from_str("Expected a promise for Game FPS media"))?;
    let input = JsFuture::from(promise).await?;
    let bytes = |key: &str| {
        Reflect::get(&input, &JsValue::from_str(key))?
            .dyn_into::<Uint8Array>()
            .map(|array| array.to_vec())
            .map_err(|_| JsValue::from_str(&format!("Missing Game FPS {key} bytes")))
    };
    let rom = bytes("rom")?;
    let disk = bytes("disk")?;
    let selection = crate::disk_import::decode_disk_source("Mixed Scene", "mixed_scene.adf", disk);
    Media::checked(rom, selection)
        .map(Some)
        .map_err(|error| JsValue::from_str(&error))
}

pub(crate) fn attach(
    mut app: EmulatorApp,
    cc: &eframe::CreationContext<'_>,
    media: Option<Media>,
) -> Result<EmulatorApp, String> {
    if let Some(media) = media {
        let focus = Focus::register(&cc.egui_ctx)
            .map_err(|error| format!("Could not watch Game window focus: {error:?}"))?;
        GameFps::new(Ok(media), "Glow WebGL".into(), focus).attach(&mut app);
    }
    Ok(app)
}

pub(super) fn publish(name: &str, value: JsValue) {
    let _ = Reflect::set(&js_sys::global(), &JsValue::from_str(name), &value);
}

/// Remember a brief blur or hidden tab even if it returns before the next GUI pass.
#[derive(Debug)]
pub(super) struct Focus {
    window: web_sys::Window,
    document: web_sys::Document,
    listener: Closure<dyn FnMut()>,
}

impl Focus {
    fn register(ctx: &egui::Context) -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No browser window"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("No browser document"))?;
        let ctx = ctx.clone();
        let focus = Self {
            window,
            document,
            listener: Closure::new(move || {
                if !crate::host_runtime::host_window_active() {
                    focus::record_focus_loss(&ctx);
                }
            }),
        };
        focus
            .window
            .add_event_listener_with_callback("blur", focus.listener.as_ref().unchecked_ref())?;
        focus.document.add_event_listener_with_callback(
            "visibilitychange",
            focus.listener.as_ref().unchecked_ref(),
        )?;
        Ok(focus)
    }
}

impl Drop for Focus {
    fn drop(&mut self) {
        let callback = self.listener.as_ref().unchecked_ref();
        let _ = self
            .window
            .remove_event_listener_with_callback("blur", callback);
        let _ = self
            .document
            .remove_event_listener_with_callback("visibilitychange", callback);
    }
}
