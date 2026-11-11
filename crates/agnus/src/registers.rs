//! Agnus register reads and queued control and DMA pointer writes.

use super::{Agnus, AgnusModel};
use config::custom_reg::agnus as reg;
use config::mask::dmacon;
use config::{stage_mutation, MutationMode};

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
            reg::DMACON,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BPLCON0; active state changes only when due.
    #[inline]
    pub fn set_bplcon0(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BPLCON0,
            val,
            4,
            MutationMode::OverwritePending,
        )
    }

    /// Queues DIWSTRT; active state changes only when due.
    #[inline]
    pub fn set_diwstrt(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::DIWSTRT,
            val,
            4,
            MutationMode::OverwritePending,
        )
    }

    /// Queues DIWSTOP; active state changes only when due.
    #[inline]
    pub fn set_diwstop(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::DIWSTOP,
            val,
            4,
            MutationMode::OverwritePending,
        )
    }

    /// Queues the sprpos family; active state changes only when due.
    #[inline]
    pub fn set_sprpos(&mut self, index: usize, val: u16) -> bool {
        if index >= 8 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::SPR0POS + (index as u16) * 8,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues the sprctl family; active state changes only when due.
    #[inline]
    pub fn set_sprctl(&mut self, index: usize, val: u16) -> bool {
        if index >= 8 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::SPR0CTL + (index as u16) * 8,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues DSKPTH; active state changes only when due.
    #[inline]
    pub fn set_dskpth(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::DSKPTH,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues DSKPTL; active state changes only when due.
    #[inline]
    pub fn set_dskptl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::DSKPTL,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues REFPTR; active state changes only when due.
    #[inline]
    pub fn set_refptr(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::REFPTR,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues VPOSW; active state changes only when due.
    #[inline]
    pub fn set_vposw(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::VPOSW,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues VHPOSW; active state changes only when due.
    #[inline]
    pub fn set_vhposw(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::VHPOSW,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues DDFSTRT; active state changes only when due.
    #[inline]
    pub fn set_ddfstrt(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::DDFSTRT,
            val,
            4,
            MutationMode::OverwritePending,
        )
    }

    /// Queues DDFSTOP; active state changes only when due.
    #[inline]
    pub fn set_ddfstop(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::DDFSTOP,
            val,
            4,
            MutationMode::OverwritePending,
        )
    }

    /// Queues the audlch family; active state changes only when due.
    #[inline]
    pub fn set_audlch(&mut self, index: usize, val: u16) -> bool {
        if index >= 4 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::AUD0LCH + (index as u16) * 16,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues the audlcl family; active state changes only when due.
    #[inline]
    pub fn set_audlcl(&mut self, index: usize, val: u16) -> bool {
        if index >= 4 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::AUD0LCL + (index as u16) * 16,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues the bplpth family; active state changes only when due.
    #[inline]
    pub fn set_bplpth(&mut self, index: usize, val: u16) -> bool {
        if index >= 6 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::BPL1PTH + (index as u16) * 4,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues the bplptl family; active state changes only when due.
    #[inline]
    pub fn set_bplptl(&mut self, index: usize, val: u16) -> bool {
        if index >= 6 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::BPL1PTL + (index as u16) * 4,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BPL1MOD; active state changes only when due.
    #[inline]
    pub fn set_bpl1mod(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BPL1MOD,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BPL2MOD; active state changes only when due.
    #[inline]
    pub fn set_bpl2mod(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BPL2MOD,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues the sprpth family; active state changes only when due.
    #[inline]
    pub fn set_sprpth(&mut self, index: usize, val: u16) -> bool {
        if index >= 8 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::SPR0PTH + (index as u16) * 4,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues the sprptl family; active state changes only when due.
    #[inline]
    pub fn set_sprptl(&mut self, index: usize, val: u16) -> bool {
        if index >= 8 {
            return false;
        }
        stage_mutation(
            &mut self.mutations,
            reg::SPR0PTL + (index as u16) * 4,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Returns the pipelined (H, V) beam position as seen by CPU register reads (5 CCKs ahead of display beam).
    #[inline]
    fn pipelined_beam_readout(&self) -> (u16, u16) {
        let max_lines = match self.model {
            AgnusModel::OcsNtsc8370 => Self::NTSC_FRAME_LINES,
            _ => Self::PAL_FRAME_LINES,
        };
        let line_ccks = match self.model {
            AgnusModel::OcsNtsc8370 => {
                if self.lol {
                    Self::NTSC_LONG_LINE_CCKS
                } else {
                    Self::NTSC_SHORT_LINE_CCKS
                }
            }
            _ => Self::PAL_LINE_CCKS,
        };

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
            val |= 0x8000;
        }
        match self.model {
            AgnusModel::OcsNtsc8370 => val |= 0x1000,
            AgnusModel::OcsPal8371 => {}
        }

        let (_h, effective_v) = self.pipelined_beam_readout();
        val |= (effective_v >> 8) & 0x07;
        val
    }

    /// Reads DMA Control and Blitter status (DMACONR at $DFF002)
    #[inline]
    pub fn read_dmaconr(&self) -> u16 {
        let mut val = self.dmacon & 0x07FF;
        if self.blitter.is_busy {
            val |= dmacon::BBUSY;
        }
        if self.blitter.is_zero {
            val |= dmacon::BZERO;
        }
        val
    }

    /// Reads an Agnus register by offset ($000..$1FE).
    /// Write-only registers return floating open bus $FFFF.
    #[inline]
    pub fn read_register(&self, offset: u16) -> u16 {
        match offset & 0x1FE {
            reg::DMACONR => self.read_dmaconr(),
            reg::VPOSR => self.vposr(),
            reg::VHPOSR => self.vhposr(),
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
