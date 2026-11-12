//! Motherboard-facing signal state for Denise.

use crate::Denise;
use config::custom_reg::denise as sync;
use config::mask::game_port::MASK_DIRECTION_INPUTS;
use serde::{Deserialize, Serialize};

const MASK_VERTICAL_CURSOR: u16 = 0x1ff;

/// Incoming values retained until the next input sampling phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeniseInputs {
    pub rga_strobe: Option<u16>,
}

/// Delivered input levels and driven outputs, separate from device internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenisePins {
    pub incoming: Option<DeniseInputs>,
    /// Delivered controller direction/quadrature levels (connector pins 1..4).
    pub game_port_inputs: [u8; 2],
    /// Unconsumed RGA synchronization event delivered by the motherboard.
    pub rga_strobe: Option<u16>,
}

impl Default for DenisePins {
    fn default() -> Self {
        Self {
            incoming: None,
            game_port_inputs: [MASK_DIRECTION_INPUTS; 2],
            rga_strobe: None,
        }
    }
}

impl Denise {
    /// Input capture only; quadrature counting belongs to the 2.4 receiver.
    pub fn receive_game_port_pins(&mut self, pins: [u8; 2]) {
        self.pins.game_port_inputs = pins.map(|value| value & MASK_DIRECTION_INPUTS);
    }
}

/// Local receiver cursor; full field/equalization reconstruction remains 2.4 work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeniseSync {
    pub hpos: u16,
    pub vpos: u16,
    pub vertical_blank: bool,
    pub equalization: bool,
    pub long_line: bool,
    pub synchronized: bool,
}

impl Default for DeniseSync {
    fn default() -> Self {
        Self {
            hpos: 0,
            vpos: 0,
            vertical_blank: true,
            equalization: false,
            long_line: false,
            synchronized: false,
        }
    }
}

impl DeniseSync {
    pub(super) fn receive_strobe(&mut self, address: u16) {
        match address {
            sync::REGISTER_STREQU | sync::REGISTER_STRVBL | sync::REGISTER_STRHOR => {
                let equalization = address == sync::REGISTER_STREQU;
                self.vpos = if !self.synchronized || (equalization && !self.equalization) {
                    0
                } else {
                    self.vpos.wrapping_add(1) & MASK_VERTICAL_CURSOR
                };
                self.hpos = 0;
                self.equalization = equalization;
                self.vertical_blank = address != sync::REGISTER_STRHOR;
                self.long_line = false;
                self.synchronized = true;
            }
            sync::REGISTER_STRLONG => self.long_line = true,
            _ => {}
        }
    }
}

impl Denise {
    /// Samples retained inputs before engine work; source changes cannot alter them.
    pub fn sample_input_pins(&mut self) {
        let Some(input) = self.pins.incoming.take() else {
            return;
        };
        self.pins.rga_strobe = input.rga_strobe;
    }
}
