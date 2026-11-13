//! Four mouse counters, pin-derived low bits and the JOYTEST write latch.

use crate::Denise;
use config::mask::game_port::MASK_DIRECTION_INPUTS;

const MASK_PHASE: u8 = 3;
const MASK_JOY_PHASES: u16 = 0x0303;
const PHASE_FORWARD: u8 = 1;
const PHASE_BACKWARD: u8 = 3;
const MASK_JOYTEST_WRITE: u16 = 0xfcfc;

// Indexed by active-low connector pins 1..4. Each axis's binary phase is
// (quadrature << 1) | (primary ^ quadrature), packed as JOY's Y/X bytes.
const JOY_PHASES: [u16; 16] = [
    0x0202, 0x0302, 0x0203, 0x0303, 0x0102, 0x0002, 0x0103, 0x0003, 0x0201, 0x0301, 0x0200, 0x0300,
    0x0101, 0x0001, 0x0100, 0x0000,
];

fn count_axis(counter: u8, phase: u8) -> u8 {
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
            let phases = JOY_PHASES[usize::from(pins & MASK_DIRECTION_INPUTS)];
            // The counter already retains the last consumed phase. JOYTEST
            // preserves it, and restore needs no separate change flag.
            if *counter & MASK_JOY_PHASES == phases {
                continue;
            }
            let [y, x] = counter.to_be_bytes();
            let [phase_y, phase_x] = phases.to_be_bytes();
            let x = count_axis(x, phase_x);
            let y = count_axis(y, phase_y);
            *counter = u16::from_be_bytes([y, x]);
        }
    }

    pub(super) fn apply_joytest(&mut self, value: u16) {
        for counter in [&mut self.joy0dat, &mut self.joy1dat] {
            *counter = (*counter & !MASK_JOYTEST_WRITE) | (value & MASK_JOYTEST_WRITE);
        }
    }
}
