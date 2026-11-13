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
    /// Captures current levels; the local engine advances quadrature counters.
    pub fn receive_game_port_pins(&mut self, pins: [u8; 2]) {
        self.pins.game_port_inputs = pins.map(|value| value & MASK_DIRECTION_INPUTS);
    }
}

/// Local receiver cursor and field state derived from received RGA strobes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeniseSync {
    pub hpos: u16,
    pub vpos: u16,
    pub vertical_blank: bool,
    pub equalization: bool,
    pub long_line: bool,
    pub synchronized: bool,
    pub field_locked: bool,
    pub field_odd: bool,
    pub field_lines: u16,
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
            field_locked: false,
            field_odd: false,
            field_lines: frame_builder::PAL_MAX_RASTER_LINES as u16,
        }
    }
}

impl DeniseSync {
    fn receive_strobe(&mut self, address: u16, interlaced: bool) -> bool {
        if address == sync::REGISTER_STRLONG {
            self.long_line = true;
            return false;
        }
        if !matches!(
            address,
            sync::REGISTER_STREQU | sync::REGISTER_STRVBL | sync::REGISTER_STRHOR
        ) {
            return false;
        }
        let equalization = address == sync::REGISTER_STREQU;
        let field_start = equalization && !self.equalization;
        let completed = field_start && self.field_locked;
        if completed {
            self.field_lines = self.vpos.wrapping_add(1);
            self.field_odd = interlaced && !self.field_odd;
        }
        if field_start || !self.synchronized {
            self.vpos = 0;
        } else {
            self.vpos = self.vpos.wrapping_add(1) & MASK_VERTICAL_CURSOR;
        }
        if field_start {
            self.field_locked = true;
        }
        self.hpos = 0;
        self.equalization = equalization;
        self.vertical_blank = address != sync::REGISTER_STRHOR;
        self.long_line = false;
        self.synchronized = true;
        completed
    }
}

impl Denise {
    pub(super) fn receive_sync_strobe(&mut self, address: u16) {
        let interlaced = self.bplcon0 & config::mask::bplcon0::BIT_LACE != 0;
        let completed = self.sync.receive_strobe(address, interlaced);
        if address == sync::REGISTER_STRLONG {
            return;
        }
        self.frame_builder.begin_line(
            interlaced,
            self.sync.field_odd,
            completed,
            self.sync.vertical_blank,
        );
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
