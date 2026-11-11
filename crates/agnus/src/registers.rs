//! Queued Agnus register setters and private due-event application.

use super::Agnus;
use config::custom_reg::agnus as reg;
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

    /// Queues COPCON; active state changes only when due.
    #[inline]
    pub fn set_copcon(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COPCON,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTCON0; active state changes only when due.
    #[inline]
    pub fn set_bltcon0(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTCON0,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTCON1; active state changes only when due.
    #[inline]
    pub fn set_bltcon1(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTCON1,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTAFWM; active state changes only when due.
    #[inline]
    pub fn set_bltafwm(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTAFWM,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTALWM; active state changes only when due.
    #[inline]
    pub fn set_bltalwm(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTALWM,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTCPTH; active state changes only when due.
    #[inline]
    pub fn set_bltcpth(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTCPTH,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTCPTL; active state changes only when due.
    #[inline]
    pub fn set_bltcptl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTCPTL,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTBPTH; active state changes only when due.
    #[inline]
    pub fn set_bltbpth(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTBPTH,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTBPTL; active state changes only when due.
    #[inline]
    pub fn set_bltbptl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTBPTL,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTAPTH; active state changes only when due.
    #[inline]
    pub fn set_bltapth(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTAPTH,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTAPTL; active state changes only when due.
    #[inline]
    pub fn set_bltaptl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTAPTL,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTDPTH; active state changes only when due.
    #[inline]
    pub fn set_bltdpth(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTDPTH,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTDPTL; active state changes only when due.
    #[inline]
    pub fn set_bltdptl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTDPTL,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTSIZE; active state changes only when due.
    #[inline]
    pub fn set_bltsize(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTSIZE,
            val,
            1,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTCMOD; active state changes only when due.
    #[inline]
    pub fn set_bltcmod(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTCMOD,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTBMOD; active state changes only when due.
    #[inline]
    pub fn set_bltbmod(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTBMOD,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTAMOD; active state changes only when due.
    #[inline]
    pub fn set_bltamod(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTAMOD,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTDMOD; active state changes only when due.
    #[inline]
    pub fn set_bltdmod(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTDMOD,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues BLTCDAT; active state changes only when due.
    #[inline]
    pub fn set_bltcdat(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTCDAT,
            val,
            1,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTBDAT; active state changes only when due.
    #[inline]
    pub fn set_bltbdat(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTBDAT,
            val,
            1,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTADAT; active state changes only when due.
    #[inline]
    pub fn set_bltadat(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTADAT,
            val,
            1,
            MutationMode::Pipeline,
        )
    }

    /// Queues COP1LCH; active state changes only when due.
    #[inline]
    pub fn set_cop1lch(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COP1LCH,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues COP1LCL; active state changes only when due.
    #[inline]
    pub fn set_cop1lcl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COP1LCL,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues COP2LCH; active state changes only when due.
    #[inline]
    pub fn set_cop2lch(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COP2LCH,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues COP2LCL; active state changes only when due.
    #[inline]
    pub fn set_cop2lcl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COP2LCL,
            val,
            2,
            MutationMode::OverwritePending,
        )
    }

    /// Queues COPJMP1; active state changes only when due.
    #[inline]
    pub fn set_copjmp1(&mut self) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COPJMP1,
            0,
            1,
            MutationMode::OverwritePending,
        )
    }

    /// Queues COPJMP2; active state changes only when due.
    #[inline]
    pub fn set_copjmp2(&mut self) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COPJMP2,
            0,
            1,
            MutationMode::OverwritePending,
        )
    }

    /// Queues COPINS; active state changes only when due.
    #[inline]
    pub fn set_copins(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COPINS,
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
}
