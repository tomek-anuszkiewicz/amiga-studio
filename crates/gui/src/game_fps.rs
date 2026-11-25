//! Opt-in production Game launch and drawing measurement for Mixed Scene.

#[cfg(target_arch = "wasm32")]
pub(crate) mod browser;
mod drawing;
mod progress;
mod submissions;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use drawing::record_focus_loss;
pub(crate) use drawing::{prepare_draw, record_draw};

use crate::app_host::HostEffect;
use crate::disk_import::{DiskImage, DiskSelection};
use crate::host_commands::{HostCommand, HostObservation, HostReply, HostValue};
use crate::player::PlayerScreen;
use crate::{EmulatorApp, ScreenScale, ViewMode};
#[cfg(not(target_arch = "wasm32"))]
use progress::StartupFocus;
use progress::{validate_session, Boot, Progress};
use serde::Serialize;
use std::time::Duration;
use submissions::Sample;

const PAYLOAD: &[u8] = include_bytes!("../../../tests/mixed_scene/program.bin");
const ENTRY_ADDRESS: u32 = 0x1000;
const SIGNATURE_BYTES: usize = 16;
const STATS_ADDRESS: u32 = 0x8000;
const WORK_OFFSET: usize = 4;
const FAULT_OFFSET: usize = 36;
const STATS_LENGTH: usize = FAULT_OFFSET + 4;

pub(crate) fn observe_boot(memory: &physical_memory::PhysicalMemory) -> HostValue {
    // Debug reads observe physical memory without touching guest bus state.
    let signature_matches =
        PAYLOAD[..SIGNATURE_BYTES]
            .iter()
            .enumerate()
            .all(|(offset, &expected)| {
                memory.read_byte_debug(ENTRY_ADDRESS + offset as u32) == expected
            });
    let read = |offset| {
        u32::from_be_bytes(std::array::from_fn(|index| {
            memory.read_byte_debug(STATS_ADDRESS + (offset + index) as u32)
        }))
    };
    HostValue::MixedSceneBoot {
        signature_matches,
        fields: read(0),
        work_loops: read(WORK_OFFSET),
        fault: read(FAULT_OFFSET),
    }
}

#[derive(Debug)]
pub(crate) struct Media {
    rom: Vec<u8>,
    disk: DiskImage,
}

impl Media {
    fn checked(rom: Vec<u8>, mut disks: DiskSelection) -> Result<Self, String> {
        if !disks.issues.is_empty() {
            return Err("Could not select a readable standard 880 KiB Mixed Scene ADF".into());
        }
        let disk = disks.images.pop().ok_or("Missing Mixed Scene ADF")?;
        Ok(Self { rom, disk })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn native(rom: &std::path::Path, adf: &std::path::Path) -> Result<Self, String> {
        if !adf
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(crate::disk_import::is_adf)
        {
            return Err("Select the Mixed Scene ADF file".into());
        }
        let rom =
            std::fs::read(rom).map_err(|error| format!("Could not read Kickstart: {error}"))?;
        Self::checked(rom, crate::disk_import::read_disk_path(adf))
    }
}

#[derive(Debug)]
enum Stage {
    WaitingForFocus,
    Launching(u64),
    Boot(Boot),
    Sampling,
    Pausing,
    ReadingAfter,
    Finished,
}

#[derive(Debug, PartialEq, Serialize)]
struct Display {
    viewport_points: [f32; 2],
    pixels_per_point: f32,
    screen_scale: ScreenScale,
}

impl Display {
    fn current(app: &EmulatorApp, ctx: &egui::Context) -> Self {
        let rect = ctx.viewport_rect();
        Self {
            viewport_points: [rect.width(), rect.height()],
            pixels_per_point: ctx.pixels_per_point(),
            screen_scale: app.screen_scale,
        }
    }
}

#[derive(Debug)]
pub(crate) struct GameFps {
    #[cfg(not(target_arch = "wasm32"))]
    result_path: std::path::PathBuf,
    #[cfg(target_arch = "wasm32")]
    _browser_focus: browser::Focus,
    #[cfg(not(target_arch = "wasm32"))]
    startup_focus: StartupFocus,
    renderer: String,
    stage: Stage,
    media: Option<Media>,
    generation: Option<u64>,
    progress_pending: bool,
    boot: Option<(Progress, Progress)>,
    after: Option<Progress>,
    sample: Option<Sample>,
    display: Option<Display>,
    error: Option<String>,
}

impl GameFps {
    fn new(
        media: Result<Media, String>,
        renderer: String,
        #[cfg(not(target_arch = "wasm32"))] result_path: std::path::PathBuf,
        #[cfg(target_arch = "wasm32")] browser_focus: browser::Focus,
    ) -> Self {
        let (media, error) = match media {
            Ok(media) => (Some(media), None),
            Err(error) => (None, Some(error)),
        };
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            result_path,
            #[cfg(target_arch = "wasm32")]
            _browser_focus: browser_focus,
            #[cfg(not(target_arch = "wasm32"))]
            startup_focus: StartupFocus::default(),
            renderer,
            stage: Stage::WaitingForFocus,
            media,
            generation: None,
            progress_pending: false,
            boot: None,
            after: None,
            sample: None,
            display: None,
            error,
        }
    }

    fn attach(self, app: &mut EmulatorApp, ctx: &egui::Context) {
        drawing::install(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        drawing::watch_focus(ctx);
        app.game_fps = Some(Box::new(self));
    }

    pub(crate) fn sampling(&self) -> bool {
        matches!(self.stage, Stage::Sampling)
    }

    pub(crate) fn accept_progress(&mut self, reply: HostReply) {
        if let Err(error) = self.record_progress(reply) {
            self.error = Some(error);
        }
    }

    fn record_progress(&mut self, reply: HostReply) -> Result<(), String> {
        let status = reply.status;
        validate_session(self.generation, status.generation, true)?;
        if !self.progress_pending {
            return Err("Unexpected guest observation reply".into());
        }
        self.progress_pending = false;
        let progress = |fields, work_loops, fault| Progress {
            fields,
            work_loops,
            fault,
            cck: status.cck,
            field_number: status.field_number,
        };
        match (&mut self.stage, reply.value?) {
            (
                Stage::Boot(boot),
                HostValue::MixedSceneBoot {
                    signature_matches,
                    fields,
                    work_loops,
                    fault,
                },
            ) if status.running && status.continuous_run && status.speed == 1.0 => {
                self.boot = boot.observe(signature_matches, progress(fields, work_loops, fault))?;
            }
            (
                Stage::ReadingAfter,
                HostValue::Memory {
                    address: STATS_ADDRESS,
                    bytes,
                },
            ) if !status.running && bytes.len() == STATS_LENGTH => {
                let read = |offset| u32::from_be_bytes(std::array::from_fn(|i| bytes[offset + i]));
                self.after = Some(progress(read(0), read(WORK_OFFSET), read(FAULT_OFFSET)));
            }
            _ => return Err("Unexpected guest observation reply".into()),
        }
        Ok(())
    }

    fn start_manual(app: &mut EmulatorApp, media: Media) -> Result<(), String> {
        if app.player.screen != PlayerScreen::Setup {
            app.navigate_player(PlayerScreen::Setup);
        }
        let launch = &app
            .player
            .catalog
            .selected()
            .ok_or("Select the standard A500 configuration")?
            .launch;
        if app.player.screen != PlayerScreen::Setup
            || launch.preset != config::A500Preset::Standard1Mb
            || launch.video != config::VideoStandard::Pal
            || launch.drives != [true, false, false, false]
        {
            return Err("Game FPS requires Manual Setup with a standard PAL 1 MB A500 and only DF0 connected".into());
        }
        app.select_setup_rom("Kickstart.rom".into(), media.rom);
        app.select_setup_disk(0, media.disk);
        if let Some(error) = &app.file_error {
            return Err(error.clone());
        }
        if !app.manual_ready() {
            return Err("Select valid ROM and DF0 media in Manual Setup".into());
        }
        app.start_manual();
        Ok(())
    }

    fn advance(&mut self, app: &mut EmulatorApp, ctx: &egui::Context) -> Result<(), String> {
        if let Some(error) = self.error.take().or_else(|| app.file_error.clone()) {
            return Err(error);
        }
        let status = app.host.status;
        if app.host.error.is_some() || status.shutdown || status.halted {
            return Err("Machine owner stopped unexpectedly".into());
        }
        let focused = drawing::focused(ctx);
        let ready_to_launch = if matches!(self.stage, Stage::WaitingForFocus) {
            #[cfg(not(target_arch = "wasm32"))]
            {
                if !focused {
                    // Clear the loss reported by this pass so focus can settle again.
                    drawing::watch_focus(ctx);
                }
                self.startup_focus
                    .observe(focused, std::time::Instant::now())
            }
            #[cfg(target_arch = "wasm32")]
            {
                focused
            }
        } else {
            validate_session(self.generation, status.generation, focused)?;
            false
        };

        match &self.stage {
            Stage::WaitingForFocus if ready_to_launch => {
                #[cfg(not(target_arch = "wasm32"))]
                eprintln!("The Game window gained focus. Starting the benchmark session.");
                #[cfg(target_arch = "wasm32")]
                {
                    drawing::watch_focus(ctx);
                    browser::publish("amigaGameFpsArmed", wasm_bindgen::JsValue::TRUE);
                }
                let media = self.media.take().ok_or("Missing Game FPS media")?;
                Self::start_manual(app, media)?;
                self.stage = Stage::Launching(status.generation);
            }
            Stage::Launching(previous) if !app.player.busy => {
                if status.generation != previous.wrapping_add(1)
                    || !status.drives[0].inserted
                    || app.player.mounted[0].is_none()
                    || app.player.mounted[0] != app.player.setup_mounted[0]
                {
                    return Err("Manual Setup did not launch a new Game session with the selected disk in DF0".into());
                }
                check_running_game(app)?;
                self.generation = Some(status.generation);
                self.stage = Stage::Boot(Boot::default());
            }
            Stage::Boot(_) => {
                check_running_game(app)?;
                if let Some((_, before)) = self.boot {
                    self.display = Some(Display::current(app, ctx));
                    drawing::arm(ctx, status.generation, before.field_number);
                    self.stage = Stage::Sampling;
                } else if !self.progress_pending {
                    self.request_progress(
                        app,
                        HostCommand::Observe(HostObservation::MixedSceneBoot),
                    );
                }
            }
            Stage::Sampling => {
                check_running_game(app)?;
                if self.display.as_ref() != Some(&Display::current(app, ctx)) {
                    return Err("Game viewport or scale changed during sampling".into());
                }
                if let Some(sample) = drawing::sample(ctx) {
                    self.sample = Some(sample);
                    app.host.queue(HostCommand::Pause);
                    self.stage = Stage::Pausing;
                }
            }
            Stage::Pausing if !status.running && app.host.can_edit() => {
                self.request_progress(
                    app,
                    HostCommand::ReadMemory {
                        address: STATS_ADDRESS,
                        length: STATS_LENGTH,
                    },
                );
                self.stage = Stage::ReadingAfter;
            }
            Stage::ReadingAfter if self.after.is_some() => {
                let (_, before) = self.boot.ok_or("Missing confirmed boot progress")?;
                let after = self.after.ok_or("Missing final guest progress")?;
                if !after.advances(before) {
                    return Err("Guest failed to advance or entered its exception handler".into());
                }
                self.finish(ctx, None);
            }
            _ => {}
        }
        if !matches!(self.stage, Stage::Sampling | Stage::Finished) {
            // Setup and final observation can wait for focus or queued commands.
            ctx.request_repaint_after(Duration::from_millis(20));
        }
        Ok(())
    }

    fn request_progress(&mut self, app: &mut EmulatorApp, command: HostCommand) {
        app.host.queue_effect(command, HostEffect::GameFpsProgress);
        self.progress_pending = true;
    }

    fn finish(&mut self, ctx: &egui::Context, error: Option<String>) {
        self.stage = Stage::Finished;
        drawing::disarm(ctx);
        let report = serde_json::json!({
            "error": error,
            "generation": self.generation.unwrap_or_default(),
            "sample": self.sample,
            "boot_first": self.boot.map(|(first, _)| first),
            "guest_before": self.boot.map(|(_, before)| before),
            "guest_after": self.after,
            "renderer": self.renderer,
            "display": self.display,
        });
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Err(error) = std::fs::write(&self.result_path, report.to_string()) {
                eprintln!("Could not write Game FPS result: {error}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        #[cfg(target_arch = "wasm32")]
        browser::publish(
            "amigaGameFpsResult",
            wasm_bindgen::JsValue::from_str(&report.to_string()),
        );
    }
}

fn check_running_game(app: &EmulatorApp) -> Result<(), String> {
    let status = app.host.status;
    if app.player.screen == PlayerScreen::Session
        && app.view_mode == ViewMode::ScreenOnly
        && status.running
        && status.continuous_run
        && !status.pause_pending
        && status.speed == 1.0
    {
        Ok(())
    } else {
        Err("Expected the loaded Game session to keep running at 1×".into())
    }
}

pub(crate) fn update(app: &mut EmulatorApp, ctx: &egui::Context) {
    if ctx.output(|output| output.num_completed_passes != 0) {
        return;
    }
    if let Some(mut run) = app.game_fps.take() {
        if !matches!(run.stage, Stage::Finished) {
            if let Err(error) = run.advance(app, ctx) {
                run.finish(ctx, Some(error));
            }
        }
        app.game_fps = Some(run);
    }
}

/// Configure the real desktop application using isolated benchmark inputs.
#[cfg(not(target_arch = "wasm32"))]
pub fn attach_native(
    app: &mut EmulatorApp,
    cc: &eframe::CreationContext<'_>,
    rom: &std::path::Path,
    adf: &std::path::Path,
    result_path: std::path::PathBuf,
) {
    let renderer = cc.wgpu_render_state.as_ref().map_or_else(
        || "Unknown native renderer".into(),
        |state| {
            let info = state.adapter.get_info();
            format!(
                "Wgpu {:?}: {} ({})",
                info.backend, info.name, info.driver_info
            )
        },
    );
    let media = Media::native(rom, adf);
    if media.is_ok() {
        eprintln!(
            "Waiting for the Game window to gain focus before starting the benchmark session."
        );
    }
    GameFps::new(media, renderer, result_path).attach(app, &cc.egui_ctx);
}
