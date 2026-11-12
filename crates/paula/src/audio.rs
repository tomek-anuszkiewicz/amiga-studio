//! Four Paula audio state machines and fixed-capacity digital stereo output.

use serde::{Deserialize, Serialize};

#[path = "audio_channel.rs"]
mod channel;
use channel::Attachment;
pub use channel::{AudioChannel, AudioState};

pub const MAX_VOLUME: u8 = 64;
pub const MASK_VOLUME_REGISTER: u16 = 0x007f;
pub const AUDIO_RING_BUFFER_CAPACITY: usize = 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StereoSample {
    pub left: i16,
    pub right: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audio {
    pub channels: [AudioChannel; config::AUDIO_CHANNEL_COUNT],
    pub adkcon: u16,
    #[serde(with = "config::big_array")]
    pub ring_buffer: [StereoSample; AUDIO_RING_BUFFER_CAPACITY],
    pub ring_write_pos: usize,
    pub ring_read_pos: usize,
    pub ring_count: usize,
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

impl Audio {
    pub fn new() -> Self {
        Self {
            channels: [AudioChannel::default(); config::AUDIO_CHANNEL_COUNT],
            adkcon: 0,
            ring_buffer: [StereoSample::default(); AUDIO_RING_BUFFER_CAPACITY],
            ring_write_pos: 0,
            ring_read_pos: 0,
            ring_count: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn set_adkcon(&mut self, value: u16) {
        self.adkcon = value;
    }

    pub fn set_channel_dma(&mut self, channel: usize, enabled: bool) {
        if let Some(channel) = self.channels.get_mut(channel) {
            channel.set_dma_enabled(enabled);
        }
    }

    pub fn set_dma_enables(&mut self, mask: u8, master: bool) {
        for channel in 0..config::AUDIO_CHANNEL_COUNT {
            self.set_channel_dma(channel, master && mask & (1 << channel) != 0);
        }
    }

    fn attachment(&self, channel: usize) -> (bool, bool) {
        use crate::register_bits::audio_disk_control as control;
        (
            self.adkcon & (control::BIT_VOLUME_MODULATION_FIRST << channel) != 0,
            self.adkcon & (control::BIT_PERIOD_MODULATION_FIRST << channel) != 0,
        )
    }

    fn apply_attachment(&mut self, channel: usize, attachment: Attachment) {
        if channel + 1 == self.channels.len() {
            return;
        }
        match attachment {
            Attachment::None => {}
            Attachment::Volume(value) => {
                self.set_vol(channel + 1, (value & MASK_VOLUME_REGISTER) as u8)
            }
            Attachment::Period(value) => self.set_per(channel + 1, value),
        }
    }

    /// The interrupt mask is Paula's current AUD0..AUD3 pending state in bits 0..3.
    pub fn step_cck(&mut self, interrupts: u8) {
        for index in 0..config::AUDIO_CHANNEL_COUNT {
            let (volume, period) = self.attachment(index);
            let attachment =
                self.channels[index].step(interrupts & (1 << index) != 0, volume, period);
            self.apply_attachment(index, attachment);
        }
        let mut output = [0; config::AUDIO_CHANNEL_COUNT];
        for (index, sample) in output.iter_mut().enumerate() {
            if self.attachment(index) == (false, false) {
                *sample = self.channels[index].output_scaled();
            }
        }
        self.push_sample(StereoSample {
            left: output[1] + output[2],
            right: output[0] + output[3],
        });
    }

    pub fn poll_channel_irq(&mut self, channel: usize) -> bool {
        self.channels.get_mut(channel).is_some_and(|channel| {
            let pending = channel.irq_pending;
            channel.irq_pending = false;
            pending
        })
    }

    pub fn set_dat(&mut self, channel: usize, value: u16, interrupt: bool) {
        if channel < self.channels.len() {
            let (volume, period) = self.attachment(channel);
            let attachment = self.channels[channel].load_word(value, interrupt, volume, period);
            self.apply_attachment(channel, attachment);
        }
    }

    pub fn set_len(&mut self, channel: usize, value: u16) {
        if let Some(channel) = self.channels.get_mut(channel) {
            channel.len = value;
        }
    }

    pub fn set_per(&mut self, channel: usize, value: u16) {
        if let Some(channel) = self.channels.get_mut(channel) {
            channel.per = value;
        }
    }

    pub fn set_vol(&mut self, channel: usize, value: u8) {
        if let Some(channel) = self.channels.get_mut(channel) {
            channel.vol = (value & MASK_VOLUME_REGISTER as u8).min(MAX_VOLUME);
        }
    }

    pub fn push_sample(&mut self, sample: StereoSample) {
        self.ring_buffer[self.ring_write_pos] = sample;
        self.ring_write_pos = (self.ring_write_pos + 1) % AUDIO_RING_BUFFER_CAPACITY;
        if self.ring_count == AUDIO_RING_BUFFER_CAPACITY {
            self.ring_read_pos = (self.ring_read_pos + 1) % AUDIO_RING_BUFFER_CAPACITY;
        } else {
            self.ring_count += 1;
        }
    }

    pub fn pop_sample(&mut self) -> Option<StereoSample> {
        if self.ring_count == 0 {
            return None;
        }
        let sample = self.ring_buffer[self.ring_read_pos];
        self.ring_read_pos = (self.ring_read_pos + 1) % AUDIO_RING_BUFFER_CAPACITY;
        self.ring_count -= 1;
        Some(sample)
    }

    pub fn samples_available(&self) -> usize {
        self.ring_count
    }
}
