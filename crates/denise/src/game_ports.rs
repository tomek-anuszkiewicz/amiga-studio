//! Four mouse counters, pin-derived low bits and the JOYTEST write latch.

use crate::Denise;

const MASK_PHASE: u8 = 3;
const BIT_QUADRATURE: u8 = 1;
const BIT_X_INPUT: u8 = 1;
const BIT_Y_INPUT: u8 = 0;
const BIT_X_QUADRATURE: u8 = 3;
const BIT_Y_QUADRATURE: u8 = 2;
const PHASE_FORWARD: u8 = 1;
const PHASE_BACKWARD: u8 = 3;
const MASK_JOYTEST_WRITE: u16 = 0xfcfc;

fn count_axis(counter: u8, primary: bool, quadrature: bool) -> u8 {
    let phase = (u8::from(quadrature) << BIT_QUADRATURE) | u8::from(primary ^ quadrature);
    match phase.wrapping_sub(counter & MASK_PHASE) & MASK_PHASE {
        PHASE_FORWARD => counter.wrapping_add(1),
        PHASE_BACKWARD => counter.wrapping_sub(1),
        // Simultaneous contact changes still expose both pins, without an inferred direction.
        _ => (counter & !MASK_PHASE) | phase,
    }
}

impl Denise {
    pub(super) fn step_game_ports(&mut self) {
        for (counter, pins) in [
            (&mut self.joy0dat, self.pins.game_port_inputs[0]),
            (&mut self.joy1dat, self.pins.game_port_inputs[1]),
        ] {
            let [y, x] = counter.to_be_bytes();
            let x = count_axis(
                x,
                pins & (1 << BIT_X_INPUT) == 0,
                pins & (1 << BIT_X_QUADRATURE) == 0,
            );
            let y = count_axis(
                y,
                pins & (1 << BIT_Y_INPUT) == 0,
                pins & (1 << BIT_Y_QUADRATURE) == 0,
            );
            *counter = u16::from_be_bytes([y, x]);
        }
    }

    pub(super) fn apply_joytest(&mut self, value: u16) {
        for counter in [&mut self.joy0dat, &mut self.joy1dat] {
            *counter = (*counter & !MASK_JOYTEST_WRITE) | (value & MASK_JOYTEST_WRITE);
        }
    }
}
