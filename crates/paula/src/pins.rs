//! Motherboard-facing signal state for Paula.

use crate::Paula;
use serde::{Deserialize, Serialize};

/// Incoming values retained until the next input sampling phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaulaInputs {
    pub int2_high: bool,
    pub int3_high: bool,
    pub int6_high: bool,
    pub pot0dat: u16,
    pub pot1dat: u16,
    pub legacy_vblank_event: bool,
    pub disk_read_high: bool,
    pub rga_strobe: Option<u16>,
}

/// Delivered input levels and driven outputs, separate from device internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaulaPins {
    pub incoming: Option<PaulaInputs>,
    /// Delivered active-low interrupt inputs from CIA-A, Agnus and CIA-B.
    pub int2_high: bool,
    pub int3_high: bool,
    pub int6_high: bool,
    /// Delivered connector POT levels; conversion counters remain legacy state.
    pub pot_input_high: u16,
    /// Disk connector data input and output pins; no bitstream engine implied.
    pub disk_read_high: bool,
    pub disk_write_high: bool,
    pub disk_write_enable: bool,
    /// Active-high serialized DMAL output; the 2.4 engine supplies its bits.
    pub dmal_output_high: bool,
    /// Unconsumed STRHOR RGA event, independent of CPU register queue capture.
    pub rga_strobe: Option<u16>,
}

impl Default for PaulaPins {
    fn default() -> Self {
        Self {
            incoming: None,
            int2_high: true,
            int3_high: true,
            int6_high: true,
            pot_input_high: config::mask::game_port::MASK_POT_DATA,
            disk_read_high: true,
            disk_write_high: true,
            disk_write_enable: false,
            dmal_output_high: false,
            rga_strobe: None,
        }
    }
}

impl Paula {
    /// Samples current digital POT levels without another transport delay.
    /// Proportional conversion and digital pin behavior are reconstructed in 2.4.
    pub fn receive_pot_inputs(&mut self, pins: u16) {
        self.pins.pot_input_high = pins;
        self.potgor = pins;
    }

    /// Captures delivered electrical levels; Paula samples them during its step.
    pub fn receive_interrupt_pins(&mut self, int2_high: bool, int3_high: bool, int6_high: bool) {
        self.pins.int2_high = int2_high;
        self.pins.int3_high = int3_high;
        self.pins.int6_high = int6_high;
    }

    /// Samples after due CPU writes. Asserted levels set, release does not acknowledge.
    pub(super) fn sample_interrupt_pins(&mut self) {
        let mask = if self.pins.int2_high {
            0
        } else {
            config::mask::intreq::BIT_PORTS
        } | if self.pins.int3_high {
            0
        } else {
            config::mask::intreq::BIT_BLIT
        } | if self.pins.int6_high {
            0
        } else {
            config::mask::intreq::BIT_EXTER
        };
        self.interrupts.request(mask);
    }
}

/// Receiver-owned phase anchor; request framing and serial shifting remain 2.4.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaulaSync {
    pub cck_since_horizontal: u16,
}

impl Paula {
    /// Samples retained inputs before engine work; source changes cannot alter them.
    pub fn sample_input_pins(&mut self) {
        let Some(input) = self.pins.incoming.take() else {
            return;
        };
        self.receive_interrupt_pins(input.int2_high, input.int3_high, input.int6_high);
        self.pot0dat = input.pot0dat;
        self.pot1dat = input.pot1dat;
        if input.legacy_vblank_event {
            self.set_interrupt_request(config::mask::intreq::BIT_VERTB);
        }
        self.pins.disk_read_high = input.disk_read_high;
        self.pins.rga_strobe = input.rga_strobe;
    }
}
