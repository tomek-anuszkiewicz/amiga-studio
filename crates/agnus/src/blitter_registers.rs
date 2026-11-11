//! Queued Blitter register writes through the owning Agnus.

use super::Agnus;
use config::custom_reg::agnus as reg;
use config::{stage_mutation, MutationMode, PROVISIONAL_CUSTOM_WRITE_DELAY_CCK};

impl Agnus {
    /// Queues BLTCON0; active state changes only when due.
    #[inline]
    pub fn set_bltcon0(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTCON0,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTCON1; active state changes only when due.
    #[inline]
    pub fn set_bltcon1(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTCON1,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTAFWM; active state changes only when due.
    #[inline]
    pub fn set_bltafwm(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTAFWM,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTALWM; active state changes only when due.
    #[inline]
    pub fn set_bltalwm(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTALWM,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTCPTH; active state changes only when due.
    #[inline]
    pub fn set_bltcpth(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTCPTH,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTCPTL; active state changes only when due.
    #[inline]
    pub fn set_bltcptl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTCPTL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTBPTH; active state changes only when due.
    #[inline]
    pub fn set_bltbpth(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTBPTH,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTBPTL; active state changes only when due.
    #[inline]
    pub fn set_bltbptl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTBPTL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTAPTH; active state changes only when due.
    #[inline]
    pub fn set_bltapth(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTAPTH,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTAPTL; active state changes only when due.
    #[inline]
    pub fn set_bltaptl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTAPTL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTDPTH; active state changes only when due.
    #[inline]
    pub fn set_bltdpth(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTDPTH,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTDPTL; active state changes only when due.
    #[inline]
    pub fn set_bltdptl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTDPTL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTSIZE; active state changes only when due.
    #[inline]
    pub fn set_bltsize(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTSIZE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTCMOD; active state changes only when due.
    #[inline]
    pub fn set_bltcmod(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTCMOD,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTBMOD; active state changes only when due.
    #[inline]
    pub fn set_bltbmod(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTBMOD,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTAMOD; active state changes only when due.
    #[inline]
    pub fn set_bltamod(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTAMOD,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTDMOD; active state changes only when due.
    #[inline]
    pub fn set_bltdmod(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTDMOD,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues BLTCDAT; active state changes only when due.
    #[inline]
    pub fn set_bltcdat(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::BLTCDAT,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
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
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
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
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }
}
