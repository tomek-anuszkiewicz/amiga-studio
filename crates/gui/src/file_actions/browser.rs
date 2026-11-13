//! Browser file selection, recent directory references and snapshot downloads.

use super::{EmulatorApp, FileAction, TOAST_DURATION_FRAMES};
use js_sys::{Array, Promise, Uint8Array};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
    time::Duration,
};
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Document, HtmlAnchorElement};

#[wasm_bindgen(module = "/src/file_actions/browser_files.js")]
extern "C" {
    fn restore_history() -> Promise;
    fn recent_names(kind: &str) -> Array;
    fn remember_file(kind: &str, name: &str, handle: &JsValue) -> Promise;
    #[wasm_bindgen(catch)]
    fn pick_file(
        kind: &str,
        title: &str,
        extensions: &Array,
        recent_index: i32,
    ) -> Result<Promise, JsValue>;
}

struct SelectedFile {
    name: String,
    data: Vec<u8>,
    handle: JsValue,
}

type ReadResult = Result<Option<SelectedFile>, JsValue>;
const FILE_POLL_INTERVAL: Duration = Duration::from_millis(50);
const NO_RECENT_FILE_INDEX: i32 = -1;

pub struct BrowserFileTask {
    action: FileAction,
    future: Pin<Box<dyn Future<Output = ReadResult>>>,
}

impl std::fmt::Debug for BrowserFileTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BrowserFileTask")
            .field(&self.action)
            .finish()
    }
}

#[derive(Default)]
pub struct BrowserHistory {
    pending: Vec<JsFuture>,
}

impl std::fmt::Debug for BrowserHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserHistory")
            .field("pending", &self.pending.len())
            .finish()
    }
}

impl BrowserHistory {
    pub fn restore(&mut self) {
        self.pending.push(JsFuture::from(restore_history()));
    }

    pub(super) fn remember(&mut self, action: FileAction, name: &str, handle: &JsValue) {
        self.pending
            .push(JsFuture::from(remember_file(action.key(), name, handle)));
    }

    pub fn names(&self, binary: bool) -> Vec<String> {
        let action = if binary {
            FileAction::Binary
        } else {
            FileAction::Rom
        };
        recent_names(action.key())
            .iter()
            .filter_map(|value| value.as_string())
            .collect()
    }

    fn poll(&mut self, ctx: &egui::Context) -> Option<String> {
        let mut error = None;
        self.pending.retain_mut(|future| {
            match Pin::new(future).poll(&mut Context::from_waker(Waker::noop())) {
                Poll::Ready(result) => {
                    if let Err(reason) = result {
                        error = Some(format!("Could not persist recent files: {reason:?}"));
                    }
                    ctx.request_repaint();
                    false
                }
                Poll::Pending => true,
            }
        });
        if !self.pending.is_empty() {
            ctx.request_repaint_after(FILE_POLL_INTERVAL);
        }
        error
    }
}

impl FileAction {
    fn key(self) -> &'static str {
        match self {
            Self::Binary => "binary",
            Self::Rom => "rom",
            Self::State => "state",
        }
    }
}

fn document() -> Result<Document, JsValue> {
    web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("Browser document is unavailable."))
}

impl BrowserFileTask {
    fn new(action: FileAction, recent_index: i32) -> Result<Self, JsValue> {
        let extensions = action
            .extensions()
            .iter()
            .map(|extension| JsValue::from_str(&format!(".{extension}")))
            .collect();
        let selection = pick_file(action.key(), action.title(), &extensions, recent_index)?;
        let future = Box::pin(async move {
            let selected = JsFuture::from(selection).await?;
            if selected.is_null() {
                return Ok(None);
            }
            let selected = Array::from(&selected);
            let file: web_sys::File = selected.get(0).dyn_into()?;
            let buffer = JsFuture::from(file.array_buffer()).await?;
            Ok(Some(SelectedFile {
                name: file.name(),
                data: Uint8Array::new(&buffer).to_vec(),
                handle: selected.get(1),
            }))
        });
        Ok(Self { action, future })
    }
}

impl EmulatorApp {
    pub(super) fn open_file_dialog(&mut self, action: FileAction) {
        self.open_browser_file_dialog(action, NO_RECENT_FILE_INDEX);
    }

    pub(crate) fn open_recent_browser_file(&mut self, binary: bool, index: usize) {
        let action = if binary {
            FileAction::Binary
        } else {
            FileAction::Rom
        };
        self.open_browser_file_dialog(action, index as i32);
    }

    fn open_browser_file_dialog(&mut self, action: FileAction, recent_index: i32) {
        match BrowserFileTask::new(action, recent_index) {
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
        if let Some(error) = self.browser_history.poll(ctx) {
            self.file_error = Some(error);
        }
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
                    Ok(Some(file)) => {
                        self.accept_file_data(action, file.name, file.data, file.handle)
                    }
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
