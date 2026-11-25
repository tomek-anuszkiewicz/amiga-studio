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
    Pause,
    Command,
    Inspection,
    File(FileCompletion),
    Player(crate::player::PlayerEffect),
    Configuration,
}

#[derive(Debug)]
pub struct AppHost {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) native_gamepads: bool,
    pub(crate) keyboard_pending: VecDeque<(crate::host_input::KeyboardEvent, u64)>,
    view_release_generation: Option<u64>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) pointer: crate::window_pointer::WindowPointer,
    #[cfg(not(target_arch = "wasm32"))]
    pending_pointer: crate::host_input::InputQueue,
    pub status: HostStatus,
    pub data: Option<Box<HostInspection>>,
    pub presented: Option<crate::host_video::FrameInfo>,
    pub runtime: Option<HostRuntime>,
    startup: Option<MachineOwner>,
    queued: VecDeque<(HostCommand, HostEffect)>,
    pending: HashMap<u64, HostEffect>,
    inspector_request: bool,
    activation_pending: bool,
    pub(crate) inspector_revision: Option<u64>,
    debugger_connected: Option<bool>,
    pub(crate) commands: Vec<HostCommand>,
    pub(crate) error: Option<HostRuntimeError>,
}

impl AppHost {
    fn fail(&mut self, error: HostRuntimeError) {
        self.error.get_or_insert(error);
        self.queued.clear();
        self.pending.clear();
        self.commands.clear();
        self.inspector_request = false;
        self.activation_pending = false;
        self.view_release_generation = None;
        self.status.running = false;
        self.status.continuous_run = false;
        self.status.pause_pending = false;
    }

    pub(crate) fn accept_status(&mut self, status: HostStatus) {
        if status.generation != self.status.generation {
            if !status.continuous_run {
                self.data = None;
            }
            self.presented = None;
            self.inspector_revision = None;
            self.keyboard_pending.clear();
            self.view_release_generation = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.pointer.release();
                self.pending_pointer.clear();
            }
        }
        self.status = status;
    }

    pub(crate) fn new(session: Box<debugger::DebuggerSession>) -> Self {
        let owner = MachineOwner::new(session);
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            native_gamepads: false,
            keyboard_pending: VecDeque::new(),
            view_release_generation: None,
            #[cfg(not(target_arch = "wasm32"))]
            pointer: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            pending_pointer: Default::default(),
            status: owner.status(),
            data: None,
            presented: None,
            runtime: None,
            startup: Some(owner),
            queued: VecDeque::new(),
            pending: HashMap::new(),
            inspector_request: false,
            activation_pending: false,
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
            HostCommand::Pause => HostEffect::Pause,
            _ => HostEffect::Command,
        };
        self.queue_effect(command, effect);
    }

    pub fn is_idle(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if !self.pending_pointer.is_empty() {
            return false;
        }
        self.queued.is_empty()
            && self.pending.is_empty()
            && self.commands.is_empty()
            && self.keyboard_pending.is_empty()
            && self.view_release_generation.is_none()
            && !self.status.pause_pending
            && !self.activation_pending
    }

    pub(crate) fn release_input_for_view_change(&mut self) {
        self.keyboard_pending.clear();
        self.view_release_generation = Some(self.status.generation);
    }

    pub(crate) fn can_edit(&self) -> bool {
        self.error.is_none()
            && self
                .queued
                .iter()
                .all(|(_, effect)| matches!(effect, HostEffect::Clock))
            && self
                .pending
                .values()
                .all(|effect| matches!(effect, HostEffect::Clock))
            && self.commands.is_empty()
    }

    pub(crate) fn pause_requested(&self) -> bool {
        self.status.pause_pending
            || self
                .queued
                .iter()
                .any(|(_, effect)| matches!(effect, HostEffect::Pause))
            || self
                .pending
                .values()
                .any(|effect| matches!(effect, HostEffect::Pause))
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
        if self.error.is_some() {
            return;
        }
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
        if self.error.is_some() {
            return Vec::new();
        }
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
                Err(error) => {
                    self.fail(error);
                    return Vec::new();
                }
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
                Err(error) => {
                    self.fail(error);
                }
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
                HostEffect::Player(action) => self.complete_player(action, reply.value),
                HostEffect::Configuration => self.complete_configuration_save(reply.value),
                HostEffect::Command | HostEffect::Run | HostEffect::Pause => {
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
                        self.host.status = runtime.status();
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
        if previous.generation != self.host.status.generation {
            self.player.captured = false;
        }
        if previous.running && !self.host.status.running {
            self.disassembly_view_addr = None;
        }
        if previous.generation != self.host.status.generation && self.host.data.is_none() {
            self.inspection.ready = false;
        }
        if let Some(error) = &self.host.error {
            self.file_error = Some(match error {
                HostRuntimeError::Fatal(message) => message.clone(),
                error => format!("Machine worker failed: {error:?}"),
            });
        }
        if first_pass {
            self.host.refresh_utc(crate::host_clock::utc_now());
        }
    }
}

impl crate::EmulatorApp {
    pub(crate) fn finish_ui_pass(&mut self, ctx: &egui::Context, first_pass: bool) {
        if first_pass {
            self.poll_file_dialog(ctx);
            self.handle_gamepad_actions(ctx);
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.render_saved_progress(ctx);
        crate::layout::file_dialogs::render(self, ctx);
        #[cfg(not(target_arch = "wasm32"))]
        crate::window_keyboard::with_artwork_paste(ctx, |paste| {
            paste.active = self.player.screen == crate::player::PlayerScreen::Library
                && self.player.library_editor.is_some()
                && self.player.pending_launch.is_none()
                && self.file_error.is_none()
                && !self.load_binary_modal_open;
            if !paste.active {
                paste.pending = None;
            }
        });

        self.host.flush();
        self.host.request_submission_retry(ctx);
        if first_pass {
            self.update_host_input(ctx);
        }
    }

    fn handle_gamepad_actions(&mut self, ctx: &egui::Context) {
        if self.keyboard_policy(ctx).frontend_active() {
            if let Some(runtime) = &self.host.runtime {
                for action in runtime.input_sender().take_frontend_actions() {
                    use crate::player_config::FrontendAction;
                    if action == FrontendAction::Debug {
                        self.set_view_mode(crate::ViewMode::Developer);
                    } else {
                        self.apply_key_command(action.into(), ctx);
                    }
                    if action == FrontendAction::Load {
                        break;
                    }
                }
            }
        }
    }

    pub(crate) fn update_host_input(&mut self, ctx: &egui::Context) {
        let mut policy = self.keyboard_policy(ctx);
        let capture_requested = ctx
            .data_mut(|data| data.remove_temp::<bool>(egui::Id::unique("capture_requested")))
            .unwrap_or(false);
        self.player.captured = policy.gameplay_active()
            && self.host.view_release_generation.is_none()
            && (self.player.captured || capture_requested);
        if ctx.input(|input| input.pointer.button_pressed(egui::PointerButton::Middle)) {
            self.player.captured = false;
        }
        #[cfg(target_arch = "wasm32")]
        if crate::host_runtime::take_pointer_capture_loss() {
            self.player.captured = false;
        }
        if self.player.captured {
            if let Some(error) =
                self.player
                    .active_configuration
                    .as_ref()
                    .and_then(|configuration| {
                        configuration.validate(self.player.current_mappings()).err()
                    })
            {
                self.player.captured = false;
                self.file_error = Some(format!(
                    "Correct input configuration before capture: {error}"
                ));
                policy.gui_owns_input = true;
            }
        }
        let mouse = self.player.captured;
        policy.native_mouse_captured = cfg!(not(target_arch = "wasm32")) && mouse;
        let gameplay = policy.gameplay_active();
        if !gameplay {
            self.host
                .keyboard_pending
                .retain(|(event, _)| *event == crate::host_input::KeyboardEvent::ReleaseAll);
        }
        let frontend = policy.frontend_active();
        crate::keyboard_input::with_inbox(ctx, |inbox| {
            inbox.set_policy(policy, self.host.status.generation);
            if !self.player.editor.as_ref().is_some_and(|editor| {
                editor
                    .recording
                    .as_ref()
                    .is_some_and(|recording| recording.controller.is_none())
            }) {
                inbox.stop_recording();
            }
        });

        if self.host.error.is_some() {
            return;
        }
        if let Some(runtime) = &self.host.runtime {
            let sender = runtime.input_sender();
            let configuration_applied =
                self.player
                    .active_configuration
                    .as_ref()
                    .is_none_or(|configuration| {
                        sender.configure(&configuration.controls, self.player.current_mappings())
                    });
            if let Some(pads) = sender.gamepads() {
                self.player.gamepads = pads;
            }
            let activation_applied = configuration_applied
                && sender.apply_input_activation(
                    gameplay,
                    mouse,
                    gameplay,
                    Some(frontend),
                    self.host.view_release_generation,
                    self.player
                        .editor
                        .as_ref()
                        .and_then(|editor| editor.gamepad_recording()),
                );
            self.host.activation_pending = !configuration_applied || !activation_applied;
            if self.host.activation_pending {
                ctx.request_repaint_after(HOST_RETRY_DELAY);
            } else {
                self.host.view_release_generation = None;
                while let Some((event, generation)) = self.host.keyboard_pending.front() {
                    #[cfg(target_arch = "wasm32")]
                    if crate::host_runtime::host_release_pending() {
                        ctx.request_repaint_after(HOST_RETRY_DELAY);
                        break;
                    }
                    if !sender.try_keyboard_event(event, *generation) {
                        ctx.request_repaint_after(HOST_RETRY_DELAY);
                        break;
                    }
                    self.host.keyboard_pending.pop_front();
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::CursorGrab(if mouse {
                    egui::CursorGrab::Locked
                } else {
                    egui::CursorGrab::None
                }));
                ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(!mouse));
                if mouse {
                    for event in ctx.input(|input| input.events.clone()) {
                        if let Some(event) = self.host.pointer.translate(&event, true) {
                            self.host.pending_pointer.push(event);
                        }
                    }
                    if !self.host.activation_pending {
                        while let Some(event) = self.host.pending_pointer.first() {
                            if !sender.try_host_event(event, self.host.status.generation) {
                                ctx.request_repaint_after(HOST_RETRY_DELAY);
                                break;
                            }
                            self.host.pending_pointer.pop();
                        }
                    }
                } else {
                    self.host.pointer.release();
                    self.host.pending_pointer.clear();
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            if self.host.native_gamepads && !runtime.start_native_gamepads() {
                ctx.request_repaint_after(HOST_RETRY_DELAY);
            }
        }
    }
}
