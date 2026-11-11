//! Queued Blitter register writes through the owning Agnus.

use super::Agnus;
use config::custom_reg::agnus as reg;
use config::{stage_mutation, MutationMode};

impl Agnus {
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
}
