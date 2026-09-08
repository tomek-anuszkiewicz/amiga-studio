//! CMPM (Compare Memory) Instruction Handlers
//!
//! Compares memory operands via postincrement: `CMPM (Ay)+, (Ax)+`.
//! Evaluates ((Ax) - (Ay)) and updates N, Z, V, and C flags.
//! Neither memory location is modified. Extend (X) flag is unaffected.

use crate::micro::ea;
use crate::core::Cpu;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Micro-Step Callbacks
// ============================================================================

/// ALU compare callback for Byte
pub fn alu_cmpm_b(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.scratch[1] & 0xFF) as u8;
    let d = (state.micro.last_read & 0xFF) as u8;
    crate::instructions::cmp::cmp_b(state, s, d);
}

/// ALU compare callback for Word
pub fn alu_cmpm_w(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.scratch[1] & 0xFFFF) as u16;
    let d = state.micro.last_read;
    crate::instructions::cmp::cmp_w(state, s, d);
}

/// ALU compare callback for Long
pub fn alu_cmpm_l(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.scratch[2];
    let d = state.micro.scratch[1];
    crate::instructions::cmp::cmp_l(state, s, d);
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_CMPM_B: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: Some(ea::latch_src_b_and_calc_dst_pi_b), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_cmpm_b), base_clocks: 4 },
];

pub static STEPS_CMPM_W: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: Some(ea::ea_calc_src_pi_w), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: Some(ea::latch_src_w_and_calc_dst_pi_w), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_cmpm_w), base_clocks: 4 },
];

pub static STEPS_CMPM_L: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: Some(ea::ea_calc_src_pi_l), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: Some(ea::latch_src_l_and_calc_dst_pi_l), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_cmpm_l), base_clocks: 4 },
];

/// Decodes the micro-step sequence for CMPM based on size (0 = Byte, 1 = Word, 2 = Long)
pub const fn decode_cmpm_steps(size: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => Some(&STEPS_CMPM_B),
        1 => Some(&STEPS_CMPM_W),
        2 => Some(&STEPS_CMPM_L),
        _ => None,
    }
}


