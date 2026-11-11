//! Queued Copper register writes and live read strobes through Agnus.

use super::Agnus;
use config::custom_reg::agnus as reg;
use config::{stage_mutation, MutationMode, PROVISIONAL_CUSTOM_WRITE_DELAY_CCK};

impl Agnus {
    /// Queues COPCON; active state changes only when due.
    #[inline]
    pub fn set_copcon(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COPCON,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
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
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues COP1LCL; active state changes only when due.
    #[inline]
    pub fn set_cop1lcl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COP1LCL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues COP2LCH; active state changes only when due.
    #[inline]
    pub fn set_cop2lch(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COP2LCH,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues COP2LCL; active state changes only when due.
    #[inline]
    pub fn set_cop2lcl(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COP2LCL,
            val,
            PROVISIONAL_CUSTOM_WRITE_DELAY_CCK,
            MutationMode::Pipeline,
        )
    }

    /// Queues COPINS; active state changes only when due.
    #[inline]
    pub fn set_copins(&mut self, val: u16) -> bool {
        stage_mutation(
            &mut self.mutations,
            reg::COPINS,
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
