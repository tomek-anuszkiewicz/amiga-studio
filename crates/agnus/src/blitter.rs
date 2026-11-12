//! Agnus 4-Channel DMA Blitter Emulation
//!
//! High-speed block image transfer, 256-minterm logic unit, barrel shifter,
//! and Bresenham line drawer.

#[path = "blitter_line.rs"]
pub mod line;
#[path = "blitter_minterm.rs"]
pub mod minterm;
#[path = "blitter_phase.rs"]
pub mod phase;

#[path = "blitter_area.rs"]
mod area;
#[path = "blitter_line_engine.rs"]
mod line_engine;
#[path = "blitter_control.rs"]
mod registers;

use crate::register_bits::{blitter_fields as field, bltcon};
pub use line::LineDrawer;
pub use minterm::{apply_fill, eval_minterm};
use phase::PendingBlitWord;
pub use phase::{barrel_shift, word_phases, BlitterPhase, MAX_WORD_PHASES};

use serde::{Deserialize, Serialize};

const RESET_AB_DATA: u16 = 0xAAAA;
const RESET_C_DATA: u16 = 0x5555;

/// Agnus 4-channel DMA Blitter state and registers
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Blitter {
    /// Active phase index within the current word's cycle sequence (0..3)
    pub phase_index: u8,
    /// Blitter Control 0 (channel enables A-D, minterms LF0-LF7, shift A)
    pub bltcon0: u16,
    /// Blitter Control 1 (shift B, descending flag, line mode, fill mode)
    pub bltcon1: u16,
    /// Blitter First Word Mask for Channel A
    pub bltafwm: u16,
    /// Blitter Last Word Mask for Channel A
    pub bltalwm: u16,
    /// Channel A source pointer (18-bit word address, stored as an even byte address)
    pub bltapt: u32,
    /// Channel B source pointer (18-bit word address, stored as an even byte address)
    pub bltbpt: u32,
    /// Channel C source/destination pointer (18-bit word address, stored as an even byte address)
    pub bltcpt: u32,
    /// Channel D destination pointer (18-bit word address, stored as an even byte address)
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

    /// Previous masked source A word for cross-word shifting.
    pub aold: u16,
    /// Previous source B word for cross-word shifting.
    pub bold: u16,
    /// B shifter output, also loaded by CPU BLTBDAT writes.
    pub bhold: u16,
    /// Most recently computed logic/fill result.
    pub dhold: u16,
    pub fill_carry: bool,
    pub(crate) pending_word: Option<PendingBlitWord>,
    pub(crate) draining: bool,
    pub(crate) line_write_enabled: bool,

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

    /// Current row index during area blit
    pub cur_row: usize,
    /// Current column index (in words) during area blit
    pub cur_col: usize,
    /// Total rows in active blit
    pub total_rows: usize,
    /// Total words per row in active blit
    pub total_cols: usize,
    /// Bresenham line drawer state
    pub line_drawer: LineDrawer,
    /// Remaining pixels in line mode
    pub line_pixels_left: usize,
    /// Hardware pipeline startup cycles before word execution begins (BLT_STRT1, BLT_STRT2)
    pub startup_cycles: u8,
}

impl Blitter {
    /// Creates a new uninitialized Blitter instance
    pub fn new() -> Self {
        let mut b = Self::default();
        b.reset();
        b
    }

    /// Resets all Blitter registers to power-on defaults
    pub fn reset(&mut self) {
        self.phase_index = 0;
        self.startup_cycles = 0;
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
        self.bltadat = RESET_AB_DATA;
        self.bltbdat = RESET_AB_DATA;
        self.bltcdat = RESET_C_DATA;
        self.aold = 0;
        self.bold = 0;
        self.bhold = RESET_AB_DATA;
        self.pending_word = None;
        self.draining = false;
        self.line_write_enabled = false;
        self.dhold = 0;
        self.fill_carry = false;
        self.is_busy = false;
        self.is_zero = true;
        self.dma_enabled = false;
        self.bltpri = false;
        self.blit_irq = false;
        self.cur_row = 0;
        self.cur_col = 0;
        self.total_rows = 0;
        self.total_cols = 0;
        self.line_drawer.reset();
        self.line_pixels_left = 0;
    }

    /// Sets Blitter DMA enabled state from DMACON
    #[inline]
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        self.dma_enabled = enabled;
    }

    /// Sets Blitter Nasty priority mode from DMACON (BLTPRI, bit 10)
    #[inline]
    pub fn set_bltpri(&mut self, enabled: bool) {
        self.bltpri = enabled;
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

    /// Returns the local phase without changing the operation.
    pub fn current_phase(&self) -> BlitterPhase {
        if !self.is_busy {
            return BlitterPhase::Idle;
        }
        if self.startup_cycles != 0 {
            return BlitterPhase::BusIdle;
        }
        if self.bltcon1 & bltcon::BIT_LINE != 0 {
            return self.line_phase();
        }
        if self.draining {
            return if self.pending_word.is_some() {
                BlitterPhase::WriteD
            } else {
                BlitterPhase::BusIdle
            };
        }
        let (phases, count) = self.active_word_phases();
        let phase = phases[usize::from(self.phase_index.min(count - 1))];
        if phase == BlitterPhase::WriteD && self.pending_word.is_none() {
            BlitterPhase::BusIdle
        } else {
            phase
        }
    }

    /// Only an actual source read or destination write requests Chip RAM.
    pub fn wants_bus(&self) -> bool {
        self.dma_enabled
            && matches!(
                self.current_phase(),
                BlitterPhase::FetchA
                    | BlitterPhase::FetchB
                    | BlitterPhase::FetchC
                    | BlitterPhase::WriteD
            )
    }

    /// Advances internal work without supplying a memory grant.
    #[inline]
    pub fn step_cck(&mut self) {
        self.step_cck_ram(&mut [], false);
    }

    /// One CCK, with at most one word transferred when Agnus grants the bus.
    pub fn step_cck_ram(&mut self, chip_ram: &mut [u8], bus_granted: bool) {
        if !self.is_busy || !self.dma_enabled {
            return;
        }
        if self.startup_cycles != 0 {
            self.startup_cycles = self.startup_cycles.wrapping_sub(1);
            return;
        }
        if self.wants_bus() && (!bus_granted || chip_ram.is_empty()) {
            return;
        }
        if self.bltcon1 & bltcon::BIT_LINE != 0 {
            self.step_line_cycle(chip_ram);
        } else {
            self.step_area_cycle(chip_ram);
        }
    }

    fn active_word_phases(&self) -> ([BlitterPhase; phase::MAX_WORD_PHASES], u8) {
        word_phases(
            self.bltcon0 & bltcon::BIT_USEA != 0,
            self.bltcon0 & bltcon::BIT_USEB != 0,
            self.bltcon0 & bltcon::BIT_USEC != 0,
            self.bltcon0 & bltcon::BIT_USED != 0,
            self.bltcon1 & field::MASK_FILL != 0,
        )
    }

    /// HRM 6: the write loads B's shifted holding value using current controls.
    pub fn write_bltbdat(&mut self, word: u16) {
        self.bltbdat = word;
        let shift = (self.bltcon1 >> field::BIT_SHIFT) & field::MASK_SHIFT_VALUE;
        self.bhold = barrel_shift(word, self.bold, shift, self.bltcon1 & bltcon::BIT_DESC != 0);
        self.bold = word;
    }
}
