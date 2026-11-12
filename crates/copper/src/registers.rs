//! Copper control register and list restart strobes.

use super::{Copper, CopperState};
use config::chip_ram::MASK_OCS_DMA_ADDRESS;
use config::mask::copcon;

impl Copper {
    /// Restarts execution using Copper list 1 (COPJMP1 strobe)
    #[inline]
    pub fn restart_list1(&mut self) {
        self.cop_pc = self.cop1lc & MASK_OCS_DMA_ADDRESS;
        self.is_running = true;
        self.is_waiting = false;
        self.state = CopperState::FetchIR1(Self::FETCH_WORD_CCK);
    }

    /// Restarts execution using Copper list 2 (COPJMP2 strobe)
    #[inline]
    pub fn restart_list2(&mut self) {
        self.cop_pc = self.cop2lc & MASK_OCS_DMA_ADDRESS;
        self.is_running = true;
        self.is_waiting = false;
        self.state = CopperState::FetchIR1(Self::FETCH_WORD_CCK);
    }

    /// Writes COPCON control register
    #[inline]
    pub fn set_copcon(&mut self, val: u16) {
        self.copcon = val;
        self.cdang = (val & copcon::BIT_CDANG) != 0;
    }
}
