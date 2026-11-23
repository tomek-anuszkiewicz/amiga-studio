#![allow(dead_code, clippy::unwrap_used, clippy::panic)]

use egui::{Context, RawInput};
use egui_kittest::Harness;
use gui::host_commands::{HostCommand, HostReply, HostValue};
use gui::{EmulatorApp, ViewMode};
use std::time::{Duration, Instant};

pub fn settle(app: &mut EmulatorApp, context: &Context) -> egui::FullOutput {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut combined = egui::FullOutput::default();
    loop {
        let output = context.run(
            RawInput {
                time: Some(context.input(|input| input.time)),
                ..RawInput::default()
            },
            |context| app.update_ui(context),
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

pub fn confirm_replacement(app: &mut EmulatorApp) {
    use egui_kittest::kittest::Queryable;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1024.0, 600.0))
        .build_state(
            |ctx, app: &mut EmulatorApp| app.update_ui(ctx),
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
                app.host.status = runtime.status();
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
) -> egui::FullOutput {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut combined = egui::FullOutput::default();
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
        for (id, delta) in &textures.set {
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
