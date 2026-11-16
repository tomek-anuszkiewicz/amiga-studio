//! Queued Copper register writes and live read strobes through Agnus.
//!
//! Data writes change active state only at the owner capture boundary.
//! Attempted queue overflow is fatal before admission; address strobes remain separate.

use super::Agnus;
use config::custom_reg::agnus as reg;
use config::{MutationMode, PROVISIONAL_CUSTOM_WRITE_DELAY_CCK};

impl Agnus {
    #[inline]
    pub fn set_copcon(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_COPCON,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_cop1lch(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_COP1LCH,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_cop1lcl(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_COP1LCL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_cop2lch(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_COP2LCH,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_cop2lcl(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_COP2LCL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    #[inline]
    pub fn set_copins(&mut self, val: u16) -> bool {
        self.mutations.stage(
            reg::REGISTER_COPINS,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Triggers COPJMP1 strobe: restarts Copper execution at COP1LC
    #[inline]
    pub fn strobe_copjmp1(&mut self) {
        self.copper.restart_list1();
    }

    /// Triggers COPJMP2 strobe: restarts Copper execution at COP2LC
    #[inline]
    pub fn strobe_copjmp2(&mut self) {
        self.copper.restart_list2();
    }
}
