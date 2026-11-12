//! HRM 2 field geometry and HRM 7 programmable beam counters.

use crate::register_bits::vposr;
use crate::{Agnus, AgnusModel};
use config::mask::bplcon0;

const MASK_VERTICAL_LOW: u16 = 0x00ff;
const BIT_VERTICAL_HIGH: u16 = 0x0100;
const MASK_HORIZONTAL: u16 = 0x00ff;
const MASK_OCS_VPOSW_HIGH: u16 = 0x0001;

impl Agnus {
    pub(super) fn line_ccks(&self) -> u16 {
        match self.model {
            AgnusModel::OcsPal8371 => Self::PAL_LINE_CCKS,
            AgnusModel::OcsNtsc8370 if self.lol => Self::NTSC_LONG_LINE_CCKS,
            AgnusModel::OcsNtsc8370 => Self::NTSC_SHORT_LINE_CCKS,
        }
    }

    pub(super) fn field_lines(&self) -> u16 {
        let short = match self.model {
            AgnusModel::OcsPal8371 => Self::PAL_FRAME_LINES,
            AgnusModel::OcsNtsc8370 => Self::NTSC_FRAME_LINES,
        };
        short + u16::from(self.lof)
    }

    pub(super) fn advance_beam(&mut self) {
        self.hpos = self.hpos.wrapping_add(1);
        if self.hpos < self.line_ccks() {
            return;
        }
        self.hpos = 0;
        if self.model == AgnusModel::OcsNtsc8370 {
            self.lol = !self.lol;
        }
        self.vpos = self.vpos.wrapping_add(1);
        if self.vpos >= self.field_lines() {
            self.vpos = 0;
            if self.dma.bplcon0 & bplcon0::BIT_LACE != 0 {
                self.lof = !self.lof;
            }
        }
    }

    pub(super) fn apply_vposw(&mut self, word: u16) {
        self.vpos = (self.vpos & MASK_VERTICAL_LOW) | ((word & MASK_OCS_VPOSW_HIGH) << 8);
        self.lof = word & vposr::BIT_LOF != 0;
    }

    pub(super) fn apply_vhposw(&mut self, word: u16) {
        self.vpos = (self.vpos & BIT_VERTICAL_HIGH) | (word >> 8);
        self.hpos = word & MASK_HORIZONTAL;
    }
}
