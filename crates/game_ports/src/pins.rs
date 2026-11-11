//! Motherboard-facing signal state for GamePorts.

use crate::{GamePorts, CIAA_PRA_FIRE_MASK};
use config::mask::game_port::DIRECTION_INPUTS;
use serde::{Deserialize, Serialize};

/// Incoming values retained until the next input sampling phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GamePortsInputs {
    pub fire_output_mask: u8,
    pub fire_output_high: u8,
    pub pot_output_mask: u16,
    pub pot_output_high: u16,
}

/// Delivered input levels and driven outputs, separate from device internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GamePortsPins {
    pub incoming: Option<GamePortsInputs>,
    /// Delivered CIA fire-pin output levels/direction and Paula POT drive state.
    pub fire_output_mask: u8,
    pub fire_output_high: u8,
    pub pot_output_mask: u16,
    pub pot_output_high: u16,
    /// Connector direction/quadrature outputs, low nibble = pins 1..4.
    /// Device waveform generation is reconstructed in 2.4, outside the router.
    pub direction_pins: [u8; 2],
}

impl Default for GamePortsPins {
    fn default() -> Self {
        Self {
            incoming: None,
            fire_output_mask: 0,
            fire_output_high: CIAA_PRA_FIRE_MASK,
            pot_output_mask: 0,
            pot_output_high: 0,
            direction_pins: [DIRECTION_INPUTS; 2],
        }
    }
}

impl GamePorts {
    /// Samples retained inputs before engine work; source changes cannot alter them.
    pub fn sample_input_pins(&mut self) {
        let Some(input) = self.pins.incoming.take() else {
            return;
        };
        self.pins.fire_output_mask = input.fire_output_mask;
        self.pins.fire_output_high = input.fire_output_high;
        self.pins.pot_output_mask = input.pot_output_mask;
        self.pins.pot_output_high = input.pot_output_high;
    }
}
