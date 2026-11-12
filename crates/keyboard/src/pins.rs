//! Motherboard-facing signal state for Keyboard.

use crate::Keyboard;
use serde::{Deserialize, Serialize};

/// Delivered input levels and driven outputs, separate from device internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyboardPins {
    pub incoming_host_data_high: Option<bool>,
    /// Open-drain KCLK and KDAT outputs; true releases the line high.
    pub clock_high: bool,
    pub data_high: bool,
    /// Resolved open-drain KDAT sampled from the motherboard.
    pub host_data_high: bool,
    /// True if the physical _RESET line is pulled low (Ctrl-Amiga-Amiga asserted)
    pub reset_line_asserted: bool,
}

impl Default for KeyboardPins {
    fn default() -> Self {
        Self {
            incoming_host_data_high: None,
            clock_high: true,
            data_high: true,
            host_data_high: true,
            reset_line_asserted: false,
        }
    }
}

impl Keyboard {
    /// Samples the resolved host return before controller work.
    pub fn sample_input_pins(&mut self) {
        if let Some(high) = self.pins.incoming_host_data_high.take() {
            self.pins.host_data_high = high;
        }
    }
}
