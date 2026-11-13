//! Amiga Digital Joystick Subsystem
//!
//! Models standard Atari 9-pin digital joysticks, directional switch decoding,
//! and active-low connector levels for the Denise receiver.

use serde::{Deserialize, Serialize};

const BIT_PIN_UP: u8 = 1 << 0;
const BIT_PIN_DOWN: u8 = 1 << 1;
const BIT_PIN_LEFT: u8 = 1 << 2;
const BIT_PIN_RIGHT: u8 = 1 << 3;
const MASK_DIRECTION_PINS: u8 = BIT_PIN_UP | BIT_PIN_DOWN | BIT_PIN_LEFT | BIT_PIN_RIGHT;

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

    /// Each closed switch pulls its connector pin low; no JOY register lives here.
    #[inline]
    pub fn direction_pins(&self) -> u8 {
        let pressed = if self.up { BIT_PIN_UP } else { 0 }
            | if self.down { BIT_PIN_DOWN } else { 0 }
            | if self.left { BIT_PIN_LEFT } else { 0 }
            | if self.right { BIT_PIN_RIGHT } else { 0 };
        MASK_DIRECTION_PINS & !pressed
    }
}
