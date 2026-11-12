//! Line mode: distinct C read, logic, D write and coordinate phases.

use super::{eval_minterm, Blitter, BlitterPhase};
use crate::register_bits::{blitter_fields as field, bltcon};
use config::chip_ram;

const PHASE_READ: u8 = 0;
const PHASE_LOGIC: u8 = 1;
const PHASE_WRITE: u8 = 2;
const PHASE_ADVANCE: u8 = 3;

impl Blitter {
    pub(crate) fn line_phase(&self) -> BlitterPhase {
        match self.phase_index {
            PHASE_READ if self.bltcon0 & bltcon::BIT_USEC != 0 => BlitterPhase::FetchC,
            PHASE_WRITE if self.line_write_enabled => BlitterPhase::WriteD,
            _ => BlitterPhase::BusIdle,
        }
    }

    pub(crate) fn step_line_cycle(&mut self, ram: &mut [u8]) {
        match self.phase_index {
            PHASE_READ => {
                if self.bltcon0 & bltcon::BIT_USEC != 0 {
                    self.bltcdat = chip_ram::read_u16(ram, self.bltcpt);
                }
            }
            PHASE_LOGIC => {
                let ash = (self.bltcon0 >> field::BIT_SHIFT) & field::MASK_SHIFT_VALUE;
                let bsh = (self.bltcon1 >> field::BIT_SHIFT) & field::MASK_SHIFT_VALUE;
                let a = (self.bltadat & self.bltafwm) >> ash;
                let b = 0u16.wrapping_sub((self.bltbdat >> bsh) & 1);
                self.dhold = eval_minterm(a, b, self.bltcdat, self.bltcon0 as u8);
                self.is_zero &= self.dhold == 0;
                self.line_write_enabled = self.bltcon0 & bltcon::BIT_USEC != 0
                    && (self.bltcon1 & field::BIT_LINE_SINGLE_DOT == 0
                        || self.line_drawer.first_pixel_on_line);
            }
            PHASE_WRITE => {
                if self.line_write_enabled {
                    chip_ram::write_u16(ram, self.bltdpt, self.dhold);
                }
            }
            PHASE_ADVANCE => {
                self.line_drawer.step_pixel(
                    &mut self.bltcon0,
                    &mut self.bltcon1,
                    &mut self.bltapt,
                    self.bltamod,
                    self.bltbmod,
                    self.bltcmod,
                    &mut self.bltcpt,
                );
                self.bltdpt = self.bltcpt;
                let bsh =
                    (self.bltcon1 >> field::BIT_SHIFT).wrapping_sub(1) & field::MASK_SHIFT_VALUE;
                self.bltcon1 =
                    (self.bltcon1 & field::MASK_CONTROL_PRESERVE) | (bsh << field::BIT_SHIFT);
                self.line_pixels_left = self.line_pixels_left.wrapping_sub(1);
                if self.line_pixels_left == 0 {
                    self.finish_blit();
                }
            }
            _ => {}
        }
        self.phase_index = if self.phase_index >= PHASE_ADVANCE {
            PHASE_READ
        } else {
            self.phase_index + 1
        };
    }
}
