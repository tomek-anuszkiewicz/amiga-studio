//! Denise 8 Hardware Sprite Engines
//!
//! 16-pixel wide hardware sprites, vertical start/stop comparators,
//! sprite pairing for 15-color mode, multiplexing, and collision detection.

use config::BeamPosition;
use serde::{Deserialize, Serialize};

/// Output pixel from the sprite mixer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpritePixel {
    /// Color palette index (COLOR16..COLOR31, values 16..31)
    pub color_index: usize,
    /// Sprite pair index (0..3)
    pub sprite_pair: usize,
}

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
    /// Active 16-bit shift register A
    pub shift_a: u16,
    /// Active 16-bit shift register B
    pub shift_b: u16,
    /// Number of remaining pixels to shift out (0..16)
    pub pixel_counter: u8,
    /// True if sprite horizontal comparator is armed
    pub is_armed: bool,
    /// True if current scanline is within vertical active window [VSTART..VSTOP)
    pub is_active_line: bool,
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

    /// Returns the horizontal start pixel position (9-bit value in low-res pixels)
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

    /// Evaluates vertical comparator at line start
    #[inline]
    pub fn update_scanline(&mut self, vpos: u16) {
        let vstart = self.vstart();
        let vstop = self.vstop();
        self.is_active_line = vpos >= vstart && vpos < vstop;
        self.pixel_counter = 0;
    }

    /// Checks horizontal comparator against current pixel coordinate
    #[inline]
    pub fn check_hstart(&mut self, hpos_pixel: u16) {
        if self.is_active_line && self.is_armed && hpos_pixel == self.hstart() {
            self.shift_a = self.data_a;
            self.shift_b = self.data_b;
            self.pixel_counter = 16;
        }
    }

    /// Shifts out 1 pixel (2 bits: bit 0 from A, bit 1 from B)
    #[inline]
    pub fn shift_pixel(&mut self) -> u8 {
        if self.pixel_counter > 0 {
            let bit_a = ((self.shift_a >> 15) & 1) as u8;
            let bit_b = ((self.shift_b >> 15) & 1) as u8;
            self.shift_a <<= 1;
            self.shift_b <<= 1;
            self.pixel_counter -= 1;
            (bit_b << 1) | bit_a
        } else {
            0
        }
    }
}

/// Aggregate structure managing all 8 hardware sprite channels
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Sprites {
    /// 8 independent hardware sprite channels (Sprites 0 to 7)
    pub channels: [SpriteChannel; 8],
    /// Sprite DMA enabled via DMACON (SPREN bit 5 and DMAEN bit 9)
    pub dma_enabled: bool,
    /// Collision control mask (CLXCON: bits 15..12 enable odd sprites in collisions)
    pub clxcon: u16,
}

impl Sprites {
    /// Creates a new uninitialized sprites subsystem
    pub fn new() -> Self {
        Self {
            channels: [SpriteChannel::default(); 8],
            dma_enabled: false,
            // By default on OCS, odd sprites participate if enabled in CLXCON
            clxcon: 0xF000,
        }
    }

    /// Resets all 8 sprite channels to power-on default
    pub fn reset(&mut self) {
        for ch in &mut self.channels {
            *ch = SpriteChannel::default();
        }
        self.dma_enabled = false;
        self.clxcon = 0xF000;
    }

    /// Sets the collision control register (CLXCON)
    #[inline]
    pub fn set_clxcon(&mut self, val: u16) {
        self.clxcon = val;
    }

    /// Sets Sprite DMA enabled state from DMACON
    #[inline]
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        if self.dma_enabled == enabled {
            return;
        }
        self.dma_enabled = enabled;
        if !enabled {
            for ch in &mut self.channels {
                ch.is_armed = false;
                ch.pixel_counter = 0;
            }
        }
    }

    /// Advances Sprite engine state by 1 Color Clock observing beam coordinates
    #[inline]
    pub fn step_cck(&mut self, beam: BeamPosition) {
        if beam.hpos == 0 {
            for ch in &mut self.channels {
                ch.update_scanline(beam.vpos);
            }
        }
    }

    /// Sets sprite position register (SPRxPOS)
    #[inline]
    pub fn set_pos(&mut self, ch: usize, val: u16) {
        if ch < 8 {
            self.channels[ch].pos = val;
        }
    }

    /// Sets sprite control register (SPRxCTL) - writing disables horizontal comparator
    #[inline]
    pub fn set_ctl(&mut self, ch: usize, val: u16) {
        if ch < 8 {
            self.channels[ch].ctl = val;
            self.channels[ch].is_armed = false;
        }
    }

    /// Sets sprite image data (SPRxDATA and SPRxDATB) - writing DATA arms comparator
    #[inline]
    pub fn set_data(&mut self, ch: usize, data_a: u16, data_b: u16) {
        if ch < 8 {
            self.channels[ch].data_a = data_a;
            self.channels[ch].data_b = data_b;
            self.channels[ch].is_armed = true;
        }
    }

    /// Evaluates all 8 sprites for a given pixel coordinate on scanline `vpos`.
    /// Returns the highest-priority non-transparent sprite pixel (if any) and updates CLXDAT.
    pub fn evaluate_pixel(&mut self, hpos_pixel: u16, clxdat: &mut u16) -> Option<SpritePixel> {
        let mut pixels = [0u8; 8];

        for (i, ch) in self.channels.iter_mut().enumerate() {
            ch.check_hstart(hpos_pixel);
            pixels[i] = ch.shift_pixel();
        }

        // Determine active sprite pairs for collision detection
        // Pairs: 0=(0,1), 1=(2,3), 2=(4,5), 3=(6,7)
        let mut pair_active = [false; 4];
        for pair in 0..4 {
            let ch_even = pair * 2;
            let ch_odd = ch_even + 1;
            let even_has = pixels[ch_even] != 0;
            // Odd sprite participates if attached or enabled via CLXCON bit (12 + pair)
            let odd_en =
                self.channels[ch_odd].is_attached() || ((self.clxcon & (1 << (12 + pair))) != 0);
            let odd_has = odd_en && (pixels[ch_odd] != 0);
            pair_active[pair] = even_has || odd_has;
        }

        // Sprite-to-Sprite collision detection in CLXDAT (HRM Table 7-3)
        // Bit 9: Pair 0 to Pair 1
        // Bit 10: Pair 0 to Pair 2
        // Bit 11: Pair 0 to Pair 3
        // Bit 12: Pair 1 to Pair 2
        // Bit 13: Pair 1 to Pair 3
        // Bit 14: Pair 2 to Pair 3
        if pair_active[0] && pair_active[1] {
            *clxdat |= 1 << 9;
        }
        if pair_active[0] && pair_active[2] {
            *clxdat |= 1 << 10;
        }
        if pair_active[0] && pair_active[3] {
            *clxdat |= 1 << 11;
        }
        if pair_active[1] && pair_active[2] {
            *clxdat |= 1 << 12;
        }
        if pair_active[1] && pair_active[3] {
            *clxdat |= 1 << 13;
        }
        if pair_active[2] && pair_active[3] {
            *clxdat |= 1 << 14;
        }

        // Output priority mixer: evaluate pairs 0..3 (Sprite 0 is highest priority)
        for pair in 0..4 {
            let ch_even = pair * 2;
            let ch_odd = ch_even + 1;

            if self.channels[ch_odd].is_attached() {
                // Attached 15-color sprite mode
                let p_even = pixels[ch_even];
                let p_odd = pixels[ch_odd];
                let combined = (p_odd << 2) | p_even;
                if combined != 0 {
                    return Some(SpritePixel {
                        color_index: 16 + combined as usize,
                        sprite_pair: pair,
                    });
                }
            } else {
                // Independent 3-color sprites (even has priority over odd partner)
                if pixels[ch_even] != 0 {
                    return Some(SpritePixel {
                        color_index: 16 + pair * 4 + pixels[ch_even] as usize,
                        sprite_pair: pair,
                    });
                }
                if pixels[ch_odd] != 0 {
                    return Some(SpritePixel {
                        color_index: 16 + pair * 4 + pixels[ch_odd] as usize,
                        sprite_pair: pair,
                    });
                }
            }
        }

        None
    }
}
