//! Amiga Quadrature Mouse Subsystem
//!
//! Models the optical/mechanical 2-button and 3-button Amiga mouse,
//! 2-phase quadrature pulse generation, and JOY0DAT register encoding.

use serde::{Deserialize, Serialize};

/// Amiga Mouse state and quadrature tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Mouse {
    /// 8-bit horizontal position counter (JOY0DAT low byte)
    pub x: u8,
    /// 8-bit vertical position counter (JOY0DAT high byte)
    pub y: u8,
    /// Left mouse button state (true = pressed, connected to CIA-A PRA bit 6)
    pub left_button: bool,
    /// Right mouse button state (true = pressed, connected to POTGO bit 10)
    pub right_button: bool,
    /// Middle mouse button state (true = pressed, connected to POTGO bit 8)
    pub middle_button: bool,
}

impl Mouse {
    /// Creates a new mouse initialized at coordinates (0, 0) with all buttons released
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets mouse position counters and button states
    pub fn reset(&mut self) {
        self.x = 0;
        self.y = 0;
        self.left_button = false;
        self.right_button = false;
        self.middle_button = false;
    }

    /// Applies relative motion deltas from host mouse movement
    #[inline]
    pub fn move_rel(&mut self, dx: i32, dy: i32) {
        self.x = (self.x as i32 + dx) as u8;
        self.y = (self.y as i32 + dy) as u8;
    }

    /// Sets left mouse button state
    #[inline]
    pub fn set_left_button(&mut self, pressed: bool) {
        self.left_button = pressed;
    }

    /// Sets right mouse button state
    #[inline]
    pub fn set_right_button(&mut self, pressed: bool) {
        self.right_button = pressed;
    }

    /// Sets middle mouse button state
    #[inline]
    pub fn set_middle_button(&mut self, pressed: bool) {
        self.middle_button = pressed;
    }

    /// Returns the 16-bit JOY0DAT register value (Y counter in bits 15-8, X counter in bits 7-0)
    #[inline]
    pub fn joy_dat(&self) -> u16 {
        ((self.y as u16) << 8) | (self.x as u16)
    }
}
