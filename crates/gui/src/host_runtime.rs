//! Bounded host exchange and machine-owner lifetime on desktop and the web.

use crate::host_commands::{HostCommand, HostReply, HostRequest, HostStatus, HOST_QUEUE_CAPACITY};
use crate::host_input::{HostInputEvent, InputQueue};
use crate::host_owner::MachineOwner;
use crate::host_video::{FrameInfo, VideoExchange, VideoExchangeError, VideoRead};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, TryLockError};

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
    input_generation: u64,
    capture_epoch: u64,
    #[cfg(not(target_arch = "wasm32"))]
    capture_started_at: std::time::SystemTime,
}

impl Mailboxes {
    fn input_active(&self) -> bool {
        self.input_enabled && self.status.running && !self.status.pause_pending
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
static NATIVE_INPUT: Mutex<Option<HostInputSender>> = Mutex::new(None);
#[cfg(not(target_arch = "wasm32"))]
static NATIVE_LISTENER: std::sync::Once = std::sync::Once::new();
#[cfg(not(target_arch = "wasm32"))]
const MAX_NATIVE_SLEEP: std::time::Duration = std::time::Duration::from_millis(1);

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn collect_native_event(
    events: &mut crate::host_native_input::NativeEvents,
    raw: rdev::Event,
) {
    let Ok(target) = NATIVE_INPUT.lock() else {
        return;
    };
    let Some(sender) = &*target else {
        events.translate(raw.event_type, false);
        return;
    };
    let Ok(mut mailboxes) = sender.shared.mailboxes.lock() else {
        return;
    };
    events.capture((mailboxes.status.generation, mailboxes.capture_epoch));
    let enabled = mailboxes.input_active() && raw.time >= mailboxes.capture_started_at;
    if let Some(event) = events.translate(raw.event_type, enabled) {
        mailboxes.push_input(event);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn collect_native_gamepad(
    gamepads: &mut crate::host_native_input::NativeGamepads,
    observed: std::time::SystemTime,
) {
    let Ok(target) = NATIVE_INPUT.lock() else {
        return;
    };
    let Some(sender) = &*target else {
        gamepads.release();
        return;
    };
    let Ok(mut mailboxes) = sender.shared.mailboxes.lock() else {
        return;
    };
    if !mailboxes.input_active() || observed < mailboxes.capture_started_at {
        gamepads.release();
        return;
    }
    for event in gamepads
        .changed_events((mailboxes.status.generation, mailboxes.capture_epoch))
        .into_iter()
        .flatten()
    {
        mailboxes.push_input(event);
    }
}

#[derive(Debug)]
struct SharedHost {
    mailboxes: Mutex<Mailboxes>,
    video: VideoExchange,
    stop: AtomicBool,
    failed: AtomicBool,
    stopped: AtomicBool,
}

/// A rejected submission retains its owned command for retry by the GUI.
#[derive(Debug)]
pub struct HostBusy(pub HostCommand);

/// Event callbacks publish under the existing bounded host mailbox boundary.
#[derive(Debug, Clone)]
pub struct HostInputSender {
    shared: Arc<SharedHost>,
}

impl HostInputSender {
    pub fn try_send(&self, event: HostInputEvent, generation: Option<u64>) -> bool {
        let Ok(mut mailboxes) = self.shared.mailboxes.try_lock() else {
            return false;
        };
        if generation.is_some_and(|generation| generation != mailboxes.status.generation) {
            return true;
        }
        if event == HostInputEvent::ReleaseAll || mailboxes.input_active() {
            mailboxes.push_input(event);
        }
        true
    }

    pub fn set_enabled(&self, enabled: bool) -> bool {
        let Ok(mut mailboxes) = self.shared.mailboxes.try_lock() else {
            return false;
        };
        if mailboxes.input_enabled != enabled {
            mailboxes.input_enabled = enabled;
            mailboxes.capture_epoch = mailboxes.capture_epoch.wrapping_add(1);
            #[cfg(not(target_arch = "wasm32"))]
            {
                mailboxes.capture_started_at = std::time::SystemTime::now();
            }
            mailboxes.push_input(HostInputEvent::ReleaseAll);
        }
        #[cfg(target_arch = "wasm32")]
        set_host_input_enabled(
            enabled,
            mailboxes.status.input_poll_seconds * 1000.0,
            mailboxes.status.generation,
        );
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostRuntimeError {
    PoisonedMailboxes,
    WorkerStopped,
    StartupFailed,
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
                input_generation: status.generation,
                capture_epoch: 0,
                #[cfg(not(target_arch = "wasm32"))]
                capture_started_at: std::time::SystemTime::now(),
            }),
            video: VideoExchange::new(spare),
            stop: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
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
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn start_native_input(&self) -> bool {
        let Ok(mut target) = NATIVE_INPUT.try_lock() else {
            return false;
        };
        *target = Some(self.input_sender());
        NATIVE_LISTENER.call_once(|| {
            std::thread::spawn(crate::host_native_input::listen);
            std::thread::spawn(crate::host_native_input::listen_gamepads);
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
            self.shared.failed.store(true, Ordering::Release);
        }
    }

    pub fn submit(&mut self, command: HostCommand) -> Result<u64, HostBusy> {
        if self.generation_barrier.is_some()
            || self.status.shutdown
            || self.shared.failed.load(Ordering::Acquire)
            || self.shared.stopped.load(Ordering::Acquire)
        {
            return Err(HostBusy(command));
        }
        let changes_generation = matches!(
            &command,
            HostCommand::Reset
                | HostCommand::ResetWarm
                | HostCommand::LoadSnapshot(_)
                | HostCommand::LoadQuickSlot(_)
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

    pub fn poll(&mut self) -> Result<Vec<HostReply>, HostRuntimeError> {
        if self.shared.failed.load(Ordering::Acquire) {
            return Err(HostRuntimeError::WorkerStopped);
        }
        let mut mailboxes = match self.shared.mailboxes.try_lock() {
            Ok(mailboxes) => mailboxes,
            Err(TryLockError::WouldBlock) => return Ok(Vec::new()),
            Err(TryLockError::Poisoned(_)) => return Err(HostRuntimeError::PoisonedMailboxes),
        };
        if mailboxes.status.revision >= self.status.revision {
            self.status = mailboxes.status;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
            && !self.status.shutdown
        {
            return Err(HostRuntimeError::WorkerStopped);
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
            Err(TryLockError::Poisoned(_)) => return Err(VideoExchangeError::PoisonedStorage),
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
        if self.shared.failed.load(Ordering::Acquire) {
            return Err(HostRuntimeError::WorkerStopped);
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
                Some(Ok(battery)) => Ok(Some(battery)),
                Some(Err(_)) => Err(HostRuntimeError::WorkerStopped),
                None => Ok(None),
            }
        }
    }
}

impl Drop for HostRuntime {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(mut target) = NATIVE_INPUT.lock() {
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
                #[cfg(not(target_arch = "wasm32"))]
                if mailboxes.status.generation != status.generation {
                    mailboxes.capture_started_at = std::time::SystemTime::now();
                    mailboxes.capture_epoch = mailboxes.capture_epoch.wrapping_add(1);
                    mailboxes.input.clear();
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
        shared.failed.store(true, Ordering::Release);
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
    loop {
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
            WorkerWait::Stop => break,
        }
    }
    worker.shared.stopped.store(true, Ordering::Release);
    context.request_repaint();
    worker.owner.status().battery
}

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
    fn set_host_input_enabled(enabled: bool, interval_ms: f64, generation: u64);
    fn host_now() -> f64;
    fn notify_gui();
}

#[cfg(target_arch = "wasm32")]
fn start_web(
    owner: MachineOwner,
    shared: Arc<SharedHost>,
    context: egui::Context,
) -> Result<web_sys::Worker, HostRuntimeError> {
    use wasm_bindgen::closure::Closure;
    const BIT_MOUSE_LEFT: u32 = 1;
    const BIT_MOUSE_RIGHT: u32 = 1 << 1;
    const BIT_MOUSE_MIDDLE: u32 = 1 << 2;
    const BIT_JOYSTICK_UP: u32 = 1;
    const BIT_JOYSTICK_DOWN: u32 = 1 << 1;
    const BIT_JOYSTICK_LEFT: u32 = 1 << 2;
    const BIT_JOYSTICK_RIGHT: u32 = 1 << 3;
    const BIT_JOYSTICK_FIRE1: u32 = 1 << 4;
    const BIT_JOYSTICK_FIRE2: u32 = 1 << 5;

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
    };
    let input = Closure::<dyn FnMut(String, String, i32, i32, u32, u64) -> bool>::new(
        move |kind: String, code: String, dx, dy, bits, generation| {
            let event = match kind.as_str() {
                "keydown" | "keyup" => {
                    let Some(code) = crate::host_keys::amiga_key_code(&code) else {
                        return true;
                    };
                    HostInputEvent::Key {
                        code,
                        pressed: kind == "keydown",
                    }
                }
                "motion" => HostInputEvent::MouseMotion { dx, dy },
                "mouse" => HostInputEvent::MouseButtons {
                    left: bits & BIT_MOUSE_LEFT != 0,
                    right: bits & BIT_MOUSE_RIGHT != 0,
                    middle: bits & BIT_MOUSE_MIDDLE != 0,
                },
                "joystick" => HostInputEvent::Joystick {
                    up: bits & BIT_JOYSTICK_UP != 0,
                    down: bits & BIT_JOYSTICK_DOWN != 0,
                    left: bits & BIT_JOYSTICK_LEFT != 0,
                    right: bits & BIT_JOYSTICK_RIGHT != 0,
                    fire1: bits & BIT_JOYSTICK_FIRE1 != 0,
                    fire2: bits & BIT_JOYSTICK_FIRE2 != 0,
                },
                "release" => HostInputEvent::ReleaseAll,
                _ => return true,
            };
            sender.try_send(event, Some(generation))
        },
    );
    let finished = Closure::<dyn FnMut(bool)>::new(move |success: bool| {
        if !success {
            finished_shared.failed.store(true, Ordering::Release);
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
