//! Warm up and time the owner's cumulative count of completed emulated fields.

use super::progress::MEASUREMENT_SPEED;
use serde::Serialize;
use std::time::Duration;

const WARMUP_FIELDS: u64 = 32;
const SAMPLE_SECONDS: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Sample {
    warmup_start_field: u64,
    start_field: u64,
    end_field: u64,
    seconds: f64,
    speed: f64,
}

#[derive(Debug)]
pub struct Sampler {
    warmup_start_field: u64,
    last_field: u64,
    start: Option<(u64, f64)>,
    pub(super) sample: Option<Sample>,
}

impl Sampler {
    pub(super) fn new(warmup_start_field: u64) -> Self {
        Self {
            warmup_start_field,
            last_field: warmup_start_field,
            start: None,
            sample: None,
        }
    }

    /// Return the time until the deadline, even if the next status shows the same field.
    pub(super) fn observe(
        &mut self,
        field_number: u64,
        now: f64,
    ) -> Result<Option<Duration>, String> {
        if self.sample.is_some() {
            return Ok(None);
        }
        if field_number < self.last_field {
            return Err("The owner's completed-field counter went backwards".into());
        }
        self.last_field = field_number;
        if field_number - self.warmup_start_field < WARMUP_FIELDS {
            return Ok(None);
        }
        let (start_field, started_at) = *self.start.get_or_insert((field_number, now));
        let elapsed = now - started_at;
        if elapsed < SAMPLE_SECONDS {
            return Ok(Some(Duration::from_secs_f64(SAMPLE_SECONDS - elapsed)));
        }
        self.sample = Some(Sample {
            warmup_start_field: self.warmup_start_field,
            start_field,
            end_field: field_number,
            seconds: elapsed,
            speed: MEASUREMENT_SPEED,
        });
        Ok(None)
    }
}
