//! Queued Blitter register writes through the owning Agnus.
//!
//! Data writes change active state only at the owner capture boundary.
//! Attempted queue overflow is fatal before admission; address strobes remain separate.

use super::Agnus;
use config::custom_reg::agnus as reg;
use config::{MutationMode, PROVISIONAL_CUSTOM_WRITE_DELAY_CCK};

impl Agnus {
    #[inline]
    pub fn set_bltcon0(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTCON0,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltcon1(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTCON1,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltafwm(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTAFWM,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltalwm(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTALWM,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltcpth(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTCPTH,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltcptl(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTCPTL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltbpth(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTBPTH,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltbptl(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTBPTL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltapth(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTAPTH,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltaptl(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTAPTL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltdpth(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTDPTH,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltdptl(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTDPTL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltsize(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTSIZE,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltcmod(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTCMOD,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltbmod(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTBMOD,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltamod(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTAMOD,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltdmod(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTDMOD,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltcdat(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTCDAT,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltbdat(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTBDAT,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_bltadat(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_BLTADAT,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }
}
