//! Amiga Centronics Parallel Printer Port Subsystem
//!
//! Models the 8-bit bidirectional data lines (CIA-A Port B)
//! and control handshake signals (CIA-B / STROBE / BUSY / POUT).

use serde::{Deserialize, Serialize};

/// Centronics parallel printer port interface
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ParallelPort {
    /// 8-bit data lines (connected to CIA-A PRB / DDRB)
    pub data: u8,
    /// 8-bit data direction (1 = output, 0 = input)
    pub direction: u8,
    /// Strobe handshake signal (active low, from CIA-B)
    pub strobe: bool,
    /// Busy status signal (active high, to CIA-B)
    pub busy: bool,
    /// Paper Out status signal (to CIA-B)
    pub paper_out: bool,
    /// Select / Online status signal (to CIA-B)
    pub select: bool,
}

impl ParallelPort {
    /// Creates a new parallel port instance
    pub fn new() -> Self {
        Self {
            select: true, // Printer online by default
            ..Default::default()
        }
    }

    /// Resets parallel port lines
    pub fn reset(&mut self) {
        self.data = 0;
        self.direction = 0;
        self.strobe = false;
        self.busy = false;
        self.paper_out = false;
        self.select = true;
    }

    /// Writes data onto the 8-bit bus lines
    #[inline]
    pub fn write_data(&mut self, val: u8) {
        self.data = val;
    }

    /// Reads data from the 8-bit bus lines
    #[inline]
    pub fn read_data(&self) -> u8 {
        self.data
    }
}
