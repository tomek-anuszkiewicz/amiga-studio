//! GUI-owned command bookkeeping; the machine is moved once at first update.

use crate::file_actions::FileCompletion;
use crate::host_commands::{HostCommand, HostReply, HostStatus, HostValue};
use crate::host_inspection::HostInspection;
use crate::host_owner::MachineOwner;
use crate::host_runtime::{HostRuntime, HostRuntimeError, HOST_RETRY_DELAY};
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
pub(crate) enum HostEffect {
    Clock,
    Run,
    Command,
    Inspection,
    File(FileCompletion),
}

#[derive(Debug)]
pub struct AppHost {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) native_input: bool,
    pub status: HostStatus,
    pub data: Option<Box<HostInspection>>,
    pub presented: Option<crate::host_video::FrameInfo>,
    pub runtime: Option<HostRuntime>,
    startup: Option<MachineOwner>,
    queued: VecDeque<(HostCommand, HostEffect)>,
    pending: HashMap<u64, HostEffect>,
    inspector_request: bool,
    pub(crate) inspector_revision: Option<u64>,
    debugger_connected: Option<bool>,
    pub(crate) commands: Vec<HostCommand>,
    pub(crate) error: Option<HostRuntimeError>,
}

impl AppHost {
    pub(crate) fn accept_status(&mut self, status: HostStatus) {
        if status.generation != self.status.generation {
            if !status.continuous_run {
                self.data = None;
            }
            self.presented = None;
            self.inspector_revision = None;
        }
        self.status = status;
    }

    pub(crate) fn new(session: Box<debugger::DebuggerSession>) -> Self {
        let owner = MachineOwner::new(session);
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            native_input: false,
            status: owner.status(),
            data: None,
            presented: None,
            runtime: None,
            startup: Some(owner),
            queued: VecDeque::new(),
            pending: HashMap::new(),
            inspector_request: false,
            inspector_revision: None,
            debugger_connected: None,
            commands: Vec::new(),
            error: None,
        }
    }

    pub fn queue(&mut self, command: HostCommand) {
        let effect = match &command {
            HostCommand::SupplyUtc(_) => HostEffect::Clock,
            HostCommand::Run => HostEffect::Run,
            _ => HostEffect::Command,
        };
        self.queue_effect(command, effect);
    }

    pub fn is_idle(&self) -> bool {
        self.queued.is_empty()
            && self.pending.is_empty()
            && self.commands.is_empty()
            && !self.status.pause_pending
    }

    pub(crate) fn can_edit(&self) -> bool {
        self.queued
            .iter()
            .all(|(_, effect)| matches!(effect, HostEffect::Clock))
            && self
                .pending
                .values()
                .all(|effect| matches!(effect, HostEffect::Clock))
            && self.commands.is_empty()
    }

    pub(crate) fn continuous_requested(&self) -> bool {
        self.status.continuous_run
            || self
                .queued
                .iter()
                .any(|(_, effect)| matches!(effect, HostEffect::Run))
            || self
                .pending
                .values()
                .any(|effect| matches!(effect, HostEffect::Run))
    }

    pub(crate) fn refresh_utc(&mut self, utc: i64) {
        let clock_pending = self
            .queued
            .iter()
            .any(|(_, effect)| matches!(effect, HostEffect::Clock))
            || self
                .pending
                .values()
                .any(|effect| matches!(effect, HostEffect::Clock));
        if !clock_pending
            && self.status.rtc_time != utc.wrapping_add(self.status.battery.offset_seconds)
            && !self.status.shutdown
        {
            self.queue(HostCommand::SupplyUtc(utc));
        }
    }

    pub(crate) fn queue_effect(&mut self, command: HostCommand, effect: HostEffect) {
        self.queued.push_back((command, effect));
    }

    pub(crate) fn flush(&mut self) {
        for command in std::mem::take(&mut self.commands) {
            self.queue(command);
        }
        if self.error.is_some() || self.status.shutdown {
            return;
        }
        let Some(runtime) = &mut self.runtime else {
            return;
        };
        while let Some((mut command, mut effect)) = self.queued.pop_front() {
            if matches!(&command, HostCommand::Run)
                && self.debugger_connected == Some(true)
                && !self.inspector_request
            {
                self.queued.push_front((command, effect));
                command = HostCommand::Inspect;
                effect = HostEffect::Inspection;
                self.inspector_request = true;
            }
            match runtime.submit(command) {
                Ok(id) => {
                    self.pending.insert(id, effect);
                }
                Err(busy) => {
                    self.queued.push_front((busy.0, effect));
                    break;
                }
            }
        }
    }

    pub(crate) fn request_submission_retry(&self, context: &egui::Context) {
        if !self.queued.is_empty() && self.error.is_none() {
            context.request_repaint_after(HOST_RETRY_DELAY);
        }
    }

    pub(crate) fn poll(
        &mut self,
        context: &egui::Context,
        developer: bool,
    ) -> Vec<(HostEffect, HostReply)> {
        if let Some(owner) = self.startup.take() {
            match HostRuntime::start(owner, context.clone()) {
                Ok(runtime) => {
                    self.runtime = Some(runtime);
                    if developer {
                        // Capture stopped data before an initially queued Run.
                        self.queued
                            .push_front((HostCommand::Inspect, HostEffect::Inspection));
                        self.inspector_request = true;
                    }
                }
                Err(error) => self.error = Some(error),
            }
        }
        let mut completed = Vec::new();
        if self.debugger_connected != Some(developer) {
            self.queued.push_front((
                HostCommand::SetDebuggerEnabled(developer),
                HostEffect::Command,
            ));
            self.debugger_connected = Some(developer);
        }
        if let Some(runtime) = &mut self.runtime {
            match runtime.poll() {
                Ok(replies) => {
                    let status = runtime.status();
                    self.accept_status(status);
                    for reply in replies {
                        let Some(effect) = self.pending.remove(&reply.id) else {
                            continue;
                        };
                        if matches!(&effect, HostEffect::Inspection) {
                            self.inspector_request = false;
                            if reply.status.generation == self.status.generation {
                                if let Ok(HostValue::Inspection(data)) = reply.value {
                                    self.data = Some(data);
                                    self.inspector_revision = Some(reply.status.revision);
                                }
                            }
                        } else {
                            completed.push((effect, reply));
                        }
                    }
                }
                Err(error) => self.error = Some(error),
            }
        }
        self.flush();
        if developer
            && !self.status.running
            && !self.status.shutdown
            && !self.inspector_request
            && self.inspector_revision != Some(self.status.revision)
            && self.queued.is_empty()
            && self.error.is_none()
        {
            self.queue_effect(HostCommand::Inspect, HostEffect::Inspection);
            self.inspector_request = true;
            self.flush();
        }
        completed
    }
}

impl crate::EmulatorApp {
    pub(crate) fn poll_host(&mut self, ctx: &egui::Context, first_pass: bool) {
        #[cfg(not(target_arch = "wasm32"))]
        if ctx.input(|input| input.viewport().close_requested()) && !self.close_ready {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self.closing {
                self.closing = true;
                self.host.queue(HostCommand::Shutdown);
            }
        }
        let previous = self.host.status;
        let previous_inspection = self.host.inspector_revision;
        let completed = self
            .host
            .poll(ctx, self.view_mode == crate::ViewMode::Developer);
        if previous_inspection != self.host.inspector_revision
            && (self.host.inspector_revision.is_some() || self.host.data.is_none())
        {
            self.inspection.ready = false;
        }
        for (effect, reply) in completed {
            match effect {
                HostEffect::File(action) => self.complete_file_action(action, reply.value),
                HostEffect::Command | HostEffect::Run => {
                    if let Err(error) = reply.value {
                        self.file_error = Some(error);
                    }
                }
                HostEffect::Inspection | HostEffect::Clock => {}
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.closing {
            if let Some(runtime) = &mut self.host.runtime {
                match runtime.finish_shutdown() {
                    Ok(Some(battery)) => {
                        self.host.status.battery = battery;
                        self.close_ready = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    Err(error) => {
                        self.file_error = Some(format!("Machine worker failed: {error:?}"));
                        self.close_ready = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    Ok(None) => ctx.request_repaint_after(HOST_RETRY_DELAY),
                }
            }
        }
        if previous.running && !self.host.status.running {
            self.disassembly_view_addr = None;
        }
        if previous.generation != self.host.status.generation && self.host.data.is_none() {
            self.inspection.ready = false;
        }
        if let Some(error) = self.host.error {
            self.file_error = Some(format!("Machine worker failed: {error:?}"));
        }
        if first_pass {
            self.host.refresh_utc(crate::host_clock::utc_now());
            self.handle_keyboard_shortcuts(ctx);
        }
    }
}

impl crate::EmulatorApp {
    pub(crate) fn update_input_capture(&self, ctx: &egui::Context) {
        if let Some(runtime) = &self.host.runtime {
            let focused = ctx.input(|input| input.focused);
            let screen_focus = ctx.memory(|memory| {
                memory.has_focus(egui::Id::new(
                    crate::layout::main_viewport::amiga_screen::SCREEN_INPUT_ID,
                ))
            });
            let enabled = self.host.status.running
                && !self.host.status.pause_pending
                && focused
                && (!ctx.wants_keyboard_input() || screen_focus)
                && (self.view_mode == crate::ViewMode::ScreenOnly || screen_focus);
            if !runtime.input_sender().set_enabled(enabled) {
                ctx.request_repaint_after(crate::host_runtime::HOST_RETRY_DELAY);
            }
            #[cfg(not(target_arch = "wasm32"))]
            if self.host.native_input && !runtime.start_native_input() {
                ctx.request_repaint_after(crate::host_runtime::HOST_RETRY_DELAY);
            }
        }
    }
}
