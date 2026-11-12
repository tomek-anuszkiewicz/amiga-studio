//! Agnus Copper: granted two-word fetches and beam-synchronized instructions.

use crate::register_bits::copcon;
use config::chip_ram::MASK_OCS_DMA_ADDRESS;
use config::custom_reg;
use serde::{Deserialize, Serialize};

#[path = "copper_control.rs"]
mod registers;

/// Instruction progress survives both arbitration denial and DMA suspension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CopperState {
    #[default]
    Idle,
    FetchIR1,
    FetchIR2,
    Waiting,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Copper {
    pub cop1lc: u32,
    pub cop2lc: u32,
    pub cop_pc: u32,
    pub ir1: u16,
    pub ir2: u16,
    pub copcon: u16,
    pub dma_enabled: bool,
    pub state: CopperState,
    /// SKIP still fetches the next instruction, but suppresses its operation.
    pub discard_next: bool,
}

impl Copper {
    const BIT_INSTR_TYPE: u16 = 0x0001;
    const BIT_WAIT_SKIP: u16 = 0x0001;
    const BIT_BFD: u16 = 0x8000;
    const MASK_VPOS_PROGRAMMABLE: u16 = 0x7F;
    const BIT_VPOS_FORCE_BIT7: u16 = 0x80;
    const MASK_HPOS_COMPARE: u16 = 0x00FE;
    const MASK_MOVE_REG: u16 = 0x01FE;
    const CDANG_REGISTER_LIMIT: u16 = custom_reg::agnus::REGISTER_COP1LCH;
    const OCS_MIN_REGISTER_LIMIT: u16 = custom_reg::agnus::REGISTER_BLTCON0;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// DMACON gates bus use; it does not discard a partially fetched instruction.
    #[inline]
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        self.dma_enabled = enabled;
    }

    #[inline]
    pub fn is_active_fetching(&self) -> bool {
        self.dma_enabled && matches!(self.state, CopperState::FetchIR1 | CopperState::FetchIR2)
    }

    #[inline]
    pub fn is_danger_enabled(&self) -> bool {
        self.copcon & copcon::BIT_CDANG != 0
    }

    /// HRM 2: masked lexicographic comparison, with vertical bit 7 always enabled.
    fn comparison_satisfied(&self, hpos: u16, vpos: u16, blitter_busy: bool) -> bool {
        if self.ir2 & Self::BIT_BFD == 0 && blitter_busy {
            return false;
        }
        let vertical_mask =
            ((self.ir2 >> 8) & Self::MASK_VPOS_PROGRAMMABLE) | Self::BIT_VPOS_FORCE_BIT7;
        let horizontal_mask = self.ir2 & Self::MASK_HPOS_COMPARE;
        let beam = (vpos & vertical_mask, hpos & horizontal_mask);
        let target = ((self.ir1 >> 8) & vertical_mask, self.ir1 & horizontal_mask);
        beam >= target
    }

    /// Advances local comparison/restart state without reading memory.
    pub fn step_cck(&mut self, hpos: u16, vpos: u16, blitter_busy: bool) {
        if hpos == 0 && vpos == 0 {
            self.restart_list1();
        }
        if self.dma_enabled
            && self.state == CopperState::Waiting
            && self.comparison_satisfied(hpos, vpos, blitter_busy)
        {
            self.state = CopperState::FetchIR1;
        }
    }

    /// Accepts exactly one word read by Agnus in a granted Copper memory cycle.
    /// A returned MOVE enters Agnus's retained common-register output.
    pub fn receive_dma_word(
        &mut self,
        word: u16,
        hpos: u16,
        vpos: u16,
        blitter_busy: bool,
    ) -> Option<(u16, u16)> {
        if !self.is_active_fetching() {
            return None;
        }
        self.cop_pc = self.cop_pc.wrapping_add(2) & MASK_OCS_DMA_ADDRESS;
        if self.state == CopperState::FetchIR1 {
            self.ir1 = word;
            self.state = CopperState::FetchIR2;
            return None;
        }
        self.ir2 = word;
        self.state = CopperState::FetchIR1;
        if self.discard_next {
            self.discard_next = false;
            return None;
        }
        if self.ir1 & Self::BIT_INSTR_TYPE == 0 {
            let register = self.ir1 & Self::MASK_MOVE_REG;
            let minimum = if self.is_danger_enabled() {
                Self::OCS_MIN_REGISTER_LIMIT
            } else {
                Self::CDANG_REGISTER_LIMIT
            };
            if register < minimum {
                self.state = CopperState::Idle;
                None
            } else {
                Some((register, word))
            }
        } else if self.ir2 & Self::BIT_WAIT_SKIP != 0 {
            self.discard_next = self.comparison_satisfied(hpos, vpos, blitter_busy);
            None
        } else {
            self.state = CopperState::Waiting;
            None
        }
    }
}
