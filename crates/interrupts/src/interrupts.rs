//! Amiga 500 Central Interrupt Controller (INTENA / INTREQ / IPL 1..6)
//!
//! Models the 14-source interrupt priority encoder, atomic SET/CLR bit 15 logic,
//! master interrupt enable gating (bit 14 INTEN), and 6 prioritized CPU interrupt levels (IPL 1..6).

use config::mask::intreq;
use serde::{Deserialize, Serialize};

const LEVEL1: u8 = 1;
const LEVEL2: u8 = 2;
const LEVEL3: u8 = 3;
const LEVEL4: u8 = 4;
const LEVEL5: u8 = 5;
const LEVEL6: u8 = 6;

const MASK_IRQ_WRITE: u16 = 0x7FFF;

/// Returns the interrupt request bitmask for the specified audio channel (0..3)
#[inline(always)]
pub const fn irq_aud(channel: usize) -> u16 {
    (intreq::BIT_AUD0 as u32).wrapping_shl(channel as u32) as u16
}

/// Mask of all 14 physical interrupt sources (bits 0..13)
const MASK_ALL_IRQS: u16 = 0x3FFF;

/// Level 6 mask: EXTER (bit 13)
const MASK_LEVEL6: u16 = intreq::BIT_EXTER;
/// Level 5 mask: DSKSYN (bit 12) | RBF (bit 11)
const MASK_LEVEL5: u16 = intreq::BIT_DSKSYN | intreq::BIT_RBF;
/// Level 4 mask: AUD3..0 (bits 10..7)
const MASK_LEVEL4: u16 = intreq::BIT_AUD3 | intreq::BIT_AUD2 | intreq::BIT_AUD1 | intreq::BIT_AUD0;
/// Level 3 mask: BLIT (bit 6) | VERTB (bit 5) | COPER (bit 4)
const MASK_LEVEL3: u16 = intreq::BIT_BLIT | intreq::BIT_VERTB | intreq::BIT_COPER;
/// Level 2 mask: PORTS (bit 3)
const MASK_LEVEL2: u16 = intreq::BIT_PORTS;
/// Level 1 mask: SOFT (bit 2) | DSKBLK (bit 1) | TBE (bit 0)
const MASK_LEVEL1: u16 = intreq::BIT_SOFT | intreq::BIT_DSKBLK | intreq::BIT_TBE;

/// Amiga 500 Central Interrupt Priority Controller
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InterruptController {
    /// Interrupt Enable register (INTENA write at $DFF09A, INTENAR read at $DFF01C)
    pub intena: u16,
    /// Interrupt Request register (INTREQ write at $DFF09C, INTREQR read at $DFF01E)
    pub intreq: u16,
}

impl InterruptController {
    /// Creates a new interrupt controller initialized to power-on reset defaults (all interrupts disabled, zero requests).
    #[inline]
    pub fn new() -> Self {
        Self {
            intena: 0,
            intreq: 0,
        }
    }

    /// Resets all interrupt enable and request registers to zero.
    #[inline]
    pub fn reset(&mut self) {
        self.intena = 0;
        self.intreq = 0;
    }

    /// Writes the INTENA register following SET/CLR bit 15 semantics:
    /// - If bit 15 is 1, each 1 bit in val (bits 0..14) sets/enables the corresponding interrupt.
    /// - If bit 15 is 0, each 1 bit in val (bits 0..14) clears/disables the corresponding interrupt.
    /// - Bits written with 0 remain unmodified.
    #[inline]
    pub fn write_intena(&mut self, val: u16) {
        if (val & intreq::BIT_SET_CLR) != 0 {
            self.intena |= val & MASK_IRQ_WRITE;
        } else {
            self.intena &= !(val & MASK_IRQ_WRITE);
        }
    }

    /// Writes the INTREQ register following SET/CLR bit 15 semantics:
    /// - If bit 15 is 1, each 1 bit in val (bits 0..14) sets/asserts the corresponding interrupt request.
    /// - If bit 15 is 0, each 1 bit in val (bits 0..14) clears the corresponding interrupt request.
    /// - Bits written with 0 remain unmodified.
    #[inline]
    pub fn write_intreq(&mut self, val: u16) {
        if (val & intreq::BIT_SET_CLR) != 0 {
            self.intreq |= val & MASK_IRQ_WRITE;
        } else {
            self.intreq &= !(val & MASK_IRQ_WRITE);
        }
    }

    /// Asserts interrupt request bits immediately without requiring the bit 15 SET flag.
    /// Used by custom chips and hardware events to assert request lines directly.
    #[inline]
    pub fn request(&mut self, mask: u16) {
        self.intreq |= mask & MASK_IRQ_WRITE;
    }

    /// Returns true if the master interrupt enable bit (INTEN, bit 14) is active.
    #[inline]
    pub fn is_master_enabled(&self) -> bool {
        (self.intena & intreq::BIT_INTEN) != 0
    }

    /// Evaluates active, enabled interrupt requests and returns the 14-bit mask of pending interrupts.
    /// Returns 0 if the master interrupt enable bit (INTEN, bit 14) is cleared.
    #[inline]
    pub fn pending_mask(&self) -> u16 {
        if !self.is_master_enabled() {
            0
        } else {
            self.intreq & self.intena & MASK_ALL_IRQS
        }
    }

    /// Priority encoder: evaluates all active, enabled interrupt requests and returns the highest active level (0..6).
    /// If master enable (bit 14) is disabled or no enabled requests are pending, returns 0.
    #[inline]
    pub fn pending_level(&self) -> u8 {
        let pending = self.pending_mask();
        if pending == 0 {
            return 0;
        }

        // Level 6: External / CIA-B (bit 13)
        if (pending & MASK_LEVEL6) != 0 {
            return LEVEL6;
        }
        // Level 5: Disk Sync (bit 12) or Serial Receive (bit 11)
        if (pending & MASK_LEVEL5) != 0 {
            return LEVEL5;
        }
        // Level 4: Audio channels 0..3 (bits 10..7)
        if (pending & MASK_LEVEL4) != 0 {
            return LEVEL4;
        }
        // Level 3: Blitter (bit 6), Vertical Blank (bit 5), Copper (bit 4)
        if (pending & MASK_LEVEL3) != 0 {
            return LEVEL3;
        }
        // Level 2: External / CIA-A (bit 3)
        if (pending & MASK_LEVEL2) != 0 {
            return LEVEL2;
        }
        // Level 1: Software (bit 2), Disk Block (bit 1), Serial Transmit (bit 0)
        if (pending & MASK_LEVEL1) != 0 {
            return LEVEL1;
        }

        0
    }

    /// Reads the INTENAR register ($DFF01C).
    #[inline]
    pub fn read_intenar(&self) -> u16 {
        self.intena
    }

    /// Reads the INTREQR register ($DFF01E).
    #[inline]
    pub fn read_intreqr(&self) -> u16 {
        self.intreq
    }
}
