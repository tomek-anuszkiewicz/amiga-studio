//! Browser file selection, recent directory references and snapshot downloads.

use super::{EmulatorApp, FileAction};
use crate::disk_import::{
    check_disk_source_size, decode_disk_source, DiskSelection, MAX_DISK_CHOICES,
};
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
        multiple: bool,
    ) -> Result<Promise, JsValue>;
}

struct SelectedFile {
    name: String,
    data: Vec<u8>,
    handle: JsValue,
}

enum SelectedAssets {
    Single(SelectedFile),
    Disks(DiskSelection),
}

type ReadResult = Result<Option<SelectedAssets>, JsValue>;
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
            Self::SetupRom => "setup-rom",
            Self::SetupDisk(_) => "setup-disk",
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
        let multiple = matches!(action, FileAction::SetupDisk(_));
        let selection = pick_file(
            action.key(),
            action.title(),
            &extensions,
            recent_index,
            multiple,
        )?;
        let future = Box::pin(async move {
            let selected = JsFuture::from(selection).await?;
            if selected.is_null() {
                return Ok(None);
            }
            let selected = Array::from(&selected);
            if multiple {
                return Ok(Some(SelectedAssets::Disks(
                    decode_selected_disks(selected).await,
                )));
            }
            let file: web_sys::File = selected.get(0).dyn_into()?;
            let buffer = JsFuture::from(file.array_buffer()).await?;
            Ok(Some(SelectedAssets::Single(SelectedFile {
                name: file.name(),
                data: Uint8Array::new(&buffer).to_vec(),
                handle: selected.get(1),
            })))
        });
        Ok(Self { action, future })
    }

    fn from_drop(file: egui::DroppedFileHandle) -> Self {
        let future = Box::pin(async move {
            let name = file.path().to_string_lossy().into_owned();
            let data = file
                .bytes_async()
                .await
                .map_err(|error| JsValue::from_str(&error))?;
            Ok(Some(SelectedAssets::Single(SelectedFile {
                name,
                data,
                handle: JsValue::NULL,
            })))
        });
        Self {
            action: FileAction::Binary,
            future,
        }
    }
}

async fn decode_selected_disks(sources: Array) -> DiskSelection {
    let mut selection = DiskSelection::default();
    for source in sources.iter() {
        if selection.images.len() >= MAX_DISK_CHOICES {
            selection.issues.push(format!(
                "Disk selection exceeds the {MAX_DISK_CHOICES}-image limit"
            ));
            break;
        }
        let file = match browser_disk_file(source).await {
            Ok(file) => file,
            Err(error) => {
                selection
                    .issues
                    .push(format!("Could not read selected disk: {error:?}"));
                continue;
            }
        };
        let name = file.name();
        if let Err(error) = check_disk_source_size(&name, file.size() as u64) {
            selection.issues.push(format!("{name}: {error}"));
            continue;
        }
        match JsFuture::from(file.array_buffer()).await {
            Ok(buffer) => {
                let decoded = decode_disk_source(&name, &name, Uint8Array::new(&buffer).to_vec());
                selection.issues.extend(decoded.issues);
                let remaining = MAX_DISK_CHOICES - selection.images.len();
                if decoded.images.len() > remaining {
                    selection.issues.push(format!(
                        "{name}: Disk selection exceeds the {MAX_DISK_CHOICES}-image limit"
                    ));
                }
                selection
                    .images
                    .extend(decoded.images.into_iter().take(remaining));
            }
            Err(error) => selection
                .issues
                .push(format!("{name}: Could not read selected file: {error:?}")),
        }
    }
    selection
}

async fn browser_disk_file(source: JsValue) -> Result<web_sys::File, JsValue> {
    if source.is_instance_of::<web_sys::File>() {
        return source.dyn_into();
    }
    let handle: web_sys::FileSystemFileHandle = source.dyn_into()?;
    JsFuture::from(handle.get_file()).await?.dyn_into()
}

impl EmulatorApp {
    pub(crate) fn accept_dropped_file(&mut self, file: egui::DroppedFileHandle) {
        self.browser_file_task = Some(BrowserFileTask::from_drop(file));
    }

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
        self.host.queue_effect(
            crate::host_commands::HostCommand::SaveSnapshot,
            crate::app_host::HostEffect::File(super::FileCompletion::Download),
        );
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
                    Ok(Some(SelectedAssets::Single(file))) => {
                        self.accept_file_data(action, file.name, file.data, file.handle)
                    }
                    Ok(Some(SelectedAssets::Disks(selection))) => {
                        if let FileAction::SetupDisk(drive) = action {
                            self.accept_disk_selection(drive, selection);
                        }
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

pub(super) fn download_state(json: &str) -> Result<(), JsValue> {
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
