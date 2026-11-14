//! Timed stereo steps, BLEP reconstruction and bounded host PCM storage.

use config::VideoStandard;
use serde::{Deserialize, Deserializer, Serialize};

#[path = "audio_tables.rs"]
mod tables;
use tables::{
    BLEP_A500_NTSC_FILTER_OFF, BLEP_A500_NTSC_FILTER_ON, BLEP_A500_PAL_FILTER_OFF,
    BLEP_A500_PAL_FILTER_ON, BLEP_TABLE_LENGTH,
};

pub const AUDIO_SAMPLE_RATE: u32 = 44_100;
/// Stereo pairs: more than one longest PAL field at 44.1 kHz.
pub const AUDIO_RING_BUFFER_CAPACITY: usize = 1024;
const BLEP_SCALE: i64 = 1 << 17;
/// Two full-volume signed eight-bit channels fill each signed 16-bit side.
const PCM_GAIN: i64 = 2;
const CHANNELS_PER_SIDE: i32 = config::AUDIO_CHANNEL_COUNT as i32 / 2;
const MAX_STEREO_LEVEL: i32 = CHANNELS_PER_SIDE * (-(i8::MIN as i32)) * u8::MAX as i32;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StereoSample {
    pub left: i16,
    pub right: i16,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
struct StereoStep {
    cck: u64,
    left: i32,
    right: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct OutputState {
    standard: VideoStandard,
    cck: u64,
    phase: u32,
    filter_enabled: bool,
    left: i32,
    right: i32,
    #[serde(with = "step_storage")]
    steps: Box<[StereoStep; BLEP_TABLE_LENGTH]>,
    step_head: usize,
    step_count: usize,
    #[serde(with = "config::big_array")]
    samples: [StereoSample; AUDIO_RING_BUFFER_CAPACITY],
    read_pos: usize,
    write_pos: usize,
    sample_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub(super) struct AudioOutput {
    state: OutputState,
}

impl<'de> Deserialize<'de> for AudioOutput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let state = OutputState::deserialize(deserializer)?;
        if state.phase >= state.standard.color_clock_hz()
            || state.step_head >= BLEP_TABLE_LENGTH
            || state.step_count > BLEP_TABLE_LENGTH
            || state.read_pos >= AUDIO_RING_BUFFER_CAPACITY
            || state.write_pos >= AUDIO_RING_BUFFER_CAPACITY
            || state.sample_count > AUDIO_RING_BUFFER_CAPACITY
            || state.left.abs_diff(0) > MAX_STEREO_LEVEL as u32
            || state.right.abs_diff(0) > MAX_STEREO_LEVEL as u32
            || state.write_pos != (state.read_pos + state.sample_count) % AUDIO_RING_BUFFER_CAPACITY
        {
            return Err(serde::de::Error::custom(
                "Invalid audio phase or buffer positions",
            ));
        }
        let mut previous_age = u64::MAX / 2;
        for offset in 0..state.step_count {
            let step = state.steps[(state.step_head + offset) % BLEP_TABLE_LENGTH];
            let age = state.cck.wrapping_sub(step.cck);
            if age >= previous_age
                || step.left.abs_diff(0) > (2 * MAX_STEREO_LEVEL) as u32
                || step.right.abs_diff(0) > (2 * MAX_STEREO_LEVEL) as u32
            {
                return Err(serde::de::Error::custom(
                    "Audio steps must have ordered past times",
                ));
            }
            previous_age = age;
        }
        Ok(Self { state })
    }
}

impl AudioOutput {
    pub(super) fn new(standard: VideoStandard) -> Self {
        Self {
            state: OutputState {
                standard,
                cck: 0,
                phase: 0,
                filter_enabled: false,
                left: 0,
                right: 0,
                steps: Box::new([StereoStep::default(); BLEP_TABLE_LENGTH]),
                step_head: 0,
                step_count: 0,
                samples: [StereoSample::default(); AUDIO_RING_BUFFER_CAPACITY],
                read_pos: 0,
                write_pos: 0,
                sample_count: 0,
            },
        }
    }

    pub(super) fn reset(&mut self) {
        let state = &mut self.state;
        state.cck = 0;
        state.phase = 0;
        state.filter_enabled = false;
        state.left = 0;
        state.right = 0;
        state.steps.fill(StereoStep::default());
        state.step_head = 0;
        state.step_count = 0;
        state.samples.fill(StereoSample::default());
        state.read_pos = 0;
        state.write_pos = 0;
        state.sample_count = 0;
    }

    pub(super) fn set_filter_enabled(&mut self, enabled: bool) {
        self.state.filter_enabled = enabled;
    }

    /// Render the elapsed interval before the channel transitions at its end.
    pub(super) fn advance_cck(&mut self) {
        self.state.cck = self.state.cck.wrapping_add(1);
        self.state.phase = self.state.phase.wrapping_add(AUDIO_SAMPLE_RATE);
        let clock_hz = self.state.standard.color_clock_hz();
        if self.state.phase >= clock_hz {
            self.state.phase -= clock_hz;
            let sample = self.render();
            self.push_sample(sample);
        }
    }

    pub(super) fn record_levels(&mut self, left: i32, right: i32) {
        let state = &mut self.state;
        let left_step = left - state.left;
        let right_step = right - state.right;
        if left_step == 0 && right_step == 0 {
            return;
        }
        state.left = left;
        state.right = right;
        // At most one combined stereo step per CCK fits the complete table tail.
        while state.step_count != 0
            && state.cck.wrapping_sub(state.steps[state.step_head].cck) >= BLEP_TABLE_LENGTH as u64
        {
            state.step_head = (state.step_head + 1) % BLEP_TABLE_LENGTH;
            state.step_count -= 1;
        }
        if state.step_count != 0 {
            let last = (state.step_head + state.step_count - 1) % BLEP_TABLE_LENGTH;
            if state.steps[last].cck == state.cck {
                state.steps[last].left += left_step;
                state.steps[last].right += right_step;
                if state.steps[last].left == 0 && state.steps[last].right == 0 {
                    state.step_count -= 1;
                }
                return;
            }
        }
        let next = (state.step_head + state.step_count) % BLEP_TABLE_LENGTH;
        state.steps[next] = StereoStep {
            cck: state.cck,
            left: left_step,
            right: right_step,
        };
        state.step_count += 1;
    }

    fn render(&mut self) -> StereoSample {
        let state = &mut self.state;
        let table = match (state.standard, state.filter_enabled) {
            (VideoStandard::Pal, false) => &BLEP_A500_PAL_FILTER_OFF,
            (VideoStandard::Pal, true) => &BLEP_A500_PAL_FILTER_ON,
            (VideoStandard::Ntsc, false) => &BLEP_A500_NTSC_FILTER_OFF,
            (VideoStandard::Ntsc, true) => &BLEP_A500_NTSC_FILTER_ON,
        };
        let rate = i64::from(AUDIO_SAMPLE_RATE);
        let scale = BLEP_SCALE * rate;
        let mut left = i64::from(state.left) * scale;
        let mut right = i64::from(state.right) * scale;
        // phase/rate is the fraction of a CCK after this PCM sample's instant.
        while state.step_count != 0
            && state.cck.wrapping_sub(state.steps[state.step_head].cck) > BLEP_TABLE_LENGTH as u64
        {
            state.step_head = (state.step_head + 1) % BLEP_TABLE_LENGTH;
            state.step_count -= 1;
        }
        for offset in 0..state.step_count {
            let step = state.steps[(state.step_head + offset) % BLEP_TABLE_LENGTH];
            let age = state.cck.wrapping_sub(step.cck) as i64 * rate - i64::from(state.phase);
            let index = (age / rate) as usize;
            if index < BLEP_TABLE_LENGTH {
                let fraction = age % rate;
                let first = i64::from(table[index]);
                let next = i64::from(table.get(index + 1).copied().unwrap_or(0));
                let correction = first * (rate - fraction) + next * fraction;
                left -= i64::from(step.left) * correction;
                right -= i64::from(step.right) * correction;
            }
        }
        StereoSample {
            left: (left * PCM_GAIN / scale).clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16,
            right: (right * PCM_GAIN / scale).clamp(i64::from(i16::MIN), i64::from(i16::MAX))
                as i16,
        }
    }

    fn push_sample(&mut self, sample: StereoSample) {
        let state = &mut self.state;
        state.samples[state.write_pos] = sample;
        state.write_pos = (state.write_pos + 1) % AUDIO_RING_BUFFER_CAPACITY;
        if state.sample_count == AUDIO_RING_BUFFER_CAPACITY {
            state.read_pos = (state.read_pos + 1) % AUDIO_RING_BUFFER_CAPACITY;
        } else {
            state.sample_count += 1;
        }
    }

    pub(super) fn pop_sample(&mut self) -> Option<StereoSample> {
        let state = &mut self.state;
        if state.sample_count == 0 {
            return None;
        }
        let sample = state.samples[state.read_pos];
        state.read_pos = (state.read_pos + 1) % AUDIO_RING_BUFFER_CAPACITY;
        state.sample_count -= 1;
        Some(sample)
    }

    pub(super) fn samples_available(&self) -> usize {
        self.state.sample_count
    }
}

/// Allocate fixed history only during construction or restore, keeping large
/// arrays off the machine snapshot's stack. Stepping and reset reuse storage.
mod step_storage {
    use super::{StereoStep, BLEP_TABLE_LENGTH};
    use serde::de::{Error, IgnoredAny, SeqAccess, Visitor};
    use serde::{Deserializer, Serialize, Serializer};
    use std::fmt;

    pub(super) fn serialize<S: Serializer>(
        steps: &[StereoStep; BLEP_TABLE_LENGTH],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        steps.as_slice().serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Box<[StereoStep; BLEP_TABLE_LENGTH]>, D::Error> {
        struct StepVisitor;
        impl<'de> Visitor<'de> for StepVisitor {
            type Value = Box<[StereoStep; BLEP_TABLE_LENGTH]>;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "exactly {BLEP_TABLE_LENGTH} audio steps")
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut steps = Vec::with_capacity(BLEP_TABLE_LENGTH);
                for index in 0..BLEP_TABLE_LENGTH {
                    steps.push(
                        sequence
                            .next_element()?
                            .ok_or_else(|| A::Error::invalid_length(index, &self))?,
                    );
                }
                if sequence.next_element::<IgnoredAny>()?.is_some() {
                    return Err(A::Error::invalid_length(BLEP_TABLE_LENGTH + 1, &self));
                }
                steps
                    .into_boxed_slice()
                    .try_into()
                    .map_err(|_| A::Error::custom("Invalid audio history length"))
            }
        }
        deserializer.deserialize_seq(StepVisitor)
    }
}
