//! Four Paula audio state machines and fixed-capacity digital stereo output.

use config::VideoStandard;
use serde::{Deserialize, Serialize};

#[path = "audio_channel.rs"]
mod channel;
use channel::Attachment;
pub use channel::{AudioChannel, AudioState};
#[path = "audio_output.rs"]
mod output;
use output::AudioOutput;
pub use output::{StereoSample, AUDIO_RING_BUFFER_CAPACITY, AUDIO_SAMPLE_RATE};

pub const MAX_VOLUME: u8 = 64;
pub const MASK_VOLUME_REGISTER: u16 = 0x007f;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audio {
    pub channels: [AudioChannel; config::AUDIO_CHANNEL_COUNT],
    pub adkcon: u16,
    output: AudioOutput,
}

impl Default for Audio {
    fn default() -> Self {
        Self::new(VideoStandard::Pal)
    }
}

impl Audio {
    pub fn new(standard: VideoStandard) -> Self {
        Self {
            channels: [AudioChannel::default(); config::AUDIO_CHANNEL_COUNT],
            adkcon: 0,
            output: AudioOutput::new(standard),
        }
    }

    pub fn reset(&mut self) {
        self.channels = [AudioChannel::default(); config::AUDIO_CHANNEL_COUNT];
        self.adkcon = 0;
        self.output.reset();
    }

    pub fn set_adkcon(&mut self, value: u16) {
        self.adkcon = value;
        self.record_output();
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
        self.output.advance_cck();
        for index in 0..config::AUDIO_CHANNEL_COUNT {
            let (volume, period) = self.attachment(index);
            let attachment =
                self.channels[index].step(interrupts & (1 << index) != 0, volume, period);
            self.apply_attachment(index, attachment);
        }
        self.record_output();
    }

    fn record_output(&mut self) {
        let mut output = [0i32; config::AUDIO_CHANNEL_COUNT];
        for (index, sample) in output.iter_mut().enumerate() {
            if self.attachment(index) == (false, false) {
                *sample = i32::from(self.channels[index].output_level());
            }
        }
        self.output
            .record_levels(output[1] + output[2], output[0] + output[3]);
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
            self.record_output();
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

    /// Selects the modeled motherboard filter without changing guest audio clocks.
    pub fn set_filter_enabled(&mut self, enabled: bool) {
        self.output.set_filter_enabled(enabled);
    }

    pub fn pop_sample(&mut self) -> Option<StereoSample> {
        self.output.pop_sample()
    }

    /// Drains oldest PCM pairs into caller-owned storage, without allocating.
    pub fn read_samples(&mut self, destination: &mut [StereoSample]) -> usize {
        let mut written = 0;
        for slot in destination {
            let Some(sample) = self.output.pop_sample() else {
                break;
            };
            *slot = sample;
            written += 1;
        }
        written
    }

    pub fn samples_available(&self) -> usize {
        self.output.samples_available()
    }
}
