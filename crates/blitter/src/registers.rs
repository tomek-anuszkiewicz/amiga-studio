//! Blitter register-triggered operation setup.

use super::{Blitter, BlitterPhase};

impl Blitter {
    /// Action method: triggers blit execution when BLTSIZE matures
    pub fn trigger_blit(&mut self, bltsize: u16) {
        self.bltsize = bltsize;
        self.is_busy = true;
        self.is_zero = true;
        self.startup_cycles = 1;

        let width = match bltsize & 0x003F {
            0 => 64,
            w => w as usize,
        };
        let height = match bltsize >> 6 {
            0 => 1024,
            h => h as usize,
        };

        self.total_cols = width;
        self.total_rows = height;
        self.cur_row = 0;
        self.cur_col = 0;
        self.aold = 0;
        self.bold = 0;
        self.fill_carry = (self.bltcon1 & 0x0004) != 0;

        if (self.bltcon1 & 0x0001) != 0 {
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
