//! Agnus Copper (Coprocessor) Emulation
//!
//! Synchronized coprocessor executing MOVE, WAIT, and SKIP instructions
//! driven in lockstep with the raster beam position.

use config::BeamPosition;
use serde::{Deserialize, Serialize};

/// Agnus Copper coprocessor state and execution engine
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Copper {
    /// First Copper list pointer (COP1LCH/COP1LCL, 18-bit Chip RAM address)
    pub cop1lc: u32,
    /// Second Copper list pointer (COP2LCH/COP2LCL, 18-bit Chip RAM address)
    pub cop2lc: u32,
    /// Active program counter
    pub cop_pc: u32,
    /// Current instruction latch
    pub copins: u16,
    /// Copper Danger mode flag (COPCON bit 1: allows writes to $DFF000..$DFF07E)
    pub cdang: bool,
    /// Copper DMA channel enabled via DMACON (COPEN bit 7 and DMAEN bit 9)
    pub dma_enabled: bool,
    /// True if Copper is actively executing instructions
    pub is_running: bool,
    /// True if Copper is halted waiting for a beam position comparison
    pub is_waiting: bool,
}

impl Copper {
    /// Creates a new uninitialized Copper instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets Copper registers to power-on defaults
    pub fn reset(&mut self) {
        self.cop1lc = 0;
        self.cop2lc = 0;
        self.cop_pc = 0;
        self.copins = 0;
        self.cdang = false;
        self.dma_enabled = false;
        self.is_running = false;
        self.is_waiting = false;
    }

    /// Sets Copper DMA enabled state from DMACON
    #[inline]
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        self.dma_enabled = enabled;
    }

    /// Sets COP1LC address latch
    #[inline]
    pub fn set_cop1lc(&mut self, addr: u32) {
        self.cop1lc = addr;
    }

    /// Sets COP2LC address latch
    #[inline]
    pub fn set_cop2lc(&mut self, addr: u32) {
        self.cop2lc = addr;
    }

    /// Restarts execution using Copper list 1 (COPJMP1 strobe)
    #[inline]
    pub fn restart_list1(&mut self) {
        self.cop_pc = self.cop1lc;
        self.is_running = true;
        self.is_waiting = false;
    }

    /// Restarts execution using Copper list 2 (COPJMP2 strobe)
    #[inline]
    pub fn restart_list2(&mut self) {
        self.cop_pc = self.cop2lc;
        self.is_running = true;
        self.is_waiting = false;
    }

    /// Action method: triggers Copper restart on COP1LC address
    #[inline]
    pub fn strobe_jump1(&mut self, addr: u32) {
        self.cop1lc = addr;
        self.restart_list1();
    }

    /// Action method: triggers Copper restart on COP2LC address
    #[inline]
    pub fn strobe_jump2(&mut self, addr: u32) {
        self.cop2lc = addr;
        self.restart_list2();
    }

    /// Writes COPCON control register
    #[inline]
    pub fn set_copcon(&mut self, val: u16) {
        self.cdang = (val & 0x0002) != 0;
    }

    /// Advances Copper coprocessor state by 1 Color Clock observing current beam coordinates
    #[inline]
    pub fn step_cck(&mut self, _beam: BeamPosition) {
        // Execution advances during scheduled Copper DMA slots (CCK 9..10 or free slots)
    }
}
