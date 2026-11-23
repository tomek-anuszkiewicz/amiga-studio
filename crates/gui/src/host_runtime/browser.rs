//! Browser worker bootstrap and DOM input callbacks.

use super::{
    HostInputEvent, HostInputSender, HostRuntimeError, MachineOwner, Ordering, SharedHost,
    WorkerLoop, WorkerWait, WEB_STOPPED, WEB_WAIT_FOR_EVENT,
};
use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
static WEB_OWNER: Mutex<Option<WorkerLoop>> = Mutex::new(None);

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(module = "/host_bridge.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn start_machine_worker(
        module: wasm_bindgen::JsValue,
        memory: wasm_bindgen::JsValue,
        notify: wasm_bindgen::JsValue,
        finished: wasm_bindgen::JsValue,
        input: wasm_bindgen::JsValue,
    ) -> Result<web_sys::Worker, wasm_bindgen::JsValue>;
    pub(crate) fn take_pointer_capture_loss() -> bool;
    pub(crate) fn host_window_active() -> bool;
    pub(crate) fn host_release_pending() -> bool;
    pub(super) fn discard_buffered_host_controls() -> String;
    pub(super) fn set_host_input_enabled(
        mouse: bool,
        gamepads: bool,
        interval_ms: f64,
        generation: u64,
    );
    fn host_now() -> f64;
    fn notify_gui();
    fn report_machine_failure(message: &str);
}

/// Installed once by the browser worker before it enters any machine work.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn host_worker_start() {
    std::panic::set_hook(Box::new(|info| {
        report_machine_failure(&info.to_string());
    }));
}

#[cfg(target_arch = "wasm32")]
pub(super) fn start_web(
    owner: MachineOwner,
    shared: Arc<SharedHost>,
    context: egui::Context,
) -> Result<web_sys::Worker, HostRuntimeError> {
    use wasm_bindgen::closure::Closure;
    const BIT_MOUSE_LEFT: u32 = 1;
    const BIT_MOUSE_RIGHT: u32 = 1 << 1;

    let mut slot = WEB_OWNER
        .try_lock()
        .map_err(|_| HostRuntimeError::StartupFailed)?;
    if slot.is_some() {
        return Err(HostRuntimeError::StartupFailed);
    }
    let notify_context = context.clone();
    let notify = Closure::<dyn FnMut()>::new(move || notify_context.request_repaint());
    let finished_shared = shared.clone();
    let sender = HostInputSender {
        shared: shared.clone(),
        repaint: context.clone(),
    };
    let input = Closure::<dyn FnMut(String, String, i32, i32, u32, u64) -> bool>::new(
        move |kind: String, code: String, dx, dy, bits, generation| {
            if kind == "pads" {
                return sender.browser_gamepads(&code, generation);
            }
            if kind == "key" {
                return sender.browser_key(&code, generation, bits != 0);
            }
            if kind == "primary-press" {
                let scale = sender.repaint.zoom_factor();
                crate::keyboard_input::receive_pointer_press(
                    &sender.repaint,
                    egui::pos2(dx as f32 / scale, dy as f32 / scale),
                );
                return true;
            }
            if kind == "focus" {
                sender.browser_focus(generation, bits != 0);
                return true;
            }
            let event = match kind.as_str() {
                "motion" => HostInputEvent::MouseMotion { port: 0, dx, dy },
                "mouse" => HostInputEvent::MouseButtons {
                    port: 0,
                    left: bits & BIT_MOUSE_LEFT != 0,
                    right: bits & BIT_MOUSE_RIGHT != 0,
                    middle: false,
                },
                "release" => HostInputEvent::ReleaseAll,
                "mouse-release" => HostInputEvent::ReleaseMouse { port: 0 },
                _ => return true,
            };
            sender.try_host_event(event, generation)
        },
    );
    let finished =
        Closure::<dyn FnMut(bool, String)>::new(move |success: bool, message: String| {
            if !success {
                finished_shared.failure.record(message);
            }
            finished_shared.stopped.store(true, Ordering::Release);
            context.request_repaint();
        });
    *slot = Some(WorkerLoop {
        owner,
        shared,
        pending_reply: None,
    });
    drop(slot);
    // JavaScript owns the callbacks until it receives the worker's final message.
    match start_machine_worker(
        wasm_bindgen::module(),
        wasm_bindgen::memory(),
        notify.into_js_value(),
        finished.into_js_value(),
        input.into_js_value(),
    ) {
        Ok(worker) => Ok(worker),
        Err(_) => {
            if let Ok(mut slot) = WEB_OWNER.try_lock() {
                *slot = None;
            }
            Err(HostRuntimeError::StartupFailed)
        }
    }
}

/// Called only by the worker. Negative delays mean event wait or final teardown.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn host_worker_tick() -> Result<f64, wasm_bindgen::JsValue> {
    let mut slot = WEB_OWNER
        .try_lock()
        .map_err(|_| wasm_bindgen::JsValue::from_str("Machine owner is busy"))?;
    let worker = slot
        .as_mut()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("Machine owner is missing"))?;
    let started = host_now();
    let (notify, wait) = worker.tick(started, || host_now() - started);
    if notify {
        notify_gui();
    }
    match wait {
        WorkerWait::Event => Ok(WEB_WAIT_FOR_EVENT),
        WorkerWait::Delay(delay) => Ok(delay.as_secs_f64() * 1000.0),
        WorkerWait::Stop => {
            *slot = None;
            Ok(WEB_STOPPED)
        }
    }
}
