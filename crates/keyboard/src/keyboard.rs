//! Amiga Keyboard Subsystem (MOS 6500/1 Microcontroller)
//!
//! Models keyboard matrix scanning, 10-key type-ahead buffer,
//! bidirectional serial communications to CIA-A SP/CNT, and Ctrl-Amiga-Amiga reset.

use serde::{Deserialize, Serialize};

/// Amiga scancodes for qualifier and control keys
pub const SCANCODE_CTRL: u8 = 0x63;
pub const SCANCODE_L_AMIGA: u8 = 0x66;
pub const SCANCODE_R_AMIGA: u8 = 0x67;

/// Amiga keyboard controller and scancode encoder
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Keyboard {
    /// Active scancode holding register
    pub current_scancode: Option<u8>,
    /// Control key active latch
    pub ctrl_pressed: bool,
    /// Left Amiga key active latch
    pub l_amiga_pressed: bool,
    /// Right Amiga key active latch
    pub r_amiga_pressed: bool,
    /// True if the physical _RESET line is pulled low (Ctrl-Amiga-Amiga asserted)
    pub reset_line_asserted: bool,
}

impl Keyboard {
    /// Creates a new keyboard instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets keyboard state
    pub fn reset(&mut self) {
        self.current_scancode = None;
        self.ctrl_pressed = false;
        self.l_amiga_pressed = false;
        self.r_amiga_pressed = false;
        self.reset_line_asserted = false;
    }

    /// Dispatches a key press event with raw Amiga scancode
    pub fn key_down(&mut self, raw_code: u8) {
        match raw_code {
            SCANCODE_CTRL => self.ctrl_pressed = true,
            SCANCODE_L_AMIGA => self.l_amiga_pressed = true,
            SCANCODE_R_AMIGA => self.r_amiga_pressed = true,
            _ => {}
        }
        if self.ctrl_pressed && self.l_amiga_pressed && self.r_amiga_pressed {
            self.reset_line_asserted = true;
        }

        // Amiga scancode transmission format: (raw_code << 1) | 0
        let encoded = (raw_code << 1) & 0xFE;
        self.current_scancode = Some(encoded);
    }

    /// Dispatches a key release event with raw Amiga scancode
    pub fn key_up(&mut self, raw_code: u8) {
        match raw_code {
            SCANCODE_CTRL => self.ctrl_pressed = false,
            SCANCODE_L_AMIGA => self.l_amiga_pressed = false,
            SCANCODE_R_AMIGA => self.r_amiga_pressed = false,
            _ => {}
        }

        // Key release bit is set: (raw_code << 1) | 1
        let encoded = ((raw_code << 1) & 0xFE) | 0x01;
        self.current_scancode = Some(encoded);
    }

    /// Acknowledges reception of the current scancode
    #[inline]
    pub fn acknowledge(&mut self) {
        self.current_scancode = None;
    }

    /// Returns true and clears the reset line if Ctrl-Amiga-Amiga was triggered
    #[inline]
    pub fn poll_reset(&mut self) -> bool {
        let was_asserted = self.reset_line_asserted;
        self.reset_line_asserted = false;
        was_asserted
    }
}
