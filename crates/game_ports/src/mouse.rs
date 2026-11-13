//! Amiga Quadrature Mouse Subsystem
//!
//! Models the optical/mechanical 2-button and 3-button Amiga mouse,
//! Quadrature phases and queued host movement; Denise owns position counters.

use serde::{Deserialize, Serialize};

/// Amiga Mouse state and quadrature tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Mouse {
    phase_x: u8,
    phase_y: u8,
    pending_x: i32,
    pending_y: i32,
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
        *self = Self::default();
    }

    /// Applies relative motion deltas from host mouse movement
    #[inline]
    pub fn move_rel(&mut self, dx: i32, dy: i32) {
        self.pending_x = self.pending_x.saturating_add(dx);
        self.pending_y = self.pending_y.saturating_add(dy);
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

    /// Advances at most one quadrature transition per axis and returns pins 1..4.
    #[inline]
    pub fn step_cck(&mut self) -> u8 {
        const MASK_PHASE: u8 = 3;
        const BIT_Y_PIN: u8 = 0;
        const BIT_X_PIN: u8 = 1;
        const BIT_Y_QUADRATURE_PIN: u8 = 2;
        const BIT_X_QUADRATURE_PIN: u8 = 3;
        const BIT_PHASE_HIGH: u8 = 1;
        for (phase, pending) in [
            (&mut self.phase_x, &mut self.pending_x),
            (&mut self.phase_y, &mut self.pending_y),
        ] {
            let direction = pending.signum();
            *pending -= direction;
            *phase = phase.wrapping_add(direction as u8) & MASK_PHASE;
        }
        // Binary phase becomes Gray-coded, active-low connector levels.
        let x_high = (self.phase_x >> BIT_PHASE_HIGH) & 1;
        let y_high = (self.phase_y >> BIT_PHASE_HIGH) & 1;
        let x_low = (self.phase_x & 1) ^ x_high;
        let y_low = (self.phase_y & 1) ^ y_high;
        ((!y_low & 1) << BIT_Y_PIN)
            | ((!x_low & 1) << BIT_X_PIN)
            | ((!y_high & 1) << BIT_Y_QUADRATURE_PIN)
            | ((!x_high & 1) << BIT_X_QUADRATURE_PIN)
    }
}
