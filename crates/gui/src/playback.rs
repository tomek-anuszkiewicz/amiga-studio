//! Host pacing and telemetry for complete video fields.

use std::time::Duration;

use config::VideoStandard;
use debugger::{session::MAX_FRAME_SLICE_CCK, DebuggerSession};

const SPEEDS: [f64; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];
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
    field_ccks: u64,
    sample_start: Option<f64>,
    sample_fields: u64,
    fps: f64,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            speed_index: NORMAL_SPEED_INDEX,
            next_field_time: None,
            presentation_start: None,
            next_presentation_time: None,
            field_ccks: 0,
            sample_start: None,
            sample_fields: 0,
            fps: 0.0,
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

    pub fn can_slow_down(&self) -> bool {
        self.speed_index > 0
    }

    pub fn can_speed_up(&self) -> bool {
        self.speed_index + 1 < SPEEDS.len()
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
        if self.field_ccks == 0 && now < self.wake_time(now) && !session.pause_pending() {
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
        let mut presentation_deadline = start + tick / presentation_hz;
        if presentation_deadline <= now {
            presentation_deadline = start + (tick + 1.0) / presentation_hz;
        }
        self.next_presentation_time = Some(presentation_deadline);
        let mut deadline = self.next_field_time.unwrap_or(now);
        if accelerated {
            // A stalled host may catch up at most one presentation interval.
            deadline = deadline.max(now - presentation_period);
        }
        loop {
            let frame = session.machine.denise.frame_builder.frame_number;
            let cck = session.machine.cck;
            session.run_frame_slice(MAX_FRAME_SLICE_CCK);
            self.field_ccks = self
                .field_ccks
                .wrapping_add(session.machine.cck.wrapping_sub(cck));
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
            if fields == 0 {
                // A partial field can already have overwritten the previous batch image.
                return false;
            }
            let clock_hz = standard.color_clock_hz();
            let period = self.field_ccks as f64 / (f64::from(clock_hz) * self.speed());
            deadline += period;
            self.next_field_time = Some(if accelerated {
                deadline
            } else {
                deadline.max(now)
            });
            self.field_ccks = 0;
            let time_limit_reached = accelerated && now + elapsed() >= presentation_deadline;
            if !accelerated || deadline >= presentation_deadline || time_limit_reached {
                return true;
            }
        }
    }

    fn reset_timing(&mut self) {
        self.next_field_time = None;
        self.presentation_start = None;
        self.next_presentation_time = None;
        self.field_ccks = 0;
        self.sample_start = None;
        self.sample_fields = 0;
        self.fps = 0.0;
    }

    fn wake_time(&self, now: f64) -> f64 {
        let field_time = self.next_field_time.unwrap_or(now);
        if self.speed_index > NORMAL_SPEED_INDEX {
            field_time.min(self.next_presentation_time.unwrap_or(now))
        } else {
            field_time
        }
    }

    pub(crate) fn request_repaint(&self, ctx: &egui::Context, session: &DebuggerSession, now: f64) {
        if session.is_running {
            let delay = if self.field_ccks != 0 || session.pause_pending() {
                0.0
            } else {
                (self.wake_time(now) - now).max(0.0)
            };
            // egui subtracts predicted_dt from repaint delays. Our deadline already
            // accounts for work elapsed, and native presentation has vsync disabled.
            let predicted = ctx.input(|input| input.predicted_dt);
            ctx.request_repaint_after(
                Duration::from_secs_f64(delay) + Duration::from_secs_f32(predicted),
            );
        }
    }
}
