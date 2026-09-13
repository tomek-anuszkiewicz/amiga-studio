//! Paula 4-Channel 8-Bit DMA Audio Subsystem
//!
//! Independent period counters, 6-bit volume scaling (0..64),
//! left/right stereo channel assignment, and DMA audio fetches.

use serde::{Deserialize, Serialize};

/// State of an individual Paula audio channel (0..3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AudioChannel {
    /// Sample location pointer in Chip RAM (AUDxLCH/AUDxLCL)
    pub lc: u32,
    /// Sample length in 16-bit words (AUDxLEN)
    pub len: u16,
    /// Period clock divider (AUDxPER, clock ticks per sample output)
    pub per: u16,
    /// 6-bit volume scale (AUDxVOL: 0 to 64 linear)
    pub vol: u8,
    /// 16-bit sample data holding latch (AUDxDAT)
    pub dat: u16,
    /// Current down-counter for sample clocking
    pub counter: u16,
    /// DMA enabled for this channel via DMACON (AUDxEN and DMAEN)
    pub dma_enabled: bool,
    /// True while channel DMA / playback is active
    pub active: bool,
}

impl AudioChannel {
    /// Resets channel state to power-on default
    pub fn reset(&mut self) {
        self.lc = 0;
        self.len = 0;
        self.per = 0;
        self.vol = 0;
        self.dat = 0;
        self.counter = 0;
        self.dma_enabled = false;
        self.active = false;
    }
}

/// 4-channel DMA audio engine
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Audio {
    /// 4 independent audio channels (0: Right, 1: Left, 2: Left, 3: Right)
    pub channels: [AudioChannel; 4],
}

impl Audio {
    /// Creates a new audio subsystem with all 4 channels silent
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets all audio channels
    pub fn reset(&mut self) {
        for ch in &mut self.channels {
            ch.reset();
        }
    }

    /// Sets DMA enabled state for a specific channel (0..3)
    #[inline]
    pub fn set_channel_dma(&mut self, channel: usize, enabled: bool) {
        if channel < 4 {
            self.channels[channel].dma_enabled = enabled;
            if !enabled {
                self.channels[channel].active = false;
            }
        }
    }

    /// Action method: synchronizes all 4 channel DMA enables from DMACON (bits 0..3 and bit 9 DMAEN)
    pub fn set_dma_enables(&mut self, channel_mask: u8, master_enabled: bool) {
        for ch in 0..4 {
            let enabled = master_enabled && ((channel_mask & (1 << ch)) != 0);
            self.set_channel_dma(ch, enabled);
        }
    }

    /// Advances the audio state by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        for ch in &mut self.channels {
            if ch.active && ch.per > 0 {
                if ch.counter == 0 {
                    ch.counter = ch.per;
                    // Scaffold placeholder: emit sample to ring buffer
                } else {
                    ch.counter = ch.counter.wrapping_sub(1);
                }
            }
        }
    }

    /// Sets sample location pointer for channel
    #[inline]
    pub fn set_loc(&mut self, channel: usize, loc: u32) {
        if channel < 4 {
            self.channels[channel].lc = loc;
        }
    }

    /// Sets sample length for channel
    #[inline]
    pub fn set_len(&mut self, channel: usize, len: u16) {
        if channel < 4 {
            self.channels[channel].len = len;
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
            self.channels[channel].dat = dat;
            self.channels[channel].active = true;
        }
    }
}
