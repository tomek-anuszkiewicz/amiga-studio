//! Bresenham Line Drawer Engine for Agnus Blitter
//!
//! Implements hardware vector line drawing, octant direction selection,
//! slope error accumulator calculation, and single-point mode.

use config::chip_ram::MASK_OCS_DMA_ADDRESS;
use config::{blitter_fields as field, mask::bltcon};
use serde::{Deserialize, Serialize};

/// Bresenham Line Drawer state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LineDrawer {
    /// True if the current pixel is the first plotted pixel on the current scanline
    pub first_pixel_on_line: bool,
}

impl LineDrawer {
    /// Creates a new LineDrawer instance
    pub fn new() -> Self {
        Self {
            first_pixel_on_line: true,
        }
    }

    /// Resets line drawer state for a new line blit
    pub fn reset(&mut self) {
        self.first_pixel_on_line = true;
    }

    /// Steps the Bresenham line drawer by 1 pixel.
    ///
    /// Updates:
    /// - `bltcon0` (ASH pixel shift 0..15)
    /// - `bltcon1` (SIGN flag bit 6, BSH texture shift)
    /// - `bltapt` (error accumulator)
    /// - `bltcpt` / `bltdpt` (word address pointers in Chip RAM)
    #[inline]
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
        let sud = (*bltcon1 & field::BIT_LINE_SUD) != 0;
        let sul = (*bltcon1 & field::BIT_LINE_SUL) != 0;
        let aul = (*bltcon1 & field::BIT_LINE_AUL) != 0;
        let sign = (*bltcon1 & field::BIT_LINE_SIGN) != 0;
        let use_a = (*bltcon0 & bltcon::BIT_USEA) != 0;

        let mut ash = (*bltcon0 >> field::BIT_SHIFT) & field::MASK_SHIFT_VALUE;

        self.first_pixel_on_line = false;

        // Secondary axis step: only when error accumulator is non-negative (!sign)
        if !sign {
            if sud {
                if sul {
                    *bltcpt = bltcpt.wrapping_sub(bltcmod as i32 as u32);
                    self.first_pixel_on_line = true;
                } else {
                    *bltcpt = bltcpt.wrapping_add(bltcmod as i32 as u32);
                    self.first_pixel_on_line = true;
                }
            } else {
                if sul {
                    if ash == 0 {
                        ash = field::MASK_SHIFT_VALUE;
                        *bltcpt = bltcpt.wrapping_sub(2);
                    } else {
                        ash -= 1;
                    }
                } else {
                    if ash == field::MASK_SHIFT_VALUE {
                        ash = 0;
                        *bltcpt = bltcpt.wrapping_add(2);
                    } else {
                        ash += 1;
                    }
                }
            }
        }

        // Primary axis step: always performed on every pixel
        if sud {
            if aul {
                if ash == 0 {
                    ash = field::MASK_SHIFT_VALUE;
                    *bltcpt = bltcpt.wrapping_sub(2);
                } else {
                    ash -= 1;
                }
            } else {
                if ash == field::MASK_SHIFT_VALUE {
                    ash = 0;
                    *bltcpt = bltcpt.wrapping_add(2);
                } else {
                    ash += 1;
                }
            }
        } else {
            if aul {
                *bltcpt = bltcpt.wrapping_sub(bltcmod as i32 as u32);
                self.first_pixel_on_line = true;
            } else {
                *bltcpt = bltcpt.wrapping_add(bltcmod as i32 as u32);
                self.first_pixel_on_line = true;
            }
        }

        *bltcpt &= MASK_OCS_DMA_ADDRESS;

        // Error accumulator step
        if use_a {
            if sign {
                *bltapt = bltapt.wrapping_add(bltbmod as i32 as u32);
            } else {
                *bltapt = bltapt.wrapping_add(bltamod as i32 as u32);
            }
        }

        // Update sign bit 6 in BLTCON1
        let new_sign = (*bltapt as i16) < 0;
        if new_sign {
            *bltcon1 |= field::BIT_LINE_SIGN;
        } else {
            *bltcon1 &= !field::BIT_LINE_SIGN;
        }

        // Write updated ash back into BLTCON0
        *bltcon0 = (*bltcon0 & field::MASK_CONTROL_PRESERVE) | (ash << field::BIT_SHIFT);
    }
}
