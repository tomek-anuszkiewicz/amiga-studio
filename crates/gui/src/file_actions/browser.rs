//! Browser-selected files and explicit snapshot downloads, without host paths.

use super::{EmulatorApp, FileAction, TOAST_DURATION_FRAMES};
use js_sys::{Array, Promise, Uint8Array};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
    time::Duration,
};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Document, Event, HtmlAnchorElement, HtmlInputElement};

type ReadResult = Result<Option<(String, Vec<u8>)>, JsValue>;
const FILE_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub struct BrowserFileTask {
    action: FileAction,
    input: HtmlInputElement,
    // Keep the host event handler alive only while its file request is pending.
    _selection: Option<Closure<dyn FnMut(Event)>>,
    future: Pin<Box<dyn Future<Output = ReadResult>>>,
}

impl std::fmt::Debug for BrowserFileTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BrowserFileTask")
            .field(&self.action)
            .finish()
    }
}

impl Drop for BrowserFileTask {
    fn drop(&mut self) {
        self.input.set_onchange(None);
        self.input.set_oncancel(None);
        self.input.remove();
    }
}

fn document() -> Result<Document, JsValue> {
    web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("Browser document is unavailable."))
}

impl BrowserFileTask {
    fn new(action: FileAction) -> Result<Self, JsValue> {
        let document = document()?;
        let input: HtmlInputElement = document.create_element("input")?.dyn_into()?;
        input.set_type("file");
        input.set_title(action.title());
        input.set_hidden(true);
        input.set_accept(
            &action
                .extensions()
                .iter()
                .map(|ext| format!(".{ext}"))
                .collect::<Vec<_>>()
                .join(","),
        );
        document
            .body()
            .ok_or_else(|| JsValue::from_str("Browser body is unavailable."))?
            .append_child(&input)?;
        let mut selection = None;
        let promise = Promise::new(&mut |resolve, _reject| {
            let handler = Closure::wrap(Box::new(move |event: Event| {
                let selected = JsValue::from_bool(event.type_() != "cancel");
                let _ = resolve.call1(&JsValue::UNDEFINED, &selected);
            }) as Box<dyn FnMut(Event)>);
            input.set_onchange(Some(handler.as_ref().unchecked_ref()));
            input.set_oncancel(Some(handler.as_ref().unchecked_ref()));
            selection = Some(handler);
        });
        let selected_input = input.clone();
        let future = Box::pin(async move {
            if JsFuture::from(promise).await?.as_bool() != Some(true) {
                return Ok(None);
            }
            let Some(file) = selected_input.files().and_then(|files| files.get(0)) else {
                return Ok(None);
            };
            let buffer = JsFuture::from(file.array_buffer()).await?;
            Ok(Some((file.name(), Uint8Array::new(&buffer).to_vec())))
        });
        input.click();
        Ok(Self {
            action,
            input,
            _selection: selection,
            future,
        })
    }
}

impl EmulatorApp {
    pub(super) fn open_file_dialog(&mut self, action: FileAction) {
        match BrowserFileTask::new(action) {
            Ok(task) => self.browser_file_task = Some(task),
            Err(error) => self.file_error = Some(format!("Could not open file picker: {error:?}")),
        }
    }

    pub(crate) fn open_save_state_dialog(&mut self) {
        let result = self
            .session
            .save_state_to_json()
            .map_err(|error| error.to_string())
            .and_then(|json| {
                download_state(&json)
                    .map_err(|error| format!("Could not download state: {error:?}"))
            });
        match result {
            Ok(()) => {
                self.toast_message =
                    Some(("State download requested.".into(), TOAST_DURATION_FRAMES))
            }
            Err(error) => self.file_error = Some(error),
        }
    }

    // Host I/O completion is pulled here; it never calls into the machine asynchronously.
    pub(crate) fn poll_file_dialog(&mut self, ctx: &egui::Context) {
        let Some(task) = &mut self.browser_file_task else {
            return;
        };
        match task
            .future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(result) => {
                let action = task.action;
                self.browser_file_task = None;
                match result {
                    Ok(Some((name, data))) => self.accept_file_data(action, name, data),
                    Ok(None) => {}
                    Err(error) => {
                        self.file_error = Some(format!("Could not read selected file: {error:?}"))
                    }
                }
                ctx.request_repaint();
            }
            Poll::Pending => ctx.request_repaint_after(FILE_POLL_INTERVAL),
        }
    }
}

fn download_state(json: &str) -> Result<(), JsValue> {
    let document = document()?;
    let link: HtmlAnchorElement = document.create_element("a")?.dyn_into()?;
    let parts = Array::of1(&JsValue::from_str(json));
    let blob = web_sys::Blob::new_with_str_sequence(&parts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    link.set_href(&url);
    link.set_download("amiga-state.json");
    link.click();
    web_sys::Url::revoke_object_url(&url)
}
