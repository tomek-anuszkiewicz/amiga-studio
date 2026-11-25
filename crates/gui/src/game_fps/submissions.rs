//! Count distinct completed field images on retained Game drawing passes.

use crate::host_video::FrameInfo;
use serde::Serialize;

const WARMUP_IMAGES: u64 = 32;
const SAMPLE_SECONDS: f64 = 10.0;

#[derive(Debug, Clone, Copy, Serialize)]
pub(super) struct Sample {
    submissions: u64,
    seconds: f64,
    deadline_delay_seconds: f64,
}

#[derive(Debug, Clone)]
pub(super) struct Submissions {
    generation: u64,
    last_field: u64,
    warmup: u64,
    deadline: Option<f64>,
    count: u64,
    pub(super) sample: Option<Sample>,
}

impl Submissions {
    pub(super) fn new(generation: u64, boot_field: u64) -> Self {
        Self {
            generation,
            last_field: boot_field,
            warmup: 0,
            deadline: None,
            count: 0,
            sample: None,
        }
    }

    /// Return the next deadline wakeup. An absent image can close the interval.
    pub(super) fn observe(
        &mut self,
        retained: bool,
        drawn: Option<FrameInfo>,
        now: f64,
    ) -> Option<f64> {
        if !retained || self.sample.is_some() {
            return None;
        }
        if let Some(deadline) = self.deadline {
            if now >= deadline {
                self.sample = Some(Sample {
                    submissions: self.count,
                    seconds: SAMPLE_SECONDS,
                    deadline_delay_seconds: now - deadline,
                });
                return Some(0.0);
            }
        }
        if let Some(info) = drawn.filter(|info| {
            info.complete
                && info.generation == self.generation
                && info.field_number > self.last_field
        }) {
            self.last_field = info.field_number;
            if self.deadline.is_some() {
                self.count += 1;
            } else {
                self.warmup += 1;
                if self.warmup == WARMUP_IMAGES {
                    self.deadline = Some(now + SAMPLE_SECONDS);
                }
            }
        }
        self.deadline.map(|deadline| deadline - now)
    }
}
