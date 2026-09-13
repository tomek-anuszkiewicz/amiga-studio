//! Denise 8 Hardware Sprite Engines
//!
//! 16-pixel wide hardware sprites, vertical start/stop comparators,
//! sprite pairing for 15-color mode, and multiplexing.

use config::BeamPosition;
use serde::{Deserialize, Serialize};

/// State for a single hardware sprite channel (0..7)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SpriteChannel {
    /// Sprite DMA Pointer (SPRxPTH / SPRxPTL)
    pub pt: u32,
    /// Sprite Position (SPRxPOS: VSTART in bits 15..8, HSTART in bits 7..0)
    pub pos: u16,
    /// Sprite Control (SPRxCTL: VSTOP in bits 15..8, ATTACH in bit 7)
    pub ctl: u16,
    /// Sprite image data low latch (SPRxDATA)
    pub data_a: u16,
    /// Sprite image data high latch (SPRxDATB)
    pub data_b: u16,
    /// True if sprite DMA is armed and actively displayed
    pub is_armed: bool,
}

impl SpriteChannel {
    /// Returns the vertical start scanline (9-bit value)
    #[inline]
    pub fn vstart(&self) -> u16 {
        let low = (self.pos >> 8) & 0xFF;
        let high = (self.ctl >> 2) & 0x01;
        (high << 8) | low
    }

    /// Returns the vertical stop scanline (9-bit value)
    #[inline]
    pub fn vstop(&self) -> u16 {
        let low = (self.ctl >> 8) & 0xFF;
        let high = (self.ctl >> 1) & 0x01;
        (high << 8) | low
    }

    /// Returns the horizontal start Color Clock position (9-bit value)
    #[inline]
    pub fn hstart(&self) -> u16 {
        let low = self.pos & 0xFF;
        let high = self.ctl & 0x01;
        (low << 1) | high
    }

    /// Returns true if this odd sprite channel is attached to its even partner
    #[inline]
    pub fn is_attached(&self) -> bool {
        (self.ctl & 0x0080) != 0
    }
}

/// Aggregate structure managing all 8 hardware sprite channels
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Sprites {
    /// 8 independent hardware sprite channels (Sprites 0 to 7)
    pub channels: [SpriteChannel; 8],
    /// Sprite DMA enabled via DMACON (SPREN bit 5 and DMAEN bit 9)
    pub dma_enabled: bool,
}

impl Sprites {
    /// Creates a new uninitialized sprites subsystem
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets all 8 sprite channels to power-on default
    pub fn reset(&mut self) {
        for ch in &mut self.channels {
            *ch = SpriteChannel::default();
        }
        self.dma_enabled = false;
    }

    /// Sets Sprite DMA enabled state from DMACON
    #[inline]
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        self.dma_enabled = enabled;
        if !enabled {
            for ch in &mut self.channels {
                ch.is_armed = false;
            }
        }
    }

    /// Advances Sprite engine state by 1 Color Clock observing beam coordinates
    #[inline]
    pub fn step_cck(&mut self, _beam: BeamPosition) {
        // Sprite comparator matching and serialization
    }

    /// Sets sprite position register (SPRxPOS)
    #[inline]
    pub fn set_pos(&mut self, ch: usize, val: u16) {
        if ch < 8 {
            self.channels[ch].pos = val;
        }
    }

    /// Sets sprite control register (SPRxCTL)
    #[inline]
    pub fn set_ctl(&mut self, ch: usize, val: u16) {
        if ch < 8 {
            self.channels[ch].ctl = val;
        }
    }

    /// Sets sprite image data (SPRxDATA and SPRxDATB)
    #[inline]
    pub fn set_data(&mut self, ch: usize, data_a: u16, data_b: u16) {
        if ch < 8 {
            self.channels[ch].data_a = data_a;
            self.channels[ch].data_b = data_b;
            self.channels[ch].is_armed = true;
        }
    }
}
