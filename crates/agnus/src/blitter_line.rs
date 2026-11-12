//! HRM chapter 6 line coordinates, octants and signed error accumulation.

use crate::register_bits::{blitter_fields as field, bltcon};
use config::chip_ram::MASK_OCS_DMA_ADDRESS;
use serde::{Deserialize, Serialize};

const WORD_BYTES: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LineDrawer {
    pub first_pixel_on_line: bool,
}

impl LineDrawer {
    pub fn new() -> Self {
        Self {
            first_pixel_on_line: true,
        }
    }

    pub fn reset(&mut self) {
        self.first_pixel_on_line = true;
    }

    /// Move on the major axis and, for a non-negative error, on the minor axis.
    pub fn step_pixel(
        &mut self,
        bltcon0: &mut u16,
        bltcon1: &mut u16,
        bltapt: &mut u32,
        bltamod: i16,
        bltbmod: i16,
        bltcmod: i16,
        bltcpt: &mut u32,
    ) {
        let horizontal_major = *bltcon1 & field::BIT_LINE_SUD != 0;
        let negative_error = *bltcon1 & field::BIT_LINE_SIGN != 0;
        let major_negative = *bltcon1 & field::BIT_LINE_AUL != 0;
        let minor_negative = *bltcon1 & field::BIT_LINE_SUL != 0;
        let move_x = horizontal_major || !negative_error;
        let move_y = !horizontal_major || !negative_error;
        let mut shift = (*bltcon0 >> field::BIT_SHIFT) & field::MASK_SHIFT_VALUE;
        if move_x {
            let negative = if horizontal_major {
                major_negative
            } else {
                minor_negative
            };
            if negative {
                if shift == 0 {
                    *bltcpt = bltcpt.wrapping_sub(WORD_BYTES);
                }
                shift = shift.wrapping_sub(1) & field::MASK_SHIFT_VALUE;
            } else {
                if shift == field::MASK_SHIFT_VALUE {
                    *bltcpt = bltcpt.wrapping_add(WORD_BYTES);
                }
                shift = shift.wrapping_add(1) & field::MASK_SHIFT_VALUE;
            }
        }
        if move_y {
            let negative = if horizontal_major {
                minor_negative
            } else {
                major_negative
            };
            let stride = bltcmod as i32 as u32;
            *bltcpt = if negative {
                bltcpt.wrapping_sub(stride)
            } else {
                bltcpt.wrapping_add(stride)
            };
        }
        *bltcpt &= MASK_OCS_DMA_ADDRESS;
        self.first_pixel_on_line = move_y;
        if *bltcon0 & bltcon::BIT_USEA != 0 {
            let adjustment = if negative_error { bltbmod } else { bltamod };
            *bltapt = bltapt.wrapping_add(adjustment as i32 as u32) & MASK_OCS_DMA_ADDRESS;
        }
        *bltcon1 &= !field::BIT_LINE_SIGN;
        if (*bltapt as i16) < 0 {
            *bltcon1 |= field::BIT_LINE_SIGN;
        }
        *bltcon0 = (*bltcon0 & field::MASK_CONTROL_PRESERVE) | (shift << field::BIT_SHIFT);
    }
}
