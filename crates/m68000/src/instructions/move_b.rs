//! MOVE Byte (8-bit) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions.

use crate::micro::common;
use crate::micro::ea;
use crate::core::Cpu;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Atomic Micro-Step Constants (2-Clock / 1 CCK)
// ============================================================================

const READ_SRC_BYTE: MicroStep = common::READ_SRC_BYTE;
const READ_BYTE_FINISH: MicroStep = common::READ_BYTE_FINISH;
const BUS_WRITE_IDLE: MicroStep = common::BUS_WRITE_IDLE;
const WRITE_DST_BYTE: MicroStep = common::WRITE_DST_BYTE;
const WRITE_DST_BYTE_RETIRE: MicroStep = common::WRITE_DST_BYTE_RETIRE;
const PREFETCH_SCRATCH_READ: MicroStep = common::PREFETCH_SCRATCH_READ;
const PREFETCH_SCRATCH_FINISH: MicroStep = common::PREFETCH_SCRATCH_FINISH;
const FETCH_EXT_READ: MicroStep = common::FETCH_EXT_READ;
const FETCH_EXT_FINISH: MicroStep = common::FETCH_EXT_FINISH;
const PREFETCH_NEXT_READ: MicroStep = common::PREFETCH_NEXT_READ;
const PREFETCH_NEXT_RETIRE: MicroStep = common::PREFETCH_NEXT_RETIRE;

// ============================================================================
// Pure ALU Callbacks: MOVE.B
// ============================================================================

#[inline(always)]
pub fn alu_move_b_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = (state.d_long(reg_src as usize) & 0xFF) as u8;
    state.set_ccr_nz_clear_vc((val as i8) < 0, val == 0);
    state.set_d_byte(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_move_b_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = (state.micro.source & 0xFF) as u8;
    state.set_ccr_nz_clear_vc((val as i8) < 0, val == 0);
    state.set_d_byte(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_move_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = (state.prefetch[0] & 0xFF) as u8;
    state.set_ccr_nz_clear_vc((val as i8) < 0, val == 0);
    state.set_d_byte(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_move_b_src_dn(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let val = (state.d_long(reg_src as usize) & 0xFF) as u8;
    state.set_ccr_nz_clear_vc((val as i8) < 0, val == 0);
    state.micro.destination = val as u32;
    state.micro.write_buffer = val as u32;
}

#[inline(always)]
pub fn alu_move_b_src_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let val = (state.micro.source & 0xFF) as u8;
    state.set_ccr_nz_clear_vc((val as i8) < 0, val == 0);
    state.micro.destination = val as u32;
    state.micro.write_buffer = val as u32;
}

#[inline(always)]
pub fn alu_move_b_src_imm(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let val = (state.prefetch[0] & 0xFF) as u8;
    state.set_ccr_nz_clear_vc((val as i8) < 0, val == 0);
    state.micro.destination = val as u32;
    state.micro.write_buffer = val as u32;
}

// ============================================================================
// Static Step Slices: MOVE.B
// ============================================================================

pub static STEPS_MOVE_B_DN_DN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_move_b_dn_dn), base_clocks: 2 }, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_DN_AI: [MicroStep; 6] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep::alu(ea::ea_calc_dst_ai), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_DN_PI: [MicroStep; 6] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep::alu(ea::ea_calc_dst_pi_b), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_DN_PD: [MicroStep; 6] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH_READ, PREFETCH_SCRATCH_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_MOVE_B_DN_D16: [MicroStep; 7] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_DN_IDX: [MicroStep; 8] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_DN_ABSW: [MicroStep; 7] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_DN_ABSL: [MicroStep; 9] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_AI_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 2 }, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_AI_AI: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_AI_PI: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_AI_PD: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH_READ, PREFETCH_SCRATCH_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_MOVE_B_AI_D16: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_AI_IDX: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_AI_ABSW: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_AI_ABSL: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PI_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 2 }, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PI_AI: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PI_PI: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PI_PD: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH_READ, PREFETCH_SCRATCH_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_MOVE_B_PI_D16: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PI_IDX: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PI_ABSW: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PI_ABSL: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 2 }, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PD_DN: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 2 }, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PD_AI: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PD_PI: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PD_PD: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH_READ, PREFETCH_SCRATCH_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_MOVE_B_PD_D16: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PD_IDX: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PD_ABSW: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PD_ABSL: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_D16_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 2 }, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_D16_AI: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_D16_PI: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_D16_PD: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH_READ, PREFETCH_SCRATCH_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_MOVE_B_D16_D16: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_D16_IDX: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_D16_ABSW: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_D16_ABSL: [MicroStep; 13] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IDX_DN: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 2 }, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IDX_AI: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IDX_PI: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IDX_PD: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH_READ, PREFETCH_SCRATCH_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_MOVE_B_IDX_D16: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IDX_IDX: [MicroStep; 13] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IDX_ABSW: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IDX_ABSL: [MicroStep; 14] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 2 }, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_AI: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_PI: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_PD: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH_READ, PREFETCH_SCRATCH_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_D16: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_IDX: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_ABSW: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_ABSL: [MicroStep; 13] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_DN: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 2 }, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_AI: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_PI: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_PD: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH_READ, PREFETCH_SCRATCH_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_D16: [MicroStep; 13] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_IDX: [MicroStep; 14] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_ABSW: [MicroStep; 13] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_ABSL: [MicroStep; 15] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 2 }, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_AI: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_PI: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_PD: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH_READ, PREFETCH_SCRATCH_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_D16: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_IDX: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_ABSW: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_ABSL: [MicroStep; 13] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 2 }, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_DN: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 2 }, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_AI: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_PI: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_PD: [MicroStep; 11] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH_READ, PREFETCH_SCRATCH_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_D16: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_IDX: [MicroStep; 13] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_ABSW: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_ABSL: [MicroStep; 14] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, READ_SRC_BYTE, READ_BYTE_FINISH, MicroStep::alu(alu_move_b_src_mem), MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IMM_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(alu_move_b_imm_dn), base_clocks: 2 }, FETCH_EXT_FINISH, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IMM_AI: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(alu_move_b_src_imm), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep::alu(ea::ea_calc_dst_ai), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IMM_PI: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(alu_move_b_src_imm), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep::alu(ea::ea_calc_dst_pi_b), BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IMM_PD: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(alu_move_b_src_imm), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH_READ, PREFETCH_SCRATCH_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_MOVE_B_IMM_D16: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(alu_move_b_src_imm), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IMM_IDX: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(alu_move_b_src_imm), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT_READ, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IMM_ABSW: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(alu_move_b_src_imm), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];
pub static STEPS_MOVE_B_IMM_ABSL: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(alu_move_b_src_imm), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 }, FETCH_EXT_FINISH, MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 }, FETCH_EXT_FINISH, BUS_WRITE_IDLE, WRITE_DST_BYTE, PREFETCH_NEXT_READ, PREFETCH_NEXT_RETIRE,
];

// ============================================================================
// Static 2D Lookup Table: MOVE.B
// ============================================================================

static MOVE_B_LOOKUP: [[&[MicroStep]; 8]; 11] = [
    [&STEPS_MOVE_B_DN_DN, &STEPS_MOVE_B_DN_AI, &STEPS_MOVE_B_DN_PI, &STEPS_MOVE_B_DN_PD, &STEPS_MOVE_B_DN_D16, &STEPS_MOVE_B_DN_IDX, &STEPS_MOVE_B_DN_ABSW, &STEPS_MOVE_B_DN_ABSL],
    [&STEPS_MOVE_B_AI_DN, &STEPS_MOVE_B_AI_AI, &STEPS_MOVE_B_AI_PI, &STEPS_MOVE_B_AI_PD, &STEPS_MOVE_B_AI_D16, &STEPS_MOVE_B_AI_IDX, &STEPS_MOVE_B_AI_ABSW, &STEPS_MOVE_B_AI_ABSL],
    [&STEPS_MOVE_B_PI_DN, &STEPS_MOVE_B_PI_AI, &STEPS_MOVE_B_PI_PI, &STEPS_MOVE_B_PI_PD, &STEPS_MOVE_B_PI_D16, &STEPS_MOVE_B_PI_IDX, &STEPS_MOVE_B_PI_ABSW, &STEPS_MOVE_B_PI_ABSL],
    [&STEPS_MOVE_B_PD_DN, &STEPS_MOVE_B_PD_AI, &STEPS_MOVE_B_PD_PI, &STEPS_MOVE_B_PD_PD, &STEPS_MOVE_B_PD_D16, &STEPS_MOVE_B_PD_IDX, &STEPS_MOVE_B_PD_ABSW, &STEPS_MOVE_B_PD_ABSL],
    [&STEPS_MOVE_B_D16_DN, &STEPS_MOVE_B_D16_AI, &STEPS_MOVE_B_D16_PI, &STEPS_MOVE_B_D16_PD, &STEPS_MOVE_B_D16_D16, &STEPS_MOVE_B_D16_IDX, &STEPS_MOVE_B_D16_ABSW, &STEPS_MOVE_B_D16_ABSL],
    [&STEPS_MOVE_B_IDX_DN, &STEPS_MOVE_B_IDX_AI, &STEPS_MOVE_B_IDX_PI, &STEPS_MOVE_B_IDX_PD, &STEPS_MOVE_B_IDX_D16, &STEPS_MOVE_B_IDX_IDX, &STEPS_MOVE_B_IDX_ABSW, &STEPS_MOVE_B_IDX_ABSL],
    [&STEPS_MOVE_B_ABSW_DN, &STEPS_MOVE_B_ABSW_AI, &STEPS_MOVE_B_ABSW_PI, &STEPS_MOVE_B_ABSW_PD, &STEPS_MOVE_B_ABSW_D16, &STEPS_MOVE_B_ABSW_IDX, &STEPS_MOVE_B_ABSW_ABSW, &STEPS_MOVE_B_ABSW_ABSL],
    [&STEPS_MOVE_B_ABSL_DN, &STEPS_MOVE_B_ABSL_AI, &STEPS_MOVE_B_ABSL_PI, &STEPS_MOVE_B_ABSL_PD, &STEPS_MOVE_B_ABSL_D16, &STEPS_MOVE_B_ABSL_IDX, &STEPS_MOVE_B_ABSL_ABSW, &STEPS_MOVE_B_ABSL_ABSL],
    [&STEPS_MOVE_B_PCD16_DN, &STEPS_MOVE_B_PCD16_AI, &STEPS_MOVE_B_PCD16_PI, &STEPS_MOVE_B_PCD16_PD, &STEPS_MOVE_B_PCD16_D16, &STEPS_MOVE_B_PCD16_IDX, &STEPS_MOVE_B_PCD16_ABSW, &STEPS_MOVE_B_PCD16_ABSL],
    [&STEPS_MOVE_B_PCIDX_DN, &STEPS_MOVE_B_PCIDX_AI, &STEPS_MOVE_B_PCIDX_PI, &STEPS_MOVE_B_PCIDX_PD, &STEPS_MOVE_B_PCIDX_D16, &STEPS_MOVE_B_PCIDX_IDX, &STEPS_MOVE_B_PCIDX_ABSW, &STEPS_MOVE_B_PCIDX_ABSL],
    [&STEPS_MOVE_B_IMM_DN, &STEPS_MOVE_B_IMM_AI, &STEPS_MOVE_B_IMM_PI, &STEPS_MOVE_B_IMM_PD, &STEPS_MOVE_B_IMM_D16, &STEPS_MOVE_B_IMM_IDX, &STEPS_MOVE_B_IMM_ABSW, &STEPS_MOVE_B_IMM_ABSL],
];

// ============================================================================
// Opcode Decoder: MOVE.B
// ============================================================================

pub const fn decode_move_b_steps(src_mode: u8, src_reg: u8, dst_mode: u8, dst_reg: u8) -> Option<&'static [MicroStep]> {
    let src_idx = match src_mode {
        0 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        5 => 4,
        6 => 5,
        7 => match src_reg { 0 => 6, 1 => 7, 2 => 8, 3 => 9, 4 => 10, _ => return None },
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
    Some(MOVE_B_LOOKUP[src_idx][dst_idx])
}
