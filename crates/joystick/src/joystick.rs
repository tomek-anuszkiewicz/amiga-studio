//! Amiga Digital Joystick Subsystem
//!
//! Models standard Atari 9-pin digital joysticks, directional switch decoding,
//! and Denise JOYxDAT register XOR bit encoding.

use serde::{Deserialize, Serialize};

/// Digital Joystick state and directional switch matrix
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Joystick {
    /// Up direction switch
    pub up: bool,
    /// Down direction switch
    pub down: bool,
    /// Left direction switch
    pub left: bool,
    /// Right direction switch
    pub right: bool,
    /// Primary fire button (connected to CIA-A PRA)
    pub fire1: bool,
    /// Secondary fire button (connected to POTGO)
    pub fire2: bool,
}

impl Joystick {
    /// Creates a new joystick with all directions and fire buttons released
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets all switches to released state
    pub fn reset(&mut self) {
        self.up = false;
        self.down = false;
        self.left = false;
        self.right = false;
        self.fire1 = false;
        self.fire2 = false;
    }

    /// Updates directional switches
    #[inline]
    pub fn set_directions(&mut self, up: bool, down: bool, left: bool, right: bool) {
        self.up = up;
        self.down = down;
        self.left = left;
        self.right = right;
    }

    /// Updates primary fire button
    #[inline]
    pub fn set_fire1(&mut self, pressed: bool) {
        self.fire1 = pressed;
    }

    /// Updates secondary fire button
    #[inline]
    pub fn set_fire2(&mut self, pressed: bool) {
        self.fire2 = pressed;
    }

    /// Encodes directional switches into Denise JOYxDAT register format:
    /// - Bit 9: LEFT
    /// - Bit 8: UP ^ LEFT
    /// - Bit 1: RIGHT
    /// - Bit 0: DOWN ^ RIGHT
    #[inline]
    pub fn joy_dat(&self) -> u16 {
        let mut val = 0u16;
        if self.left {
            val |= 1 << 9;
        }
        if self.up ^ self.left {
            val |= 1 << 8;
        }
        if self.right {
            val |= 1 << 1;
        }
        if self.down ^ self.right {
            val |= 1 << 0;
        }
        val
    }
}
