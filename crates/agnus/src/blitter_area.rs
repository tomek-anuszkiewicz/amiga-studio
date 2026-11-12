//! Area transfers, source shifts and the destination pipeline from HRM chapter 6.

use super::{apply_fill, barrel_shift, eval_minterm, Blitter, BlitterPhase, PendingBlitWord};
use crate::register_bits::{blitter_fields as field, bltcon};
use config::chip_ram;

const WORD_BYTES: u32 = 2;

impl Blitter {
    pub(crate) fn step_area_cycle(&mut self, ram: &mut [u8]) {
        match self.current_phase() {
            BlitterPhase::FetchA => {
                self.bltadat = chip_ram::read_u16(ram, self.bltapt);
                self.bltapt = self.advance_pointer(self.bltapt, WORD_BYTES);
            }
            BlitterPhase::FetchB => {
                self.write_bltbdat(chip_ram::read_u16(ram, self.bltbpt));
                self.bltbpt = self.advance_pointer(self.bltbpt, WORD_BYTES);
            }
            BlitterPhase::FetchC => {
                self.bltcdat = chip_ram::read_u16(ram, self.bltcpt);
                self.bltcpt = self.advance_pointer(self.bltcpt, WORD_BYTES);
            }
            BlitterPhase::WriteD => self.write_pending_word(ram),
            _ => {}
        }
        if self.draining {
            self.finish_blit();
            return;
        }
        let (_, count) = self.active_word_phases();
        self.phase_index = self.phase_index.wrapping_add(1);
        if self.phase_index >= count {
            self.phase_index = 0;
            self.complete_area_word();
        }
    }

    fn complete_area_word(&mut self) {
        let last_in_row = self.cur_col + 1 >= self.total_cols;
        let mut a = self.bltadat;
        if self.cur_col == 0 {
            a &= self.bltafwm;
        }
        if last_in_row {
            a &= self.bltalwm;
        }
        let shifted_a = barrel_shift(
            a,
            self.aold,
            (self.bltcon0 >> field::BIT_SHIFT) & field::MASK_SHIFT_VALUE,
            self.bltcon1 & bltcon::BIT_DESC != 0,
        );
        self.aold = a;
        let mut result = eval_minterm(shifted_a, self.bhold, self.bltcdat, self.bltcon0 as u8);
        if self.bltcon1 & field::MASK_FILL != 0 {
            result = apply_fill(
                result,
                &mut self.fill_carry,
                self.bltcon1 & bltcon::BIT_EFE != 0,
            );
        }
        self.dhold = result;
        self.is_zero &= result == 0;
        if self.bltcon0 & bltcon::BIT_USED != 0 {
            self.pending_word = Some(PendingBlitWord {
                data: result,
                last_in_row,
            });
        }
        self.cur_col = self.cur_col.wrapping_add(1);
        if last_in_row {
            self.cur_col = 0;
            self.cur_row = self.cur_row.wrapping_add(1);
            self.apply_source_modulos();
            self.fill_carry = self.bltcon1 & field::BIT_FILL_CARRY_IN != 0;
        }
        if self.cur_row >= self.total_rows {
            if self.pending_word.is_some() {
                self.draining = true;
            } else {
                self.finish_blit();
            }
        }
    }

    fn write_pending_word(&mut self, ram: &mut [u8]) {
        if let Some(word) = self.pending_word.take() {
            chip_ram::write_u16(ram, self.bltdpt, word.data);
            self.bltdpt = self.advance_pointer(self.bltdpt, WORD_BYTES);
            if word.last_in_row {
                self.bltdpt = self.advance_pointer(self.bltdpt, self.bltdmod as i32 as u32);
            }
        }
    }

    fn apply_source_modulos(&mut self) {
        if self.bltcon0 & bltcon::BIT_USEA != 0 {
            self.bltapt = self.advance_pointer(self.bltapt, self.bltamod as i32 as u32);
        }
        if self.bltcon0 & bltcon::BIT_USEB != 0 {
            self.bltbpt = self.advance_pointer(self.bltbpt, self.bltbmod as i32 as u32);
        }
        if self.bltcon0 & bltcon::BIT_USEC != 0 {
            self.bltcpt = self.advance_pointer(self.bltcpt, self.bltcmod as i32 as u32);
        }
    }

    fn advance_pointer(&self, pointer: u32, amount: u32) -> u32 {
        let next = if self.bltcon1 & bltcon::BIT_DESC != 0 {
            pointer.wrapping_sub(amount)
        } else {
            pointer.wrapping_add(amount)
        };
        next & chip_ram::MASK_OCS_DMA_ADDRESS
    }
}
