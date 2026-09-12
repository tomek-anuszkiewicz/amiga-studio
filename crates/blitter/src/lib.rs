//! Agnus 4-Channel DMA Blitter Emulation
//!
//! High-speed block image transfer, 256-minterm logic unit, barrel shifter,
//! and Bresenham line drawer.

use serde::{Deserialize, Serialize};

/// Agnus 4-channel DMA Blitter state and registers
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Blitter {
    /// Blitter Control 0 (channel enables A-D, minterms LF0-LF7, shift A)
    pub bltcon0: u16,
    /// Blitter Control 1 (shift B, descending flag, line mode, fill mode)
    pub bltcon1: u16,
    /// Blitter First Word Mask for Channel A
    pub bltafwm: u16,
    /// Blitter Last Word Mask for Channel A
    pub bltalwm: u16,
    /// Channel A source pointer (18-bit Chip RAM address)
    pub bltapt: u32,
    /// Channel B source pointer (18-bit Chip RAM address)
    pub bltbpt: u32,
    /// Channel C source/destination pointer (18-bit Chip RAM address)
    pub bltcpt: u32,
    /// Channel D destination pointer (18-bit Chip RAM address)
    pub bltdpt: u32,
    /// Blit size register: Height in rows (bits 6-15) and Width in words (bits 0-5)
    pub bltsize: u16,
    /// Channel A modulo (signed 16-bit)
    pub bltamod: i16,
    /// Channel B modulo (signed 16-bit)
    pub bltbmod: i16,
    /// Channel C modulo (signed 16-bit)
    pub bltcmod: i16,
    /// Channel D modulo (signed 16-bit)
    pub bltdmod: i16,
    /// Channel A data holding latch
    pub bltadat: u16,
    /// Channel B data holding latch
    pub bltbdat: u16,
    /// Channel C data holding latch
    pub bltcdat: u16,
    /// True while a blit operation is active
    pub is_busy: bool,
    /// True if all output words of the blit were zero (for collision/cookie cut)
    pub is_zero: bool,
}

impl Blitter {
    /// Creates a new uninitialized Blitter instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets all Blitter registers to power-on defaults
    pub fn reset(&mut self) {
        self.bltcon0 = 0;
        self.bltcon1 = 0;
        self.bltafwm = 0;
        self.bltalwm = 0;
        self.bltapt = 0;
        self.bltbpt = 0;
        self.bltcpt = 0;
        self.bltdpt = 0;
        self.bltsize = 0;
        self.bltamod = 0;
        self.bltbmod = 0;
        self.bltcmod = 0;
        self.bltdmod = 0;
        self.bltadat = 0;
        self.bltbdat = 0;
        self.bltcdat = 0;
        self.is_busy = false;
        self.is_zero = true;
    }

    /// Triggers a new blit operation by writing BLTSIZE
    #[inline]
    pub fn start_blit(&mut self, bltsize: u16) {
        self.bltsize = bltsize;
        self.is_busy = true;
        self.is_zero = true;
    }

    /// Advances the Blitter state by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        // Scaffold placeholder: advances channels when DMA slot is granted
    }
}
