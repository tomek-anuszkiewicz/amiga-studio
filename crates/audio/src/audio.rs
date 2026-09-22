//! Paula 4-Channel 8-Bit DMA Audio Subsystem
//!
//! 8-bit signed PCM sample streaming, period clock division, 6-bit linear volume scaling,
//! cross-channel volume and period modulation via ADKCON, and stereo channel mixing with ring buffer.

use serde::{Deserialize, Serialize};

/// Maximum capacity of the audio stereo output ring buffer
pub const AUDIO_RING_BUFFER_CAPACITY: usize = 1024;

/// Stereo audio sample (Left and Right channels)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StereoSample {
    pub left: i16,
    pub right: i16,
}

/// State of an individual Paula audio channel (0..3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AudioChannel {
    /// Sample location pointer in Chip RAM (AUDxLCH/AUDxLCL)
    pub lc: u32,
    /// Current DMA fetch pointer in Chip RAM
    pub current_pt: u32,
    /// Total sample length in 16-bit words (AUDxLEN)
    pub len: u16,
    /// Words remaining in current playback buffer loop
    pub words_remaining: u16,
    /// Period clock divider (AUDxPER, clock ticks per sample output)
    pub per: u16,
    /// 6-bit volume scale (AUDxVOL: 0 to 64 linear)
    pub vol: u8,
    /// 16-bit sample data holding latch (AUDxDAT)
    pub dat: u16,
    /// Current down-counter for sample clocking
    pub counter: u16,
    /// Byte selector within AUDxDAT: 0 = High Byte (first), 1 = Low Byte (second)
    pub sample_byte: u8,
    /// Active 8-bit signed PCM sample currently driving the DAC
    pub current_sample: i8,
    /// DMA enabled for this channel via DMACON (AUDxEN and DMAEN)
    pub dma_enabled: bool,
    /// True while channel DMA / playback is active
    pub active: bool,
    /// True when channel needs a new 16-bit word from DMA or CPU
    pub dma_request: bool,
    /// Level 4 audio interrupt pending flag for this channel
    pub irq_pending: bool,
    /// DMA restart strobe for Agnus pointer reload (AUDxDSR)
    pub restart_strobe: bool,
}

impl AudioChannel {
    /// Resets channel state to power-on default
    pub fn reset(&mut self) {
        self.lc = 0;
        self.current_pt = 0;
        self.len = 0;
        self.words_remaining = 0;
        self.per = 0;
        self.vol = 0;
        self.dat = 0;
        self.counter = 0;
        self.sample_byte = 0;
        self.current_sample = 0;
        self.dma_enabled = false;
        self.active = false;
        self.dma_request = false;
        self.irq_pending = false;
        self.restart_strobe = false;
    }

    /// Computes the volume-scaled output sample (-128..127 scaled by 0..64)
    #[inline]
    pub fn output_scaled(&self) -> i16 {
        if !self.active || self.per == 0 {
            0
        } else {
            let sample = self.current_sample as i32;
            let vol = self.vol.min(64) as i32;
            ((sample * vol) / 64) as i16
        }
    }

    /// Advances the channel clock by 1 tick.
    /// Returns true if the channel has consumed both bytes of AUDxDAT and requests a new word.
    #[inline]
    pub fn tick(&mut self) -> bool {
        if !self.active || self.per == 0 {
            return false;
        }

        if self.counter > 0 {
            self.counter = self.counter.wrapping_sub(1);
        }

        if self.counter == 0 {
            self.counter = self.per;

            if self.sample_byte == 0 {
                // First sample: high byte
                self.current_sample = ((self.dat >> 8) & 0xFF) as i8;
                self.sample_byte = 1;
                false
            } else {
                // Second sample: low byte
                self.current_sample = (self.dat & 0xFF) as i8;
                self.sample_byte = 0;
                // Finished both bytes of AUDxDAT -> request next word
                true
            }
        } else {
            false
        }
    }
}

#[inline]
fn default_ring_buffer() -> [StereoSample; AUDIO_RING_BUFFER_CAPACITY] {
    [StereoSample { left: 0, right: 0 }; AUDIO_RING_BUFFER_CAPACITY]
}

/// 4-channel DMA audio engine
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audio {
    /// 4 independent audio channels (0: Right, 1: Left, 2: Left, 3: Right)
    pub channels: [AudioChannel; 4],
    /// Active Audio & Disk Control register bits (ADKCON bits 0..7)
    pub adkcon: u16,
    /// Ring buffer storing mixed stereo output samples
    #[serde(skip, default = "default_ring_buffer")]
    pub ring_buffer: [StereoSample; AUDIO_RING_BUFFER_CAPACITY],
    /// Ring buffer write index
    pub ring_write_pos: usize,
    /// Ring buffer read index
    pub ring_read_pos: usize,
    /// Number of samples currently held in the ring buffer
    pub ring_count: usize,
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

impl Audio {
    /// Creates a new audio subsystem with all 4 channels silent
    pub fn new() -> Self {
        Self {
            channels: [AudioChannel::default(); 4],
            adkcon: 0,
            ring_buffer: [StereoSample::default(); AUDIO_RING_BUFFER_CAPACITY],
            ring_write_pos: 0,
            ring_read_pos: 0,
            ring_count: 0,
        }
    }

    /// Resets all audio channels and buffers
    pub fn reset(&mut self) {
        for ch in &mut self.channels {
            ch.reset();
        }
        self.adkcon = 0;
        self.ring_write_pos = 0;
        self.ring_read_pos = 0;
        self.ring_count = 0;
    }

    /// Updates the ADKCON register state (for volume and period modulation)
    #[inline]
    pub fn set_adkcon(&mut self, val: u16) {
        self.adkcon = val;
    }

    /// Sets DMA enabled state for a specific channel (0..3)
    #[inline]
    pub fn set_channel_dma(&mut self, channel: usize, enabled: bool) {
        if channel < 4 {
            self.channels[channel].dma_enabled = enabled;
            if enabled {
                if !self.channels[channel].active {
                    // Arm DMA channel: reload pointer and length
                    self.channels[channel].current_pt = self.channels[channel].lc;
                    self.channels[channel].words_remaining = self.channels[channel].len;
                    self.channels[channel].dma_request = true;
                    self.channels[channel].active = true;
                }
            } else {
                self.channels[channel].active = false;
                self.channels[channel].dma_request = false;
            }
        }
    }

    /// Action method: synchronizes all 4 channel DMA enables from DMACON
    pub fn set_dma_enables(&mut self, channel_mask: u8, master_enabled: bool) {
        for ch in 0..4 {
            let enabled = master_enabled && ((channel_mask & (1 << ch)) != 0);
            self.set_channel_dma(ch, enabled);
        }
    }

    /// Polls and clears the DMA restart strobe (`AUDxDSR`) for audio channel `ch`
    #[inline]
    pub fn poll_restart_strobe(&mut self, ch: usize) -> bool {
        if ch < 4 {
            let strobe = self.channels[ch].restart_strobe;
            self.channels[ch].restart_strobe = false;
            strobe
        } else {
            false
        }
    }

    /// Polls and clears pending Level 4 interrupt request for channel `ch`
    #[inline]
    pub fn poll_channel_irq(&mut self, ch: usize) -> bool {
        if ch < 4 {
            let irq = self.channels[ch].irq_pending;
            self.channels[ch].irq_pending = false;
            irq
        } else {
            false
        }
    }

    /// Loads a 16-bit word fetched via DMA or written directly, applying ADKCON modulation
    pub fn load_word(&mut self, channel: usize, word: u16) {
        if channel >= 4 {
            return;
        }

        // ADKCON cross-channel modulation (channels 0..2 modulating channels 1..3)
        if channel < 3 {
            let vol_mod = (self.adkcon & (1 << channel)) != 0;
            let per_mod = (self.adkcon & (1 << (4 + channel))) != 0;

            if vol_mod {
                // Use channel data to modulate subsequent channel's volume
                let target = channel + 1;
                self.channels[target].vol = (word & 0x007F).min(64) as u8;
                return;
            }

            if per_mod {
                // Use channel data to modulate subsequent channel's period
                let target = channel + 1;
                self.channels[target].per = word;
                self.channels[target].counter = word;
                return;
            }
        }

        // Standard PCM sample data
        self.channels[channel].dat = word;
        self.channels[channel].active = true;
        self.channels[channel].sample_byte = 0;
    }

    /// Advances the audio state machine by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        for ch in 0..4 {
            if self.channels[ch].tick() {
                if self.channels[ch].dma_enabled {
                    self.channels[ch].dma_request = true;
                    if self.channels[ch].words_remaining > 0 {
                        self.channels[ch].words_remaining -= 1;
                    }
                    if self.channels[ch].words_remaining == 0 {
                        // Reached end of sample buffer: reload and trigger Level 4 IRQ
                        self.channels[ch].irq_pending = true;
                        self.channels[ch].restart_strobe = true;
                        self.channels[ch].words_remaining = self.channels[ch].len;
                        self.channels[ch].current_pt = self.channels[ch].lc;
                    }
                } else {
                    // Manual (non-DMA) mode: trigger interrupt when buffer is consumed
                    self.channels[ch].irq_pending = true;
                }
            }
        }

        // Mix stereo channels:
        // Channels 1 and 2 -> Left
        // Channels 0 and 3 -> Right
        let left = self.channels[1]
            .output_scaled()
            .saturating_add(self.channels[2].output_scaled());
        let right = self.channels[0]
            .output_scaled()
            .saturating_add(self.channels[3].output_scaled());

        self.push_sample(StereoSample { left, right });
    }

    /// Advances audio state and executes cycle-by-cycle DMA fetches from Chip RAM
    pub fn step_cck_ram(&mut self, chip_ram: &[u8]) {
        for ch in 0..4 {
            if self.channels[ch].dma_enabled && self.channels[ch].dma_request {
                let pt = self.channels[ch].current_pt as usize;
                if pt + 1 < chip_ram.len() {
                    let word = u16::from_be_bytes([chip_ram[pt], chip_ram[pt + 1]]);
                    self.load_word(ch, word);
                    self.channels[ch].current_pt = self.channels[ch].current_pt.wrapping_add(2);
                    self.channels[ch].dma_request = false;
                }
            }
        }

        self.step_cck();
    }

    /// Pushes a stereo sample into the ring buffer
    #[inline]
    pub fn push_sample(&mut self, sample: StereoSample) {
        self.ring_buffer[self.ring_write_pos] = sample;
        self.ring_write_pos = (self.ring_write_pos + 1) % AUDIO_RING_BUFFER_CAPACITY;
        if self.ring_count < AUDIO_RING_BUFFER_CAPACITY {
            self.ring_count += 1;
        } else {
            // Buffer full: overwrite oldest sample and advance read pointer
            self.ring_read_pos = (self.ring_read_pos + 1) % AUDIO_RING_BUFFER_CAPACITY;
        }
    }

    /// Pops a stereo sample from the ring buffer
    #[inline]
    pub fn pop_sample(&mut self) -> Option<StereoSample> {
        if self.ring_count > 0 {
            let sample = self.ring_buffer[self.ring_read_pos];
            self.ring_read_pos = (self.ring_read_pos + 1) % AUDIO_RING_BUFFER_CAPACITY;
            self.ring_count -= 1;
            Some(sample)
        } else {
            None
        }
    }

    /// Returns the number of unread samples in the ring buffer
    #[inline]
    pub fn samples_available(&self) -> usize {
        self.ring_count
    }

    /// Sets sample location pointer for channel
    #[inline]
    pub fn set_loc(&mut self, channel: usize, loc: u32) {
        if channel < 4 {
            self.channels[channel].lc = loc;
            self.channels[channel].current_pt = loc;
        }
    }

    /// Sets sample length for channel
    #[inline]
    pub fn set_len(&mut self, channel: usize, len: u16) {
        if channel < 4 {
            self.channels[channel].len = len;
            self.channels[channel].words_remaining = len;
        }
    }

    /// Sets period divider for channel
    #[inline]
    pub fn set_per(&mut self, channel: usize, per: u16) {
        if channel < 4 {
            self.channels[channel].per = per;
            self.channels[channel].counter = per;
        }
    }

    /// Sets volume (0..64) for channel
    #[inline]
    pub fn set_vol(&mut self, channel: usize, vol: u8) {
        if channel < 4 {
            self.channels[channel].vol = vol.min(64);
        }
    }

    /// Sets data latch for channel
    #[inline]
    pub fn set_dat(&mut self, channel: usize, dat: u16) {
        if channel < 4 {
            self.load_word(channel, dat);
        }
    }
}
