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
    /// Blitter DMA channel enabled via DMACON (BLTEN bit 6 and DMAEN bit 9)
    pub dma_enabled: bool,
    /// Blitter Nasty / CPU priority mode (BLTPRI bit 10 in DMACON)
    pub bltpri: bool,
    /// Level 3 blitter interrupt request strobe (_BLITINT)
    pub blit_irq: bool,
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
        self.dma_enabled = false;
        self.bltpri = false;
        self.blit_irq = false;
    }

    /// Sets Blitter DMA enabled state from DMACON
    #[inline]
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        self.dma_enabled = enabled;
        if !enabled {
            self.is_busy = false;
        }
    }

    /// Sets Blitter Nasty priority mode from DMACON (BLTPRI, bit 10)
    #[inline]
    pub fn set_bltpri(&mut self, enabled: bool) {
        self.bltpri = enabled;
    }

    /// Synchronizes channel pointers from Agnus registers
    #[inline]
    pub fn sync_pointers(&mut self, apt: u32, bpt: u32, cpt: u32, dpt: u32) {
        self.bltapt = apt;
        self.bltbpt = bpt;
        self.bltcpt = cpt;
        self.bltdpt = dpt;
    }

    /// Synchronizes control registers and channel modulos from Agnus
    #[inline]
    pub fn sync_controls(
        &mut self,
        con0: u16,
        con1: u16,
        afwm: u16,
        alwm: u16,
        amod: i16,
        bmod: i16,
        cmod: i16,
        dmod: i16,
    ) {
        self.bltcon0 = con0;
        self.bltcon1 = con1;
        self.bltafwm = afwm;
        self.bltalwm = alwm;
        self.bltamod = amod;
        self.bltbmod = bmod;
        self.bltcmod = cmod;
        self.bltdmod = dmod;
    }

    /// Triggers a new blit operation by writing BLTSIZE
    #[inline]
    pub fn start_blit(&mut self, bltsize: u16) {
        self.trigger_blit(bltsize);
    }

    /// Action method: triggers blit execution when BLTSIZE matures
    #[inline]
    pub fn trigger_blit(&mut self, bltsize: u16) {
        self.bltsize = bltsize;
        self.is_busy = true;
        self.is_zero = true;
    }

    /// Action method: finishes the active blit, resets busy, and asserts Level 3 `_BLITINT`
    #[inline]
    pub fn finish_blit(&mut self) {
        self.is_busy = false;
        self.blit_irq = true;
    }

    /// Polls and clears the blitter interrupt request strobe (_BLITINT)
    #[inline]
    pub fn poll_blit_irq(&mut self) -> bool {
        let irq = self.blit_irq;
        self.blit_irq = false;
        irq
    }

    /// Advances the Blitter state by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        // Scaffold placeholder: advances channels when DMA slot is granted
    }
}
