//! Blitter register-triggered operation setup.

use super::{Blitter, BlitterPhase};
use config::{blitter_fields as field, mask::bltcon};

const STARTUP_CCK: u8 = 1;
const MASK_WIDTH: u16 = 0x003F;
const BIT_HEIGHT_SHIFT: u16 = 6;
const MAX_WIDTH_WORDS: usize = 64;
const MAX_HEIGHT_ROWS: usize = 1024;

impl Blitter {
    /// Action method: triggers blit execution when BLTSIZE matures
    pub fn trigger_blit(&mut self, bltsize: u16) {
        self.bltsize = bltsize;
        self.is_busy = true;
        self.is_zero = true;
        self.startup_cycles = STARTUP_CCK;

        let width = match bltsize & MASK_WIDTH {
            0 => MAX_WIDTH_WORDS,
            w => w as usize,
        };
        let height = match bltsize >> BIT_HEIGHT_SHIFT {
            0 => MAX_HEIGHT_ROWS,
            h => h as usize,
        };

        self.total_cols = width;
        self.total_rows = height;
        self.cur_row = 0;
        self.cur_col = 0;
        self.aold = 0;
        self.bold = 0;
        self.fill_carry = (self.bltcon1 & field::BIT_FILL_CARRY_IN) != 0;

        if (self.bltcon1 & bltcon::BIT_LINE) != 0 {
            // Line mode
            self.line_pixels_left = height;
            self.line_drawer.reset();
            self.anew = self.bltadat;
            self.bnew = self.bltbdat;
            self.active_phase = BlitterPhase::LinePixel;
        } else {
            // Area mode
            self.phase_index = 0;
            self.active_phase = self.active_word_phases().0[0];
        }
    }
}
