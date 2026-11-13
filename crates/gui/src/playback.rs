//! Host pacing and telemetry for complete video fields.

use std::time::Duration;

use debugger::{session::MAX_FRAME_SLICE_CCK, DebuggerSession};

const SPEEDS: [f64; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];
const NORMAL_SPEED_INDEX: usize = 2;
const FPS_SAMPLE_SECONDS: f64 = 1.0;

#[derive(Debug)]
pub struct Playback {
    speed_index: usize,
    next_field_time: Option<f64>,
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
        }
    }

    /// Executes one complete field when due. Unfinished slices retain the last image.
    /// `now` comes from egui's host clock, which is also controllable in headless tests.
    pub(crate) fn advance(&mut self, now: f64, session: &mut DebuggerSession) -> bool {
        if !session.is_running {
            self.next_field_time = None;
            self.field_ccks = 0;
            self.sample_start = None;
            self.sample_fields = 0;
            self.fps = 0.0;
            return true;
        }

        let sample_start = *self.sample_start.get_or_insert(now);
        let elapsed = now - sample_start;
        if elapsed >= FPS_SAMPLE_SECONDS {
            self.fps = self.sample_fields as f64 / elapsed;
            self.sample_fields = 0;
            self.sample_start = Some(now);
        }

        let deadline = *self.next_field_time.get_or_insert(now);
        if self.field_ccks == 0 && now < deadline && !session.pause_pending() {
            return false;
        }

        let frame = session.machine.denise.frame_builder.frame_number;
        let cck = session.machine.cck;
        session.run_frame_slice(MAX_FRAME_SLICE_CCK);
        self.field_ccks = self
            .field_ccks
            .wrapping_add(session.machine.cck.wrapping_sub(cck));
        let completed = session.machine.denise.frame_builder.frame_number != frame;
        if completed {
            self.sample_fields = self.sample_fields.wrapping_add(1);
            let clock_hz = session.machine.config.video_standard().color_clock_hz();
            let period = self.field_ccks as f64 / (f64::from(clock_hz) * self.speed());
            // Keep phase through normal host jitter, but do not build an unbounded
            // backlog when rendering is slower than emulation's requested speed.
            self.next_field_time = Some((deadline + period).max(now));
            self.field_ccks = 0;
        }
        if !session.is_running {
            self.next_field_time = None;
            self.field_ccks = 0;
            self.sample_start = None;
            self.sample_fields = 0;
            self.fps = 0.0;
        }
        completed || !session.is_running
    }

    pub(crate) fn request_repaint(&self, ctx: &egui::Context, session: &DebuggerSession, now: f64) {
        if session.is_running {
            let delay = if self.field_ccks != 0 || session.pause_pending() {
                0.0
            } else {
                (self.next_field_time.unwrap_or(now) - now).max(0.0)
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
