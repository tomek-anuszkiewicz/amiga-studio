//! Motherboard-facing signal state for FloppyController.

use crate::FloppyController;
use serde::{Deserialize, Serialize};

/// Incoming values retained until the next input sampling phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloppyInputs {
    pub control: u8,
    pub write_data_high: bool,
    pub write_enable_high: bool,
}

/// Delivered input levels and driven outputs, separate from device internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloppyPins {
    pub incoming: Option<FloppyInputs>,
    /// Delivered CIA-B control levels, released high.
    pub control_input: u8,
    /// Raw connector disk data and active-low write gate, separate from legacy media.
    pub read_data_high: bool,
    pub write_data_high: bool,
    pub write_enable_high: bool,
    /// Released/high INDEX output; deterministic drive endpoint, no media needed.
    pub index_high: bool,
}

impl Default for FloppyPins {
    fn default() -> Self {
        Self {
            incoming: None,
            control_input: u8::MAX,
            read_data_high: true,
            write_data_high: true,
            write_enable_high: true,
            index_high: true,
        }
    }
}

impl FloppyController {
    /// Samples retained inputs before engine work; source changes cannot alter them.
    pub fn sample_input_pins(&mut self) {
        let Some(input) = self.pins.incoming.take() else {
            return;
        };
        self.receive_control_pins(input.control);
        self.pins.write_data_high = input.write_data_high;
        self.pins.write_enable_high = input.write_enable_high;
    }
}
