//! Bounded host exchange and machine-owner lifetime on desktop and the web.

use crate::host_commands::{HostCommand, HostReply, HostRequest, HostStatus, HOST_QUEUE_CAPACITY};
use crate::host_failure::HostFailure;
use crate::host_input::{HostInputEvent, InputQueue};
use crate::host_owner::MachineOwner;
use crate::host_video::{FrameInfo, VideoExchange, VideoExchangeError, VideoRead};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, TryLockError};

#[cfg(all(not(target_arch = "wasm32"), panic = "abort"))]
compile_error!("The native machine owner requires panic=unwind for fatal invariant reporting");

pub(crate) const HOST_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(1);
#[cfg(target_arch = "wasm32")]
const WEB_WAIT_FOR_EVENT: f64 = -1.0;
#[cfg(target_arch = "wasm32")]
const WEB_STOPPED: f64 = -2.0;

#[derive(Debug)]
struct Mailboxes {
    commands: VecDeque<HostRequest>,
    replies: VecDeque<HostReply>,
    status: HostStatus,
    input: InputQueue,
    input_enabled: bool,
    frontend_enabled: bool,
    gamepad_enabled: bool,
    mapping: crate::input_mapping::InputMapping,
    input_generation: u64,
    #[cfg(not(target_arch = "wasm32"))]
    gamepads_started_at: std::time::SystemTime,
}

impl Mailboxes {
    fn input_active(&self) -> bool {
        self.input_enabled && self.status.running && !self.status.pause_pending
    }

    fn gamepads_active(&self) -> bool {
        self.gamepad_enabled && self.status.running && !self.status.pause_pending
    }

    fn sample_gamepads(
        &mut self,
        samples: Vec<crate::input_mapping::GamepadSample>,
        active: bool,
        frontend_allowed: bool,
    ) -> bool {
        let actions_before = self.mapping.actions.len();
        let devices = samples
            .iter()
            .map(|pad| crate::input_mapping::GamepadInfo {
                identity: pad.identity.clone(),
                name: pad.name.clone(),
            })
            .collect();
        let devices_changed = self.mapping.available != devices;
        for event in self.mapping.update_devices(devices) {
            self.push_input(event);
        }
        for sample in samples {
            self.mapping.frontend_gamepad(
                &sample.identity,
                &sample.buttons,
                self.frontend_enabled && frontend_allowed,
            );
            if active {
                for event in self
                    .mapping
                    .gamepad(&sample.identity, &sample.axes, &sample.buttons)
                {
                    self.push_input(event);
                }
            }
        }
        devices_changed || self.mapping.actions.len() != actions_before
    }

    fn resolve_input(&mut self, event: HostInputEvent) {
        let events = match event {
            HostInputEvent::Key { code, pressed } => self.mapping.keyboard(code, pressed),
            HostInputEvent::MouseMotion { .. } | HostInputEvent::MouseButtons { .. } => {
                self.mapping.pointer(event)
            }
            HostInputEvent::ReleaseCapture { .. } => vec![self.mapping.release_capture()],
            HostInputEvent::ReleaseAll => {
                self.mapping.release();
                vec![event]
            }
            _ => vec![event],
        };
        for event in events {
            self.push_input(event);
        }
    }

    fn push_input(&mut self, event: HostInputEvent) {
        if self.input_generation != self.status.generation {
            self.input.clear();
            self.input_generation = self.status.generation;
        }
        self.input.push(event);
    }
}

#[cfg(not(target_arch = "wasm32"))]
static NATIVE_GAMEPADS: Mutex<Option<HostInputSender>> = Mutex::new(None);
#[cfg(not(target_arch = "wasm32"))]
static NATIVE_GAMEPAD_LISTENER: std::sync::Once = std::sync::Once::new();
#[cfg(not(target_arch = "wasm32"))]
const MAX_NATIVE_SLEEP: std::time::Duration = std::time::Duration::from_millis(1);

#[derive(Debug)]
struct SharedHost {
    mailboxes: Mutex<Mailboxes>,
    video: VideoExchange,
    stop: AtomicBool,
    stopped: AtomicBool,
    // Failure publication never acquires the owner's locks.
    failure: HostFailure,
}

/// A rejected submission retains its owned command for retry by the GUI.
#[derive(Debug)]
pub struct HostBusy(pub HostCommand);

/// Event callbacks publish under the existing bounded host mailbox boundary.
#[derive(Debug, Clone)]
pub struct HostInputSender {
    shared: Arc<SharedHost>,
    repaint: egui::Context,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostRuntimeError {
    WorkerStopped,
    StartupFailed,
    Fatal(String),
}

/// The GUI owns this handle; the worker exclusively owns the debugger session.
#[derive(Debug)]
pub struct HostRuntime {
    shared: Arc<SharedHost>,
    repaint: egui::Context,
    #[cfg(not(target_arch = "wasm32"))]
    worker: Option<std::thread::JoinHandle<rtc::RtcBattery>>,
    #[cfg(target_arch = "wasm32")]
    worker: web_sys::Worker,
    status: HostStatus,
    next_request: u64,
    generation_barrier: Option<u64>,
}

impl HostRuntime {
    pub fn start(owner: MachineOwner, context: egui::Context) -> Result<Self, HostRuntimeError> {
        let spare = vec![0; owner.video_storage_len()];
        let status = owner.status();
        let shared = Arc::new(SharedHost {
            mailboxes: Mutex::new(Mailboxes {
                commands: VecDeque::with_capacity(HOST_QUEUE_CAPACITY),
                replies: VecDeque::with_capacity(HOST_QUEUE_CAPACITY),
                status,
                input: Default::default(),
                input_enabled: false,
                frontend_enabled: false,
                gamepad_enabled: false,
                mapping: Default::default(),
                input_generation: status.generation,
                #[cfg(not(target_arch = "wasm32"))]
                gamepads_started_at: std::time::SystemTime::now(),
            }),
            video: VideoExchange::new(spare),
            stop: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            failure: HostFailure::default(),
        });
        let worker_shared = shared.clone();
        let repaint = context.clone();
        #[cfg(not(target_arch = "wasm32"))]
        let worker = std::thread::spawn(move || run_native(owner, worker_shared, context));
        #[cfg(target_arch = "wasm32")]
        let worker = start_web(owner, worker_shared, context)?;
        Ok(Self {
            shared,
            repaint,
            #[cfg(not(target_arch = "wasm32"))]
            worker: Some(worker),
            #[cfg(target_arch = "wasm32")]
            worker,
            status,
            next_request: 1,
            generation_barrier: None,
        })
    }

    pub fn status(&self) -> HostStatus {
        self.status
    }

    pub fn input_sender(&self) -> HostInputSender {
        HostInputSender {
            shared: self.shared.clone(),
            repaint: self.repaint.clone(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn start_native_gamepads(&self) -> bool {
        let Ok(mut target) = NATIVE_GAMEPADS.try_lock() else {
            return false;
        };
        *target = Some(self.input_sender());
        NATIVE_GAMEPAD_LISTENER.call_once(|| {
            std::thread::spawn(crate::host_gamepads::listen_gamepads);
        });
        true
    }

    fn wake_worker(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(worker) = &self.worker {
            worker.thread().unpark();
        }
        #[cfg(target_arch = "wasm32")]
        if self
            .worker
            .post_message(&wasm_bindgen::JsValue::NULL)
            .is_err()
        {
            self.shared.failure.mark_failed();
        }
    }

    pub fn submit(&mut self, command: HostCommand) -> Result<u64, HostBusy> {
        if self.generation_barrier.is_some()
            || self.status.shutdown
            || self.shared.failure.is_failed()
            || self.shared.stopped.load(Ordering::Acquire)
        {
            return Err(HostBusy(command));
        }
        let changes_generation = matches!(
            &command,
            HostCommand::Reset
                | HostCommand::ResetWarm
                | HostCommand::Launch { .. }
                | HostCommand::RestorePlayer { .. }
                | HostCommand::ReconfigurePorts(_)
                | HostCommand::LoadQuickSlot { .. }
                | HostCommand::LoadRom(_)
                | HostCommand::LoadBinary { .. }
        );
        let Ok(mut mailboxes) = self.shared.mailboxes.try_lock() else {
            return Err(HostBusy(command));
        };
        if mailboxes.commands.len() == HOST_QUEUE_CAPACITY
            || self.shared.stop.load(Ordering::Acquire)
        {
            return Err(HostBusy(command));
        }
        let id = self.next_request;
        self.next_request = self.next_request.wrapping_add(1);
        if changes_generation {
            self.generation_barrier = Some(id);
        }
        mailboxes.commands.push_back(HostRequest {
            id,
            generation: self.status.generation,
            command,
        });
        drop(mailboxes);
        self.wake_worker();
        Ok(id)
    }

    fn is_worker_finished(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.worker
                .as_ref()
                .is_some_and(|worker| worker.is_finished())
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.shared.stopped.load(Ordering::Acquire)
        }
    }

    pub fn poll(&mut self) -> Result<Vec<HostReply>, HostRuntimeError> {
        if self.shared.failure.is_failed() {
            self.status.running = false;
            self.status.continuous_run = false;
            self.status.pause_pending = false;
            return Err(self.shared.failure.error());
        }
        let mut mailboxes = match self.shared.mailboxes.try_lock() {
            Ok(mailboxes) => mailboxes,
            Err(TryLockError::WouldBlock) => return Ok(Vec::new()),
            Err(TryLockError::Poisoned(_)) => {
                // Unwinding can poison the lock before the outer catch publishes
                // the diagnostic. Wait for that boundary instead of latching a
                // generic mailbox error that would hide the original failure.
                if let Some(error) = self.shared.failure.poll(|| self.is_worker_finished()) {
                    self.status.running = false;
                    self.status.continuous_run = false;
                    self.status.pause_pending = false;
                    return Err(error);
                }
                self.repaint.request_repaint_after(HOST_RETRY_DELAY);
                return Ok(Vec::new());
            }
        };
        if mailboxes.status.revision >= self.status.revision {
            self.status = mailboxes.status;
        }
        if let Some(error) = self
            .shared
            .failure
            .poll(|| self.is_worker_finished() && !self.status.shutdown)
        {
            self.status.running = false;
            self.status.continuous_run = false;
            self.status.pause_pending = false;
            return Err(error);
        }
        let replies: Vec<HostReply> = mailboxes.replies.drain(..).collect();
        for reply in &replies {
            if reply.status.revision >= self.status.revision {
                self.status = reply.status;
            }
        }
        if replies
            .iter()
            .any(|reply| Some(reply.id) == self.generation_barrier)
        {
            self.generation_barrier = None;
        }
        drop(mailboxes);
        if !replies.is_empty() {
            self.wake_worker();
        }
        Ok(replies)
    }

    pub fn try_consume<T>(
        &mut self,
        read: impl FnOnce(FrameInfo, HostStatus, &[u32]) -> T,
    ) -> Result<VideoRead<T>, VideoExchangeError> {
        if self.shared.failure.is_failed() {
            return Ok(VideoRead::Pending);
        }
        if self.generation_barrier.is_some() {
            return Ok(VideoRead::Pending);
        }
        // Publication holds this same lock until matching status is visible.
        // The GUI only tries the lock, then releases it before pixel conversion.
        let mut mailboxes = match self.shared.mailboxes.try_lock() {
            Ok(mailboxes) => Some(mailboxes),
            Err(TryLockError::WouldBlock) => {
                // The ready-image notification may already have been consumed.
                // Retry contention once work can finish; an empty slot never polls.
                self.repaint.request_repaint_after(HOST_RETRY_DELAY);
                return Ok(VideoRead::Pending);
            }
            Err(TryLockError::Poisoned(_)) => {
                // Terminal reporting belongs to poll, after the owner boundary.
                self.repaint.request_repaint_after(HOST_RETRY_DELAY);
                return Ok(VideoRead::Pending);
            }
        };
        if let Some(mailboxes) = &mailboxes {
            if mailboxes.status.revision >= self.status.revision {
                self.status = mailboxes.status;
            }
        }
        let status = self.status;
        let result = self
            .shared
            .video
            .try_consume(status.generation, |info, pixels| {
                drop(mailboxes.take());
                read(info, status, pixels)
            })?;
        drop(mailboxes);
        if !matches!(&result, VideoRead::Pending) {
            self.wake_worker();
        }
        Ok(result)
    }

    /// Finishes an acknowledged shutdown without waiting on an active worker.
    pub fn finish_shutdown(&mut self) -> Result<Option<rtc::RtcBattery>, HostRuntimeError> {
        if self.shared.failure.is_failed() {
            return Err(self.shared.failure.error());
        }
        #[cfg(target_arch = "wasm32")]
        return Ok(self
            .shared
            .stopped
            .load(Ordering::Acquire)
            .then_some(self.status.battery));
        #[cfg(not(target_arch = "wasm32"))]
        {
            if !self
                .worker
                .as_ref()
                .is_some_and(|worker| worker.is_finished())
            {
                return Ok(None);
            }
            match self.worker.take().map(|worker| worker.join()) {
                Some(Ok(battery)) => {
                    // Completion can race the GUI's last nonblocking poll. A
                    // caught fatal panic also returns normally from the thread.
                    if self.shared.failure.is_failed() {
                        return Err(self.shared.failure.error());
                    }
                    self.status.shutdown = true;
                    self.status.running = false;
                    self.status.continuous_run = false;
                    self.status.pause_pending = false;
                    self.status.battery = battery;
                    Ok(Some(battery))
                }
                Some(Err(_)) => Err(HostRuntimeError::WorkerStopped),
                None => Ok(None),
            }
        }
    }
}

impl Drop for HostRuntime {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(mut target) = NATIVE_GAMEPADS.lock() {
            if target
                .as_ref()
                .is_some_and(|sender| Arc::ptr_eq(&sender.shared, &self.shared))
            {
                *target = None;
            }
        }
        self.shared.stop.store(true, Ordering::Release);
        self.wake_worker();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct WorkerLoop {
    owner: MachineOwner,
    shared: Arc<SharedHost>,
    pending_reply: Option<HostReply>,
}

enum WorkerWait {
    Event,
    Delay(std::time::Duration),
    Stop,
}

impl WorkerLoop {
    fn tick(&mut self, now: f64, elapsed: impl Fn() -> f64) -> (bool, WorkerWait) {
        if self.shared.stop.load(Ordering::Acquire) {
            return (false, WorkerWait::Stop);
        }
        let previous_running = self.owner.status().running;
        let previous_generation = self.owner.status().generation;
        let Self {
            owner,
            shared,
            pending_reply,
        } = self;
        let mut notify = false;
        let mut contended = false;
        for _ in 0..HOST_QUEUE_CAPACITY {
            if shared.stop.load(Ordering::Acquire) {
                break;
            }
            let request = match shared.mailboxes.try_lock() {
                Ok(mut mailboxes) => {
                    if let Some(reply) = pending_reply.take() {
                        if mailboxes.replies.len() == HOST_QUEUE_CAPACITY {
                            *pending_reply = Some(reply);
                            break;
                        }
                        mailboxes.replies.push_back(reply);
                        notify = true;
                    }
                    if owner.status().shutdown {
                        break;
                    }
                    if mailboxes.replies.len() == HOST_QUEUE_CAPACITY {
                        break;
                    }
                    mailboxes.commands.pop_front()
                }
                Err(TryLockError::WouldBlock) => {
                    contended = true;
                    break;
                }
                Err(TryLockError::Poisoned(_)) => return Self::fail(shared),
            };
            let Some(request) = request else {
                break;
            };
            *pending_reply = Some(owner.execute(request));
        }
        match shared.mailboxes.try_lock() {
            Ok(mut mailboxes) => {
                if previous_generation != owner.status().generation
                    || mailboxes.input_generation != owner.status().generation
                    || !owner.status().running
                    || owner.status().pause_pending
                {
                    mailboxes.input.clear();
                } else {
                    while let Some(event) = mailboxes.input.pop() {
                        owner.collect_input(event);
                    }
                }
            }
            Err(TryLockError::WouldBlock) => contended = true,
            Err(TryLockError::Poisoned(_)) => return Self::fail(shared),
        }
        owner.advance(now, &elapsed);
        match shared.mailboxes.try_lock() {
            Ok(mut mailboxes) => {
                match owner.publish(&shared.video) {
                    Ok(published) => notify |= published,
                    Err(_) => return Self::fail(shared),
                }
                let status = owner.status();
                if mailboxes.status.generation != status.generation {
                    mailboxes.mapping.release();
                    mailboxes.input.clear();
                    mailboxes.input_generation = status.generation;
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        mailboxes.gamepads_started_at = std::time::SystemTime::now();
                    }
                }
                mailboxes.status = status;
            }
            Err(TryLockError::WouldBlock) => contended = true,
            Err(TryLockError::Poisoned(_)) => return Self::fail(shared),
        }
        notify |= previous_running != owner.status().running;
        if owner.status().shutdown && pending_reply.is_none() {
            return (notify, WorkerWait::Stop);
        }
        let wait = if contended {
            WorkerWait::Delay(HOST_RETRY_DELAY)
        } else if pending_reply.is_some() {
            WorkerWait::Event
        } else if let Some(delay) = owner.next_delay(now + elapsed()) {
            WorkerWait::Delay(delay)
        } else {
            WorkerWait::Event
        };
        (notify, wait)
    }

    fn fail(shared: &SharedHost) -> (bool, WorkerWait) {
        shared.failure.mark_failed();
        (true, WorkerWait::Stop)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_native(
    owner: MachineOwner,
    shared: Arc<SharedHost>,
    context: egui::Context,
) -> rtc::RtcBattery {
    let clock = std::time::Instant::now();
    let mut worker = WorkerLoop {
        owner,
        shared,
        pending_reply: None,
    };
    // The battery belongs to the last healthy owner boundary. Never inspect or
    // publish the partially advanced machine after a fatal failure.
    let battery = worker.owner.status().battery;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| loop {
        let started = std::time::Instant::now();
        let (notify, wait) = worker.tick(clock.elapsed().as_secs_f64(), || {
            started.elapsed().as_secs_f64()
        });
        if notify {
            context.request_repaint();
        }
        match wait {
            WorkerWait::Event => std::thread::park(),
            // Windows address-based timed parking has a coarse timer quantum.
            // Sleep uses the high-resolution timer; bound waits to service commands.
            WorkerWait::Delay(delay) if !delay.is_zero() => {
                std::thread::sleep(delay.min(MAX_NATIVE_SLEEP))
            }
            WorkerWait::Delay(_) => {}
            WorkerWait::Stop => return worker.owner.status().battery,
        }
    }));
    let battery = match result {
        Ok(battery) => battery,
        Err(payload) => {
            let message = if let Some(message) = payload.downcast_ref::<String>() {
                message.clone()
            } else if let Some(message) = payload.downcast_ref::<&str>() {
                (*message).to_owned()
            } else {
                "Machine owner panicked without a text diagnostic".to_owned()
            };
            worker.shared.failure.record(message);
            battery
        }
    };
    worker.shared.stopped.store(true, Ordering::Release);
    context.request_repaint();
    battery
}

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(target_arch = "wasm32")]
pub(crate) use browser::take_pointer_capture_loss;
#[cfg(target_arch = "wasm32")]
use browser::{set_host_input_enabled, start_web};

mod input;

#[cfg(not(target_arch = "wasm32"))]
mod gamepads;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use gamepads::collect_native_gamepad;
