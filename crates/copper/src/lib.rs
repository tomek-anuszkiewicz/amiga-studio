//! Agnus Copper (Coprocessor) Emulation
//!
//! Synchronized coprocessor executing MOVE, WAIT, and SKIP instructions
//! driven in lockstep with the raster beam position.

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
        self.is_running = false;
        self.is_waiting = false;
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

    /// Writes COPCON control register
    #[inline]
    pub fn set_copcon(&mut self, val: u16) {
        self.cdang = (val & 0x0002) != 0;
    }

    /// Advances Copper coprocessor state by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        // Scaffold placeholder: execution advances during scheduled DMA slots
    }
}
