//! Paula (MOS 8364) Architecture & Subsystem Coordinator
//!
//! 4-channel DMA audio, floppy disk MFM controller, serial UART transceiver,
//! and central interrupt multiplexer (INTENA, INTREQ -> IPL 1..6).

use serde::{Deserialize, Serialize};

/// Paula custom chip coordinator
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Paula {
    /// Interrupt Enable register (INTENA / INTENAR at $DFF09A / $DFF01C)
    pub intena: u16,
    /// Interrupt Request register (INTREQ / INTREQR at $DFF09C / $DFF01E)
    pub intreq: u16,
}

impl Paula {
    /// Creates a new Paula instance
    pub fn new() -> Self {
        Self {
            intena: 0,
            intreq: 0,
        }
    }

    /// Resets Paula interrupt registers to power-on defaults
    pub fn reset(&mut self) {
        self.intena = 0;
        self.intreq = 0;
    }

    /// Advances Paula registers by 1 Color Clock
    #[inline]
    pub fn step_cck(&mut self) {
        // Interrupt / register clocking
    }

    /// Writes INTENA register following SET/CLR bit 15 logic
    pub fn write_intena(&mut self, val: u16) {
        if (val & 0x8000) != 0 {
            self.intena |= val & 0x7FFF;
        } else {
            self.intena &= !(val & 0x7FFF);
        }
    }

    /// Writes INTREQ register following SET/CLR bit 15 logic
    pub fn write_intreq(&mut self, val: u16) {
        if (val & 0x8000) != 0 {
            self.intreq |= val & 0x7FFF;
        } else {
            self.intreq &= !(val & 0x7FFF);
        }
    }

    /// Asserts interrupt request bits
    #[inline]
    pub fn set_interrupt_request(&mut self, mask: u16) {
        self.intreq |= mask & 0x7FFF;
    }

    /// Clears interrupt request bits
    #[inline]
    pub fn clear_interrupt_request(&mut self, mask: u16) {
        self.intreq &= !(mask & 0x7FFF);
    }

    /// Evaluates pending, enabled interrupt sources and returns the highest active IPL (0..6)
    pub fn pending_interrupt_level(&self) -> u8 {
        // Master interrupt enable bit (INTEN, bit 14)
        if (self.intena & 0x4000) == 0 {
            return 0;
        }

        let pending = self.intreq & self.intena & 0x3FFF;
        if pending == 0 {
            return 0;
        }

        // Level 6: External / CIA-B (bit 13)
        if (pending & 0x2000) != 0 {
            return 6;
        }
        // Level 5: Disk Sync (bit 12) or Serial Receive (bit 11)
        if (pending & 0x1800) != 0 {
            return 5;
        }
        // Level 4: Audio channels 0..3 (bits 10..7)
        if (pending & 0x0780) != 0 {
            return 4;
        }
        // Level 3: Copper (bit 4), VBlank (bit 5), or Blitter (bit 6)
        if (pending & 0x0070) != 0 {
            return 3;
        }
        // Level 2: Ports / CIA-A (bit 3)
        if (pending & 0x0008) != 0 {
            return 2;
        }
        // Level 1: Serial Transmit (bit 0), Disk Block (bit 1), Software (bit 2)
        if (pending & 0x0007) != 0 {
            return 1;
        }

        0
    }
}
