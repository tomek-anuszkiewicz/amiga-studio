//! Conditions for recognizing Mixed Scene startup and rejecting interrupted sessions.

use crate::host_commands::HostStatus;
use crate::ScreenScale;
use serde::Serialize;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

pub const BOOT_SPEED: f64 = 1.0;
pub const DOUBLE_SPEED: f64 = 2.0;
pub const MEASUREMENT_SPEED: f64 = 4.0;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Default)]
pub struct StartupFocus {
    focused_since: Option<Instant>,
}

#[cfg(not(target_arch = "wasm32"))]
impl StartupFocus {
    pub(super) fn observe(&mut self, focused: bool, now: Instant) -> bool {
        if !focused {
            self.focused_since = None;
            return false;
        }
        let since = *self.focused_since.get_or_insert(now);
        now.duration_since(since) >= Duration::from_millis(300)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub(super) fields: u32,
    pub(super) work_loops: u32,
    pub(super) fault: u32,
    pub(super) cck: u64,
    pub(super) field_number: u64,
}

impl Progress {
    pub(super) fn advances(self, earlier: Self) -> bool {
        self.fields > earlier.fields
            && self.work_loops > earlier.work_loops
            && self.cck > earlier.cck
            && self.field_number > earlier.field_number
            && self.fault == 0
            && earlier.fault == 0
    }
}

#[derive(Debug, Default)]
pub struct Boot {
    first: Option<Progress>,
}

impl Boot {
    pub(super) fn observe(
        &mut self,
        signature_matches: bool,
        progress: Progress,
    ) -> Result<Option<(Progress, Progress)>, String> {
        if !signature_matches {
            self.first = None;
            return Ok(None);
        }
        if progress.fault != 0 {
            return Err("Mixed Scene reported a fault during boot".into());
        }
        let first = *self.first.get_or_insert(progress);
        Ok(progress.advances(first).then_some((first, progress)))
    }
}

pub fn validate_session(
    generation: Option<u64>,
    current_generation: u64,
    focused: bool,
) -> Result<(), String> {
    if !focused {
        return Err("The Game window lost focus".into());
    }
    if generation.is_some_and(|generation| generation != current_generation) {
        return Err("Machine generation changed during the benchmark".into());
    }
    Ok(())
}

pub fn validate_running(status: HostStatus) -> Result<(), String> {
    if status.shutdown || status.halted {
        return Err("Machine owner stopped unexpectedly".into());
    }
    if !status.running || !status.continuous_run || status.pause_pending {
        return Err("The Game session stopped continuous execution".into());
    }
    Ok(())
}

pub fn validate_game_view(in_game: bool, pause_requested: bool) -> Result<(), String> {
    if !in_game {
        return Err("The session left Game view".into());
    }
    if pause_requested {
        return Err("The Game session was paused".into());
    }
    Ok(())
}

pub fn validate_speed(actual: f64, expected: f64) -> Result<(), String> {
    if actual != expected {
        return Err(format!(
            "Expected Game speed {expected}×, observed {actual}×"
        ));
    }
    Ok(())
}

#[derive(Debug)]
pub struct SpeedChange {
    previous_speed: f64,
    pub(super) target_speed: f64,
    acknowledged: bool,
}

impl SpeedChange {
    pub(super) fn new(previous_speed: f64, target_speed: f64) -> Self {
        Self {
            previous_speed,
            target_speed,
            acknowledged: false,
        }
    }

    pub(super) fn acknowledge(&mut self, status: HostStatus) -> Result<(), String> {
        validate_running(status)?;
        validate_speed(status.speed, self.target_speed)?;
        self.acknowledged = true;
        Ok(())
    }

    pub(super) fn confirmed(&self, speed: f64) -> Result<bool, String> {
        if !self.acknowledged && speed == self.previous_speed {
            return Ok(false);
        }
        validate_speed(speed, self.target_speed)?;
        Ok(self.acknowledged)
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Display {
    viewport_points: [f32; 2],
    pixels_per_point: f32,
    screen_scale: ScreenScale,
}

impl Display {
    pub(super) fn current(ctx: &egui::Context, screen_scale: ScreenScale) -> Self {
        let rect = ctx.viewport_rect();
        Self {
            viewport_points: [rect.width(), rect.height()],
            pixels_per_point: ctx.pixels_per_point(),
            screen_scale,
        }
    }

    pub(super) fn validate(&self, current: &Self) -> Result<(), String> {
        if self != current {
            return Err("Game viewport or scale changed during warmup or sampling".into());
        }
        Ok(())
    }
}
