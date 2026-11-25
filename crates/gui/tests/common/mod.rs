#![allow(dead_code, clippy::unwrap_used, clippy::panic)]

use egui::{Context, RawInput, Ui};
use egui_kittest::{Harness, TestRenderer};
use gui::host_commands::{HostCommand, HostObservation, HostReply, HostValue, InputObservation};
use gui::{EmulatorApp, ViewMode};
use std::cell::Cell;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Holds draw data for headless assertions without submitting it to a renderer.
#[derive(Default)]
pub struct FrameOutput(egui::FullOutput);

impl FrameOutput {
    pub fn append(&mut self, mut newer: Self) {
        self.0.append(std::mem::take(&mut newer.0));
    }

    pub fn into_platform_output(mut self) -> egui::PlatformOutput {
        std::mem::take(&mut self.0.platform_output)
    }
}

impl Deref for FrameOutput {
    type Target = egui::FullOutput;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for FrameOutput {
    fn drop(&mut self) {
        self.0.textures_delta.clear();
    }
}

pub trait TestContext {
    fn run_test_ui(&self, input: RawInput, draw: impl FnMut(&mut Ui)) -> FrameOutput;
}

impl TestContext for Context {
    fn run_test_ui(&self, input: RawInput, draw: impl FnMut(&mut Ui)) -> FrameOutput {
        FrameOutput(self.run_ui(input, draw))
    }
}

/// Gives the application the same full viewport as eframe, without the harness frame's margins.
pub fn show_full_viewport(ui: &mut Ui, app: &mut EmulatorApp) {
    let rect = ui.ctx().content_rect();
    let mut root = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    app.update_ui(&mut root);
}

pub fn texture_updates(
    delta: &egui::TexturesDelta,
) -> impl Iterator<Item = (&egui::TextureId, &egui::epaint::ImageDelta)> {
    delta
        .set
        .iter()
        .flat_map(|(id, updates)| updates.iter().map(move |update| (id, update)))
}

type TextureUpload = (egui::TextureId, egui::epaint::ImageDelta);

thread_local! {
    // The harness draws and the test checks its uploads on the same test thread.
    static LAST_TEXTURE_UPLOADS: Cell<Vec<TextureUpload>> = const { Cell::new(Vec::new()) };
}

pub struct TextureUploads;

impl TextureUploads {
    pub fn capture(delta: &egui::TexturesDelta) {
        LAST_TEXTURE_UPLOADS.set(
            texture_updates(delta)
                .map(|(id, update)| (*id, update.clone()))
                .collect(),
        );
    }

    pub fn take() -> Vec<TextureUpload> {
        LAST_TEXTURE_UPLOADS.take()
    }
}

impl TestRenderer for TextureUploads {
    fn handle_delta(&mut self, delta: &mut egui::TexturesDelta) {
        Self::capture(delta);
        delta.clear();
    }

    fn render(
        &mut self,
        _ctx: &Context,
        _output: &egui::FullOutput,
    ) -> Result<image::RgbaImage, String> {
        Err("Texture upload observer does not render images".into())
    }
}

pub fn dropped_file(path: impl Into<PathBuf>) -> egui::DroppedFileHandle {
    #[derive(Debug)]
    struct DroppedPath(PathBuf);

    impl egui::DroppedFile for DroppedPath {
        fn path(&self) -> &Path {
            &self.0
        }

        fn bytes(&self) -> Result<Vec<u8>, String> {
            std::fs::read(&self.0).map_err(|error| error.to_string())
        }
    }

    let file: Box<dyn egui::DroppedFile + Send + Sync> = Box::new(DroppedPath(path.into()));
    file.into()
}

pub fn settle(app: &mut EmulatorApp, context: &Context) -> FrameOutput {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut combined = FrameOutput::default();
    loop {
        let output = context.run_test_ui(
            RawInput {
                time: Some(context.input(|input| input.time)),
                ..RawInput::default()
            },
            |ui| app.update_ui(ui),
        );
        combined.append(output);
        if app.host.is_idle() && (app.view_mode == ViewMode::ScreenOnly || app.host.data.is_some())
        {
            return combined;
        }
        assert!(
            Instant::now() < deadline,
            "GUI commands did not settle: {:?}",
            app.file_error
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

pub fn settle_harness(harness: &mut Harness<'_, EmulatorApp>) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut settled = false;
    loop {
        harness.step();
        let app = harness.state();
        if app.host.is_idle() && (app.view_mode == ViewMode::ScreenOnly || app.host.data.is_some())
        {
            // egui applies pending host changes, including zoom, at the next pass.
            if settled {
                return;
            }
            settled = true;
        } else {
            settled = false;
        }
        assert!(
            Instant::now() < deadline,
            "GUI commands did not settle: {:?}",
            app.file_error
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn select_library(harness: &mut Harness<'_, EmulatorApp>, root: &std::path::Path) {
    use egui::accesskit::Role;
    use egui_kittest::kittest::Queryable;

    harness
        .get_by_role_and_label(Role::Button, "Configurations")
        .click_accesskit();
    settle_harness(harness);
    // Model the native picker returning a directory to the editor.
    harness
        .state_mut()
        .player
        .editor
        .as_mut()
        .unwrap()
        .library_root = Some(root.to_path_buf());
    harness.get_by_label("Save").click_accesskit();
    settle_harness(harness);
    assert_eq!(harness.state().player.library_root.as_deref(), Some(root));
    assert!(harness.state().player.editor.is_none());
}

pub fn confirm_replacement(app: &mut EmulatorApp) {
    use egui_kittest::kittest::Queryable;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_ui_state(
            |ui, app: &mut EmulatorApp| app.update_ui(ui),
            std::mem::take(app),
        );
    settle_harness(&mut harness);
    harness.get_by_label("Replace session").click_accesskit();
    settle_harness(&mut harness);
    *app = std::mem::take(harness.state_mut());
}

/// Uses the production command boundary while the GUI has no outstanding replies.
pub fn command(app: &mut EmulatorApp, command: HostCommand) -> HostReply {
    settle(app, &Context::default());
    let reply = idle_command(app, command);
    app.host.status = app.host.runtime.as_ref().unwrap().status();
    reply
}

/// Polls the owner after GUI replies have settled, without drawing another frame.
pub fn idle_command(app: &mut EmulatorApp, command: HostCommand) -> HostReply {
    assert!(
        app.host.is_idle(),
        "GUI replies must settle before polling the owner"
    );
    let runtime = app.host.runtime.as_mut().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut pending = command;
    let id = loop {
        match runtime.submit(pending) {
            Ok(id) => break id,
            Err(busy) => pending = busy.0,
        }
        runtime.poll().unwrap();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    };
    loop {
        for reply in runtime.poll().unwrap() {
            if reply.id == id {
                return reply;
            }
        }
        assert!(
            Instant::now() < deadline,
            "Owner did not acknowledge command"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

pub fn observe_input(app: &mut EmulatorApp, key: Option<u8>) -> InputObservation {
    let reply = idle_command(app, HostCommand::Observe(HostObservation::Input { key }));
    app.host.status = reply.status;
    let Ok(HostValue::Input(input)) = reply.value else {
        panic!("Missing owner input observation: {:?}", reply.value);
    };
    input
}

pub fn read_cpu(app: &mut EmulatorApp) -> Box<cpu::CpuState> {
    let reply = idle_command(app, HostCommand::ReadCpu);
    let Ok(HostValue::Cpu(cpu)) = reply.value else {
        panic!("Missing stopped CPU observation: {:?}", reply.value);
    };
    cpu
}

pub fn read_memory(app: &mut EmulatorApp, address: u32, length: usize) -> Vec<u8> {
    let reply = idle_command(app, HostCommand::ReadMemory { address, length });
    let Ok(HostValue::Memory {
        address: actual,
        bytes,
    }) = reply.value
    else {
        panic!("Missing stopped memory observation: {:?}", reply.value);
    };
    assert_eq!(actual, address);
    assert_eq!(bytes.len(), length);
    bytes
}

pub fn inspect(app: &mut EmulatorApp) -> Box<gui::host_inspection::HostInspection> {
    let reply = command(app, HostCommand::Inspect);
    let Ok(HostValue::Inspection(data)) = reply.value else {
        panic!("Missing inspector data");
    };
    data
}

pub fn snapshot_json(app: &mut EmulatorApp) -> String {
    let reply = command(app, HostCommand::SaveSnapshot);
    let Ok(HostValue::Snapshot(json)) = reply.value else {
        panic!("Missing machine snapshot");
    };
    json
}

pub fn snapshot(app: &mut EmulatorApp) -> debugger::A500State {
    *debugger::SaveState::from_json(&snapshot_json(app))
        .unwrap()
        .state
}

pub fn wait_harness(harness: &mut Harness<'_, EmulatorApp>, ready: impl Fn(&EmulatorApp) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        settle_harness(harness);
        if ready(harness.state()) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "GUI did not reach the expected machine state"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

pub fn wait(
    app: &mut EmulatorApp,
    context: &Context,
    ready: impl Fn(&EmulatorApp) -> bool,
) -> FrameOutput {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut combined = FrameOutput::default();
    loop {
        combined.append(settle(app, context));
        if ready(app) {
            return combined;
        }
        assert!(
            Instant::now() < deadline,
            "GUI did not reach the expected machine state"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Applies egui's real full/partial texture protocol without requiring reuploads.
pub struct ScreenCapture {
    size: [usize; 2],
    pub id: Option<egui::TextureId>,
    pub pixels: Vec<egui::Color32>,
    pub options: egui::TextureOptions,
}

impl ScreenCapture {
    pub fn new(size: [usize; 2]) -> Self {
        Self {
            size,
            id: None,
            pixels: Vec::new(),
            options: egui::TextureOptions::default(),
        }
    }

    pub fn apply(&mut self, output: &egui::FullOutput) {
        self.apply_delta(&output.textures_delta);
    }

    pub fn apply_delta(&mut self, textures: &egui::TexturesDelta) {
        for (id, delta) in texture_updates(textures) {
            if delta.pos.is_none() && delta.image.size() == self.size {
                self.id = Some(*id);
            }
            if self.id != Some(*id) {
                continue;
            }
            let egui::ImageData::Color(image) = &delta.image;
            self.options = delta.options;
            if let Some([x, y]) = delta.pos {
                for row in 0..image.size[1] {
                    let start = (y + row) * self.size[0] + x;
                    self.pixels[start..start + image.size[0]].copy_from_slice(
                        &image.pixels[row * image.size[0]..(row + 1) * image.size[0]],
                    );
                }
            } else {
                self.pixels.clone_from(&image.pixels);
            }
        }
    }
}

pub mod keyboard;
