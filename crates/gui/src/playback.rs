//! Host pacing and telemetry for complete video fields.

use std::time::Duration;

use crate::host_input::{InputConsumer, InputQueue, DELIVERIES_PER_FIELD};
use config::VideoStandard;
use debugger::{session::MAX_FRAME_SLICE_CCK, DebuggerSession};

const SPEEDS: [f64; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];
pub const MIN_SPEED: f64 = SPEEDS[0];
pub const MAX_SPEED: f64 = SPEEDS[SPEEDS.len() - 1];
const NORMAL_SPEED_INDEX: usize = 2;
const FPS_SAMPLE_SECONDS: f64 = 1.0;
const PAL_PRESENTATION_HZ: f64 = 50.0;
const NTSC_PRESENTATION_HZ: f64 = 60.0;

#[derive(Debug)]
pub struct Playback {
    speed_index: usize,
    next_field_time: Option<f64>,
    presentation_start: Option<f64>,
    next_presentation_time: Option<f64>,
    sample_start: Option<f64>,
    sample_fields: u64,
    fps: f64,
    delivery_index: u32,
    field_span: u32,
    field_origin: u64,
    pub input_deliveries: u64,
    pub last_input_cck: u64,
    pub(crate) input: InputConsumer,
    pub(crate) collected_input: InputQueue,
    pub(crate) reset_seen: bool,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            speed_index: NORMAL_SPEED_INDEX,
            next_field_time: None,
            presentation_start: None,
            next_presentation_time: None,
            sample_start: None,
            sample_fields: 0,
            fps: 0.0,
            delivery_index: 0,
            field_span: 0,
            field_origin: 0,
            input_deliveries: 0,
            last_input_cck: 0,
            input: Default::default(),
            collected_input: Default::default(),
            reset_seen: false,
        }
    }
}

impl Playback {
    pub fn speed(&self) -> f64 {
        SPEEDS[self.speed_index]
    }

    pub fn fps(&self) -> f64 {
        self.fps
    }

    pub fn slower(&mut self) {
        self.set_speed_index(self.speed_index.saturating_sub(1));
    }

    pub fn faster(&mut self) {
        self.set_speed_index((self.speed_index + 1).min(SPEEDS.len() - 1));
    }

    pub fn normal_speed(&mut self) {
        self.set_speed_index(NORMAL_SPEED_INDEX);
    }

    fn set_speed_index(&mut self, index: usize) {
        if index != self.speed_index {
            self.speed_index = index;
            self.next_field_time = None;
            self.presentation_start = None;
            self.next_presentation_time = None;
        }
    }

    /// Executes due fields, presenting only the last complete image of a batch.
    /// `now` is egui's host time; `elapsed` includes work since that UI update began.
    pub(crate) fn advance(
        &mut self,
        now: f64,
        session: &mut DebuggerSession,
        elapsed: impl Fn() -> f64,
    ) -> bool {
        self.reset_seen = false;
        if !session.is_running {
            self.reset_timing();
            return true;
        }

        let sample_start = *self.sample_start.get_or_insert(now);
        let sample_elapsed = now - sample_start;
        if sample_elapsed >= FPS_SAMPLE_SECONDS {
            self.fps = self.sample_fields as f64 / sample_elapsed;
            self.sample_fields = 0;
            self.sample_start = Some(now);
        }

        self.next_field_time.get_or_insert(now);
        if now < self.wake_time(now) && !session.pause_pending() {
            return false;
        }

        let accelerated = self.speed_index > NORMAL_SPEED_INDEX;
        let standard = session.machine.config.video_standard();
        let presentation_hz = match standard {
            VideoStandard::Pal => PAL_PRESENTATION_HZ,
            VideoStandard::Ntsc => NTSC_PRESENTATION_HZ,
        };
        let presentation_period = 1.0 / presentation_hz;
        let start = *self.presentation_start.get_or_insert(now);
        // Keep the original phase without accumulating floating-point additions.
        let tick = ((now - start) * presentation_hz).floor() + 1.0;
        let presentation_deadline = *self.next_presentation_time.get_or_insert_with(|| {
            let deadline = start + tick / presentation_hz;
            if deadline <= now {
                start + (tick + 1.0) / presentation_hz
            } else {
                deadline
            }
        });
        let mut deadline = self.next_field_time.unwrap_or(now);
        if accelerated {
            // A stalled host may catch up at most one presentation interval.
            deadline = deadline.max(now - presentation_period);
        }
        let frame = session.machine.denise.frame_builder.frame_number;
        if self.field_span == 0 && !session.machine.denise.sync.field_started() {
            // Align the first grid with receiver field assembly, including after reset.
            // A guest suppressing synchronization must not prevent input indefinitely.
            let start_cck = session.machine.cck;
            let limit = session.machine.agnus.field_ccks() / DELIVERIES_PER_FIELD;
            while session.is_running
                && !session.machine.denise.sync.field_started()
                && session.machine.cck.wrapping_sub(start_cck) < u64::from(limit)
            {
                let previous = session.machine.cck;
                session.run_frame_slice(1);
                if session.machine.cck.wrapping_sub(previous) != 1 {
                    self.reset_timing();
                    self.reset_seen = true;
                    return true;
                }
            }
            deadline += session.machine.cck.wrapping_sub(start_cck) as f64
                / (f64::from(standard.color_clock_hz()) * self.speed());
        }
        let cck = session.machine.cck;
        if self.delivery_index == DELIVERIES_PER_FIELD {
            // Guest beam writes may prolong a field indefinitely. Keep serving input.
            self.delivery_index = 0;
        }
        if self.delivery_index == 0 {
            self.field_span = session.machine.agnus.field_ccks();
            self.field_origin = cck;
        }
        if self.delivery_index < DELIVERIES_PER_FIELD {
            self.input_deliveries = self.input_deliveries.wrapping_add(1);
            self.last_input_cck = cck;
            while let Some(event) = self.collected_input.pop() {
                self.input.pending.push(event);
            }
            self.delivery_index += 1;
        }
        let target = self.field_origin.wrapping_add(
            u64::from(self.field_span) * u64::from(self.delivery_index)
                / u64::from(DELIVERIES_PER_FIELD),
        );
        let budget = target.wrapping_sub(cck).min(MAX_FRAME_SLICE_CCK as u64) as usize;
        self.input.run_slice(session, budget);
        let executed = session.machine.cck.wrapping_sub(cck);
        if session.is_running && (executed == 0 || executed > budget as u64) {
            self.collected_input.clear();
            self.input.pending.clear();
            self.reset_timing();
            self.reset_seen = true;
            return true;
        }
        let fields = session
            .machine
            .denise
            .frame_builder
            .frame_number
            .wrapping_sub(frame);
        self.sample_fields = self.sample_fields.wrapping_add(fields);
        if !session.is_running {
            self.reset_timing();
            return true;
        }
        let period = executed as f64 / (f64::from(standard.color_clock_hz()) * self.speed());
        deadline += period;
        self.next_field_time = Some(if accelerated {
            deadline
        } else {
            deadline.max(now)
        });
        if fields == 0 {
            // A partial field can already have overwritten the previous batch image.
            return false;
        }
        self.delivery_index = 0;
        let time_limit_reached = accelerated && now + elapsed() >= presentation_deadline;
        if !accelerated || deadline >= presentation_deadline || time_limit_reached {
            self.next_presentation_time = None;
            return true;
        }
        // Yield before the next input boundary, even when already overdue.
        return false;
    }

    pub(crate) fn reset_timing(&mut self) {
        self.next_field_time = None;
        self.presentation_start = None;
        self.next_presentation_time = None;
        self.sample_start = None;
        self.sample_fields = 0;
        self.fps = 0.0;
        self.delivery_index = 0;
        self.field_span = 0;
    }

    fn wake_time(&self, now: f64) -> f64 {
        self.next_field_time.unwrap_or(now)
    }

    /// Host-worker wait without the GUI integration's predicted-frame adjustment.
    pub(crate) fn worker_delay(&self, now: f64) -> Duration {
        Duration::from_secs_f64((self.wake_time(now) - now).max(0.0))
    }
}
