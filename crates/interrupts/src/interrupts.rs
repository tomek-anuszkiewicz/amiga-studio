//! Amiga 500 Central Interrupt Controller (INTENA / INTREQ / IPL 1..6)
//!
//! Models the 14-source interrupt priority encoder, atomic SET/CLR bit 15 logic,
//! master interrupt enable gating (bit 14 INTEN), and 6 prioritized CPU interrupt levels (IPL 1..6).

use serde::{Deserialize, Serialize};

// --- Interrupt Source Bit Definitions ---

/// Level 1: Serial transmit buffer empty (TBE)
pub const IRQ_TBE: u16 = 1 << 0;
/// Level 1: Floppy disk block DMA finished (DSKBLK)
pub const IRQ_DSKBLK: u16 = 1 << 1;
/// Level 1: Software generated interrupt (SOFT)
pub const IRQ_SOFT: u16 = 1 << 2;

/// Level 2: I/O Ports and timers (CIA-A, PORTS)
pub const IRQ_PORTS: u16 = 1 << 3;

/// Level 3: Copper coprocessor instruction interrupt (COPER)
pub const IRQ_COPER: u16 = 1 << 4;
/// Level 3: Vertical blanking interval (VERTB)
pub const IRQ_VERTB: u16 = 1 << 5;
/// Level 3: Blitter operation finished (BLIT)
pub const IRQ_BLIT: u16 = 1 << 6;

/// Level 4: Audio channel 0 (AUD0)
pub const IRQ_AUD0: u16 = 1 << 7;
/// Level 4: Audio channel 1 (AUD1)
pub const IRQ_AUD1: u16 = 1 << 8;
/// Level 4: Audio channel 2 (AUD2)
pub const IRQ_AUD2: u16 = 1 << 9;
/// Level 4: Audio channel 3 (AUD3)
pub const IRQ_AUD3: u16 = 1 << 10;

/// Level 5: Serial receive buffer full (RBF)
pub const IRQ_RBF: u16 = 1 << 11;
/// Level 5: Floppy disk sync pattern matched (DSKSYN)
pub const IRQ_DSKSYN: u16 = 1 << 12;

/// Level 6: External interrupt line (CIA-B, EXTER)
pub const IRQ_EXTER: u16 = 1 << 13;

/// Master interrupt enable bit in INTENA (bit 14, INTEN)
pub const INTENA_INTEN: u16 = 1 << 14;

/// Atomic SET/CLR control flag (bit 15) in INTENA / INTREQ
pub const INT_SET_CLR: u16 = 1 << 15;

/// Mask of all 14 physical interrupt sources (bits 0..13)
const ALL_IRQS_MASK: u16 = 0x3FFF;

/// Level 6 mask: EXTER (bit 13)
const LEVEL6_MASK: u16 = IRQ_EXTER;
/// Level 5 mask: DSKSYN (bit 12) | RBF (bit 11)
const LEVEL5_MASK: u16 = IRQ_DSKSYN | IRQ_RBF;
/// Level 4 mask: AUD3..0 (bits 10..7)
const LEVEL4_MASK: u16 = IRQ_AUD3 | IRQ_AUD2 | IRQ_AUD1 | IRQ_AUD0;
/// Level 3 mask: BLIT (bit 6) | VERTB (bit 5) | COPER (bit 4)
const LEVEL3_MASK: u16 = IRQ_BLIT | IRQ_VERTB | IRQ_COPER;
/// Level 2 mask: PORTS (bit 3)
const LEVEL2_MASK: u16 = IRQ_PORTS;
/// Level 1 mask: SOFT (bit 2) | DSKBLK (bit 1) | TBE (bit 0)
const LEVEL1_MASK: u16 = IRQ_SOFT | IRQ_DSKBLK | IRQ_TBE;

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
        if (val & INT_SET_CLR) != 0 {
            self.intena |= val & 0x7FFF;
        } else {
            self.intena &= !(val & 0x7FFF);
        }
    }

    /// Writes the INTREQ register following SET/CLR bit 15 semantics:
    /// - If bit 15 is 1, each 1 bit in val (bits 0..14) sets/asserts the corresponding interrupt request.
    /// - If bit 15 is 0, each 1 bit in val (bits 0..14) clears the corresponding interrupt request.
    /// - Bits written with 0 remain unmodified.
    #[inline]
    pub fn write_intreq(&mut self, val: u16) {
        if (val & INT_SET_CLR) != 0 {
            self.intreq |= val & 0x7FFF;
        } else {
            self.intreq &= !(val & 0x7FFF);
        }
    }

    /// Asserts interrupt request bits immediately without requiring the bit 15 SET flag.
    /// Used by custom chips and hardware events to assert request lines directly.
    #[inline]
    pub fn request(&mut self, mask: u16) {
        self.intreq |= mask & 0x7FFF;
    }

    /// Returns true if the master interrupt enable bit (INTEN, bit 14) is active.
    #[inline]
    pub fn is_master_enabled(&self) -> bool {
        (self.intena & INTENA_INTEN) != 0
    }

    /// Evaluates active, enabled interrupt requests and returns the 14-bit mask of pending interrupts.
    /// Returns 0 if the master interrupt enable bit (INTEN, bit 14) is cleared.
    #[inline]
    pub fn pending_mask(&self) -> u16 {
        if !self.is_master_enabled() {
            0
        } else {
            self.intreq & self.intena & ALL_IRQS_MASK
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
        if (pending & LEVEL6_MASK) != 0 {
            return 6;
        }
        // Level 5: Disk Sync (bit 12) or Serial Receive (bit 11)
        if (pending & LEVEL5_MASK) != 0 {
            return 5;
        }
        // Level 4: Audio channels 0..3 (bits 10..7)
        if (pending & LEVEL4_MASK) != 0 {
            return 4;
        }
        // Level 3: Blitter (bit 6), Vertical Blank (bit 5), Copper (bit 4)
        if (pending & LEVEL3_MASK) != 0 {
            return 3;
        }
        // Level 2: External / CIA-A (bit 3)
        if (pending & LEVEL2_MASK) != 0 {
            return 2;
        }
        // Level 1: Software (bit 2), Disk Block (bit 1), Serial Transmit (bit 0)
        if (pending & LEVEL1_MASK) != 0 {
            return 1;
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
