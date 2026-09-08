//! MOVE Long (32-bit) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions.

use crate::micro::common;
use crate::micro::ea;
use crate::core::Cpu;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Atomic Micro-Step Constants
// ============================================================================

const READ_HI: MicroStep = MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 };
const READ_LO: MicroStep = MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 };
const WRITE_HI: MicroStep = MicroStep { step_fn: Cpu::step_bus_write_long_high, alu_fn: None, base_clocks: 4 };
const WRITE_LO: MicroStep = MicroStep { step_fn: Cpu::step_bus_write_long_low, alu_fn: None, base_clocks: 4 };
const WRITE_OP: MicroStep = MicroStep { step_fn: Cpu::step_bus_write_word, alu_fn: None, base_clocks: 4 };
const WRITE_HI_RETIRE: MicroStep = MicroStep { step_fn: Cpu::step_bus_write_long_high_and_retire, alu_fn: None, base_clocks: 4 };
const PREFETCH_SCRATCH: MicroStep = MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: None, base_clocks: 4 };
const FETCH_EXT: MicroStep = MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 };
const PREFETCH_RETIRE: MicroStep = common::RETIRE_STANDARD;

// ============================================================================
// Pure ALU Callbacks: MOVE.L
// ============================================================================

#[inline(always)]
pub fn alu_move_l_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.d_long(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.set_d_long(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_move_l_an_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.read_a(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.set_d_long(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_move_l_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro.scratch[1];
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.set_d_long(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_move_l_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro.scratch[1];
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.set_d_long(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_move_l_src_dn(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let val = state.d_long(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.write_buffer = val;
}

#[inline(always)]
pub fn alu_move_l_src_an(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let val = state.read_a(reg_src as usize);
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.write_buffer = val;
}

#[inline(always)]
pub fn alu_move_l_src_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let val = state.micro.scratch[1];
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.write_buffer = val;
}

#[inline(always)]
pub fn alu_move_l_src_imm(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let val = state.micro.scratch[1];
    state.set_ccr_nz_clear_vc((val as i32) < 0, val == 0);
    state.micro.write_buffer = val;
}

// ============================================================================
// Static Step Slices: MOVE.L
// ============================================================================

pub static STEPS_MOVE_L_DN_DN: [MicroStep; 1] = [
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_move_l_dn_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_L_DN_AI: [MicroStep; 5] = [
    MicroStep::alu(alu_move_l_src_dn), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_DN_PI: [MicroStep; 5] = [
    MicroStep::alu(alu_move_l_src_dn), MicroStep::alu(ea::ea_calc_move_dst_pi_l),
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_DN_PD: [MicroStep; 6] = [
    MicroStep::alu(alu_move_l_src_dn), PREFETCH_SCRATCH,
    MicroStep::alu(ea::ea_calc_dst_pd_l_lo), WRITE_OP,
    MicroStep::alu(ea::ea_calc_dst_pd_l_hi), WRITE_HI_RETIRE,
];
pub static STEPS_MOVE_L_DN_D16: [MicroStep; 5] = [
    MicroStep::alu(alu_move_l_src_dn), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_DN_IDX: [MicroStep; 6] = [
    MicroStep::alu(alu_move_l_src_dn), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_DN_ABSW: [MicroStep; 5] = [
    MicroStep::alu(alu_move_l_src_dn), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_DN_ABSL: [MicroStep; 6] = [
    MicroStep::alu(alu_move_l_src_dn), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_AN_DN: [MicroStep; 1] = [
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_move_l_an_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_L_AN_AI: [MicroStep; 5] = [
    MicroStep::alu(alu_move_l_src_an), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_AN_PI: [MicroStep; 5] = [
    MicroStep::alu(alu_move_l_src_an), MicroStep::alu(ea::ea_calc_move_dst_pi_l),
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_AN_PD: [MicroStep; 6] = [
    MicroStep::alu(alu_move_l_src_an), PREFETCH_SCRATCH,
    MicroStep::alu(ea::ea_calc_dst_pd_l_lo), WRITE_OP,
    MicroStep::alu(ea::ea_calc_dst_pd_l_hi), WRITE_HI_RETIRE,
];
pub static STEPS_MOVE_L_AN_D16: [MicroStep; 5] = [
    MicroStep::alu(alu_move_l_src_an), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_AN_IDX: [MicroStep; 6] = [
    MicroStep::alu(alu_move_l_src_an), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_AN_ABSW: [MicroStep; 5] = [
    MicroStep::alu(alu_move_l_src_an), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_AN_ABSL: [MicroStep; 6] = [
    MicroStep::alu(alu_move_l_src_an), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_AI_DN: [MicroStep; 4] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_HI,
    READ_LO, MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_move_l_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_L_AI_AI: [MicroStep; 8] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep::alu(ea::ea_calc_dst_ai), WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_AI_PI: [MicroStep; 8] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep::alu(ea::ea_calc_move_dst_pi_l), WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_AI_PD: [MicroStep; 9] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    PREFETCH_SCRATCH, MicroStep::alu(ea::ea_calc_dst_pd_l_lo),
    WRITE_OP, MicroStep::alu(ea::ea_calc_dst_pd_l_hi),
    WRITE_HI_RETIRE,
];
pub static STEPS_MOVE_L_AI_D16: [MicroStep; 8] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_AI_IDX: [MicroStep; 9] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT,
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_AI_ABSW: [MicroStep; 8] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_AI_ABSL: [MicroStep; 9] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PI_DN: [MicroStep; 4] = [
    MicroStep::alu(ea::ea_calc_src_pi_l), READ_HI,
    READ_LO, MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_move_l_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_L_PI_AI: [MicroStep; 8] = [
    MicroStep::alu(ea::ea_calc_src_pi_l), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep::alu(ea::ea_calc_dst_ai), WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PI_PI: [MicroStep; 8] = [
    MicroStep::alu(ea::ea_calc_src_pi_l), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep::alu(ea::ea_calc_move_dst_pi_l), WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PI_PD: [MicroStep; 9] = [
    MicroStep::alu(ea::ea_calc_src_pi_l), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    PREFETCH_SCRATCH, MicroStep::alu(ea::ea_calc_dst_pd_l_lo),
    WRITE_OP, MicroStep::alu(ea::ea_calc_dst_pd_l_hi),
    WRITE_HI_RETIRE,
];
pub static STEPS_MOVE_L_PI_D16: [MicroStep; 8] = [
    MicroStep::alu(ea::ea_calc_src_pi_l), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PI_IDX: [MicroStep; 9] = [
    MicroStep::alu(ea::ea_calc_src_pi_l), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT,
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PI_ABSW: [MicroStep; 8] = [
    MicroStep::alu(ea::ea_calc_src_pi_l), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PI_ABSL: [MicroStep; 9] = [
    MicroStep::alu(ea::ea_calc_src_pi_l), READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PD_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2 }, READ_HI,
    READ_LO, MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_move_l_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_L_PD_AI: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep::alu(ea::ea_calc_dst_ai), WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PD_PI: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep::alu(ea::ea_calc_move_dst_pi_l), WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PD_PD: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    PREFETCH_SCRATCH, MicroStep::alu(ea::ea_calc_dst_pd_l_lo),
    WRITE_OP, MicroStep::alu(ea::ea_calc_dst_pd_l_hi),
    WRITE_HI_RETIRE,
];
pub static STEPS_MOVE_L_PD_D16: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PD_IDX: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT,
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PD_ABSW: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PD_ABSL: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_D16_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_move_l_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_L_D16_AI: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep::alu(ea::ea_calc_dst_ai), WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_D16_PI: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep::alu(ea::ea_calc_move_dst_pi_l), WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_D16_PD: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    PREFETCH_SCRATCH, MicroStep::alu(ea::ea_calc_dst_pd_l_lo),
    WRITE_OP, MicroStep::alu(ea::ea_calc_dst_pd_l_hi),
    WRITE_HI_RETIRE,
];
pub static STEPS_MOVE_L_D16_D16: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_D16_IDX: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT,
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_D16_ABSW: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_D16_ABSL: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_IDX_DN: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_move_l_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_L_IDX_AI: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_IDX_PI: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep::alu(ea::ea_calc_move_dst_pi_l),
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_IDX_PD: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), PREFETCH_SCRATCH,
    MicroStep::alu(ea::ea_calc_dst_pd_l_lo), WRITE_OP,
    MicroStep::alu(ea::ea_calc_dst_pd_l_hi), WRITE_HI_RETIRE,
];
pub static STEPS_MOVE_L_IDX_D16: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_IDX_IDX: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_IDX_ABSW: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_IDX_ABSL: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_ABSW_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_move_l_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_L_ABSW_AI: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep::alu(ea::ea_calc_dst_ai), WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_ABSW_PI: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep::alu(ea::ea_calc_move_dst_pi_l), WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_ABSW_PD: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    PREFETCH_SCRATCH, MicroStep::alu(ea::ea_calc_dst_pd_l_lo),
    WRITE_OP, MicroStep::alu(ea::ea_calc_dst_pd_l_hi),
    WRITE_HI_RETIRE,
];
pub static STEPS_MOVE_L_ABSW_D16: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_ABSW_IDX: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT,
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_ABSW_ABSW: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_ABSW_ABSL: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_ABSL_DN: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_HI, READ_LO,
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_move_l_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_L_ABSL_AI: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_ABSL_PI: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep::alu(ea::ea_calc_move_dst_pi_l),
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_ABSL_PD: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), PREFETCH_SCRATCH,
    MicroStep::alu(ea::ea_calc_dst_pd_l_lo), WRITE_OP,
    MicroStep::alu(ea::ea_calc_dst_pd_l_hi), WRITE_HI_RETIRE,
];
pub static STEPS_MOVE_L_ABSL_D16: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_ABSL_IDX: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_ABSL_ABSW: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_ABSL_ABSL: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PCD16_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_move_l_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_L_PCD16_AI: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep::alu(ea::ea_calc_dst_ai), WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PCD16_PI: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep::alu(ea::ea_calc_move_dst_pi_l), WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PCD16_PD: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    PREFETCH_SCRATCH, MicroStep::alu(ea::ea_calc_dst_pd_l_lo),
    WRITE_OP, MicroStep::alu(ea::ea_calc_dst_pd_l_hi),
    WRITE_HI_RETIRE,
];
pub static STEPS_MOVE_L_PCD16_D16: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PCD16_IDX: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT,
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PCD16_ABSW: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PCD16_ABSL: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_HI,
    READ_LO, MicroStep::alu(alu_move_l_src_mem),
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PCIDX_DN: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_move_l_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_L_PCIDX_AI: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PCIDX_PI: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep::alu(ea::ea_calc_move_dst_pi_l),
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PCIDX_PD: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), PREFETCH_SCRATCH,
    MicroStep::alu(ea::ea_calc_dst_pd_l_lo), WRITE_OP,
    MicroStep::alu(ea::ea_calc_dst_pd_l_hi), WRITE_HI_RETIRE,
];
pub static STEPS_MOVE_L_PCIDX_D16: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PCIDX_IDX: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PCIDX_ABSW: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_PCIDX_ABSL: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_HI, READ_LO,
    MicroStep::alu(alu_move_l_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_IMM_DN: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_move_l_imm_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_L_IMM_AI: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4 },
    MicroStep::alu(alu_move_l_src_imm), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_IMM_PI: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4 },
    MicroStep::alu(alu_move_l_src_imm), MicroStep::alu(ea::ea_calc_move_dst_pi_l),
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_IMM_PD: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4 },
    MicroStep::alu(alu_move_l_src_imm), PREFETCH_SCRATCH,
    MicroStep::alu(ea::ea_calc_dst_pd_l_lo), WRITE_OP,
    MicroStep::alu(ea::ea_calc_dst_pd_l_hi), WRITE_HI_RETIRE,
];
pub static STEPS_MOVE_L_IMM_D16: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4 },
    MicroStep::alu(alu_move_l_src_imm), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_IMM_IDX: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4 },
    MicroStep::alu(alu_move_l_src_imm), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_IMM_ABSW: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4 },
    MicroStep::alu(alu_move_l_src_imm), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_HI, WRITE_LO,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_L_IMM_ABSL: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 }, MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4 },
    MicroStep::alu(alu_move_l_src_imm), MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_HI,
    WRITE_LO, PREFETCH_RETIRE,
];

// ============================================================================
// Static 2D Lookup Table: MOVE.L
// ============================================================================

static MOVE_L_LOOKUP: [[&[MicroStep]; 8]; 12] = [
    [&STEPS_MOVE_L_DN_DN, &STEPS_MOVE_L_DN_AI, &STEPS_MOVE_L_DN_PI, &STEPS_MOVE_L_DN_PD, &STEPS_MOVE_L_DN_D16, &STEPS_MOVE_L_DN_IDX, &STEPS_MOVE_L_DN_ABSW, &STEPS_MOVE_L_DN_ABSL],
    [&STEPS_MOVE_L_AN_DN, &STEPS_MOVE_L_AN_AI, &STEPS_MOVE_L_AN_PI, &STEPS_MOVE_L_AN_PD, &STEPS_MOVE_L_AN_D16, &STEPS_MOVE_L_AN_IDX, &STEPS_MOVE_L_AN_ABSW, &STEPS_MOVE_L_AN_ABSL],
    [&STEPS_MOVE_L_AI_DN, &STEPS_MOVE_L_AI_AI, &STEPS_MOVE_L_AI_PI, &STEPS_MOVE_L_AI_PD, &STEPS_MOVE_L_AI_D16, &STEPS_MOVE_L_AI_IDX, &STEPS_MOVE_L_AI_ABSW, &STEPS_MOVE_L_AI_ABSL],
    [&STEPS_MOVE_L_PI_DN, &STEPS_MOVE_L_PI_AI, &STEPS_MOVE_L_PI_PI, &STEPS_MOVE_L_PI_PD, &STEPS_MOVE_L_PI_D16, &STEPS_MOVE_L_PI_IDX, &STEPS_MOVE_L_PI_ABSW, &STEPS_MOVE_L_PI_ABSL],
    [&STEPS_MOVE_L_PD_DN, &STEPS_MOVE_L_PD_AI, &STEPS_MOVE_L_PD_PI, &STEPS_MOVE_L_PD_PD, &STEPS_MOVE_L_PD_D16, &STEPS_MOVE_L_PD_IDX, &STEPS_MOVE_L_PD_ABSW, &STEPS_MOVE_L_PD_ABSL],
    [&STEPS_MOVE_L_D16_DN, &STEPS_MOVE_L_D16_AI, &STEPS_MOVE_L_D16_PI, &STEPS_MOVE_L_D16_PD, &STEPS_MOVE_L_D16_D16, &STEPS_MOVE_L_D16_IDX, &STEPS_MOVE_L_D16_ABSW, &STEPS_MOVE_L_D16_ABSL],
    [&STEPS_MOVE_L_IDX_DN, &STEPS_MOVE_L_IDX_AI, &STEPS_MOVE_L_IDX_PI, &STEPS_MOVE_L_IDX_PD, &STEPS_MOVE_L_IDX_D16, &STEPS_MOVE_L_IDX_IDX, &STEPS_MOVE_L_IDX_ABSW, &STEPS_MOVE_L_IDX_ABSL],
    [&STEPS_MOVE_L_ABSW_DN, &STEPS_MOVE_L_ABSW_AI, &STEPS_MOVE_L_ABSW_PI, &STEPS_MOVE_L_ABSW_PD, &STEPS_MOVE_L_ABSW_D16, &STEPS_MOVE_L_ABSW_IDX, &STEPS_MOVE_L_ABSW_ABSW, &STEPS_MOVE_L_ABSW_ABSL],
    [&STEPS_MOVE_L_ABSL_DN, &STEPS_MOVE_L_ABSL_AI, &STEPS_MOVE_L_ABSL_PI, &STEPS_MOVE_L_ABSL_PD, &STEPS_MOVE_L_ABSL_D16, &STEPS_MOVE_L_ABSL_IDX, &STEPS_MOVE_L_ABSL_ABSW, &STEPS_MOVE_L_ABSL_ABSL],
    [&STEPS_MOVE_L_PCD16_DN, &STEPS_MOVE_L_PCD16_AI, &STEPS_MOVE_L_PCD16_PI, &STEPS_MOVE_L_PCD16_PD, &STEPS_MOVE_L_PCD16_D16, &STEPS_MOVE_L_PCD16_IDX, &STEPS_MOVE_L_PCD16_ABSW, &STEPS_MOVE_L_PCD16_ABSL],
    [&STEPS_MOVE_L_PCIDX_DN, &STEPS_MOVE_L_PCIDX_AI, &STEPS_MOVE_L_PCIDX_PI, &STEPS_MOVE_L_PCIDX_PD, &STEPS_MOVE_L_PCIDX_D16, &STEPS_MOVE_L_PCIDX_IDX, &STEPS_MOVE_L_PCIDX_ABSW, &STEPS_MOVE_L_PCIDX_ABSL],
    [&STEPS_MOVE_L_IMM_DN, &STEPS_MOVE_L_IMM_AI, &STEPS_MOVE_L_IMM_PI, &STEPS_MOVE_L_IMM_PD, &STEPS_MOVE_L_IMM_D16, &STEPS_MOVE_L_IMM_IDX, &STEPS_MOVE_L_IMM_ABSW, &STEPS_MOVE_L_IMM_ABSL],
];

// ============================================================================
// Opcode Decoder: MOVE.L
// ============================================================================

pub const fn decode_move_l_steps(src_mode: u8, src_reg: u8, dst_mode: u8, dst_reg: u8) -> Option<&'static [MicroStep]> {
    let src_idx = match src_mode {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 5,
        6 => 6,
        7 => match src_reg { 0 => 7, 1 => 8, 2 => 9, 3 => 10, 4 => 11, _ => return None },
        _ => return None,
    };
    let dst_idx = match dst_mode {
        0 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        5 => 4,
        6 => 5,
        7 => match dst_reg { 0 => 6, 1 => 7, _ => return None },
        _ => return None,
    };
    Some(MOVE_L_LOOKUP[src_idx][dst_idx])
}
