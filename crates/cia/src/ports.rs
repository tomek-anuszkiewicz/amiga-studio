//! Timer overrides and delayed PC pulses on the 8520 parallel port.

use crate::registers::BIT_CONTROL_PORT_OUTPUT;
use crate::Cia;

const BIT_TIMER_A_PIN: u8 = 1 << 6;
const BIT_TIMER_B_PIN: u8 = 1 << 7;
const PC_ACCESS_DELAY: u8 = 3;

impl Cia {
    pub(super) fn timer_port_outputs(&self, mut pins: u8) -> u8 {
        for (control, high, bit) in [
            (self.cra, self.timer_a_output, BIT_TIMER_A_PIN),
            (self.crb, self.timer_b_output, BIT_TIMER_B_PIN),
        ] {
            if control & BIT_CONTROL_PORT_OUTPUT != 0 {
                pins = (pins & !bit) | if high { bit } else { 0 };
            }
        }
        pins
    }

    /// Driven levels, with input pins released and timer outputs overriding DDRB.
    pub fn port_b_output_pins(&self) -> u8 {
        self.timer_port_outputs(self.prb | !self.ddrb)
    }

    pub(super) fn schedule_port_handshake(&mut self) {
        self.pc_pipeline |= 1 << (PC_ACCESS_DELAY - 1);
    }

    pub(super) fn step_port_handshake(&mut self) {
        self.pins.pc_high = self.pc_pipeline & 1 == 0;
        self.pc_pipeline >>= 1;
    }
}
