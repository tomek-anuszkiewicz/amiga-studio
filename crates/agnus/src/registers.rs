//! Agnus register reads and queued control and DMA pointer writes.

use super::{Agnus, AgnusModel};
use crate::register_bits::vposr;
use config::custom_reg::agnus as reg;
use config::mask::dmacon;
use config::{
    stage_mutation, MutationMode, DMACON_WRITE_DELAY_CCK, PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
};

const DISPLAY_CONTROL_WRITE_DELAY_CCK: u8 = 4;

impl Agnus {
    /// Capacity preflight for an atomic transfer with multiple receivers.
    #[inline]
    pub fn can_accept_register_write(&self) -> bool {
        self.mutations.iter().any(Option::is_none)
    }

    /// Queues DMACON; active state changes only when due.
    #[inline]
    pub fn set_dmacon(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_DMACON,
            val,
            DMACON_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BPLCON0; active state changes only when due.
    #[inline]
    pub fn set_bplcon0(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_BPLCON0,
            val,
            DISPLAY_CONTROL_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DIWSTRT; active state changes only when due.
    #[inline]
    pub fn set_diwstrt(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_DIWSTRT,
            val,
            DISPLAY_CONTROL_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DIWSTOP; active state changes only when due.
    #[inline]
    pub fn set_diwstop(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_DIWSTOP,
            val,
            DISPLAY_CONTROL_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the sprpos family; active state changes only when due.
    #[inline]
    pub fn set_sprpos(&mut self, index: usize, val: u16) -> bool {
        if index >= config::SPRITE_COUNT {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_SPR0POS + (index as u16) * config::SPRITE_REGISTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the sprctl family; active state changes only when due.
    #[inline]
    pub fn set_sprctl(&mut self, index: usize, val: u16) -> bool {
        if index >= config::SPRITE_COUNT {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_SPR0CTL + (index as u16) * config::SPRITE_REGISTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DSKPTH; active state changes only when due.
    #[inline]
    pub fn set_dskpth(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_DSKPTH,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DSKPTL; active state changes only when due.
    #[inline]
    pub fn set_dskptl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_DSKPTL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues REFPTR; active state changes only when due.
    #[inline]
    pub fn set_refptr(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_REFPTR,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues VPOSW; active state changes only when due.
    #[inline]
    pub fn set_vposw(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_VPOSW,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues VHPOSW; active state changes only when due.
    #[inline]
    pub fn set_vhposw(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_VHPOSW,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DDFSTRT; active state changes only when due.
    #[inline]
    pub fn set_ddfstrt(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_DDFSTRT,
            val,
            DISPLAY_CONTROL_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues DDFSTOP; active state changes only when due.
    #[inline]
    pub fn set_ddfstop(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_DDFSTOP,
            val,
            DISPLAY_CONTROL_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the audlch family; active state changes only when due.
    #[inline]
    pub fn set_audlch(&mut self, index: usize, val: u16) -> bool {
        if index >= config::AUDIO_CHANNEL_COUNT {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_AUD0LCH + (index as u16) * config::AUDIO_REGISTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the audlcl family; active state changes only when due.
    #[inline]
    pub fn set_audlcl(&mut self, index: usize, val: u16) -> bool {
        if index >= config::AUDIO_CHANNEL_COUNT {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_AUD0LCL + (index as u16) * config::AUDIO_REGISTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the bplpth family; active state changes only when due.
    #[inline]
    pub fn set_bplpth(&mut self, index: usize, val: u16) -> bool {
        if index >= config::OCS_BITPLANE_COUNT {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_BPL1PTH + (index as u16) * config::BITPLANE_POINTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the bplptl family; active state changes only when due.
    #[inline]
    pub fn set_bplptl(&mut self, index: usize, val: u16) -> bool {
        if index >= config::OCS_BITPLANE_COUNT {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_BPL1PTL + (index as u16) * config::BITPLANE_POINTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BPL1MOD; active state changes only when due.
    #[inline]
    pub fn set_bpl1mod(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_BPL1MOD,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BPL2MOD; active state changes only when due.
    #[inline]
    pub fn set_bpl2mod(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_BPL2MOD,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the sprpth family; active state changes only when due.
    #[inline]
    pub fn set_sprpth(&mut self, index: usize, val: u16) -> bool {
        if index >= config::SPRITE_COUNT {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_SPR0PTH + (index as u16) * config::SPRITE_POINTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues the sprptl family; active state changes only when due.
    #[inline]
    pub fn set_sprptl(&mut self, index: usize, val: u16) -> bool {
        if index >= config::SPRITE_COUNT {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::REGISTER_SPR0PTL + (index as u16) * config::SPRITE_POINTER_STRIDE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Returns the pipelined (H, V) beam position as seen by CPU register reads (5 CCKs ahead of display beam).
    #[inline]
    fn pipelined_beam_readout(&self) -> (u16, u16) {
        let max_lines = self.field_lines();
        let line_ccks = self.line_ccks();

        let mut h = self.hpos + Self::VHPOSR_PIPELINE_LEAD_CCKS;
        let mut v = self.vpos;
        if h >= line_ccks {
            h -= line_ccks;
            v = v.wrapping_add(1);
            if v >= max_lines {
                v = 0;
            }
        }

        let effective_v = if h <= Self::VHPOSR_VERTICAL_SETTLE_CCKS {
            self.vpos
        } else {
            v
        };
        (h as u16, effective_v)
    }

    /// Reads Vertical & Horizontal beam position (VHPOSR at $DFF006)
    ///
    /// The returned beam position reflects internal Agnus pipeline and bus latency (~5 CCKs ahead of display beam).
    #[inline]
    pub fn vhposr(&self) -> u16 {
        let (h, effective_v) = self.pipelined_beam_readout();
        let v_low = (effective_v & 0xFF) as u16;
        (v_low << 8) | (h & 0xFF)
    }

    /// Reads Vertical beam position high bit, chip ID, and LOF (VPOSR at $DFF004)
    #[inline]
    pub fn vposr(&self) -> u16 {
        let mut val = 0u16;
        if self.lof {
            val |= vposr::BIT_LOF;
        }
        match self.model {
            AgnusModel::OcsNtsc8370 => val |= vposr::CHIP_ID_NTSC_8370,
            AgnusModel::OcsPal8371 => {}
        }

        let (_h, effective_v) = self.pipelined_beam_readout();
        val |= (effective_v >> 8) & vposr::MASK_VPOS_HI;
        val
    }

    /// Reads DMA Control and Blitter status (DMACONR at $DFF002)
    #[inline]
    pub fn read_dmaconr(&self) -> u16 {
        let mut val = self.dma.dmacon & dmacon::MASK_READ_CONTROL;
        if self.blitter.is_busy {
            val |= dmacon::BIT_BBUSY;
        }
        if self.blitter.is_zero {
            val |= dmacon::BIT_BZERO;
        }
        val
    }

    /// Reads an Agnus register by offset ($000..$1FE).
    /// Write-only registers return floating open bus $FFFF.
    #[inline]
    pub fn read_register(&self, offset: u16) -> u16 {
        match offset & config::MASK_CUSTOM_REG_OFFSET {
            reg::REGISTER_DMACONR => self.read_dmaconr(),
            reg::REGISTER_VPOSR => self.vposr(),
            reg::REGISTER_VHPOSR => self.vhposr(),
            _ => config::OPEN_BUS_WORD,
        }
    }

    /// Reads DMA Control and Blitter status (DMACONR at $DFF002)
    #[inline(always)]
    pub fn dmaconr(&self) -> u16 {
        self.read_dmaconr()
    }

    /// Reads DMA Control and Blitter status without side-effects for debugging
    #[inline(always)]
    pub fn dmaconr_debug(&self) -> u16 {
        self.read_dmaconr()
    }

    /// Reads Vertical and Horizontal beam position register without side-effects for debugging
    #[inline(always)]
    pub fn vhposr_debug(&self) -> u16 {
        self.vhposr()
    }

    /// Reads Vertical position register and chip ID without side-effects for debugging
    #[inline(always)]
    pub fn vposr_debug(&self) -> u16 {
        self.vposr()
    }
}
