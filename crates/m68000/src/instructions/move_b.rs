//! MOVE Byte (8-bit) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::{MicroAction, MicroStep};
use crate::state::CpuState;

// ============================================================================
// Atomic Micro-Step Constants
// ============================================================================

const READ_OP: MicroStep = MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 };
const WRITE_OP: MicroStep = MicroStep { action: MicroAction::BusWriteByte, alu_fn: None, base_clocks: 4 };
const WRITE_OP_RETIRE: MicroStep = MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4 };
const PREFETCH_SCRATCH: MicroStep = MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: None, base_clocks: 4 };
const FETCH_EXT: MicroStep = MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 };
const PREFETCH_RETIRE: MicroStep = common::RETIRE_STANDARD;

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
    let val = (state.micro.last_read & 0xFF) as u8;
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
    state.micro.write_buffer = val as u32;
}

#[inline(always)]
pub fn alu_move_b_src_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let val = (state.micro.last_read & 0xFF) as u8;
    state.set_ccr_nz_clear_vc((val as i8) < 0, val == 0);
    state.micro.write_buffer = val as u32;
}

#[inline(always)]
pub fn alu_move_b_src_imm(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let val = (state.prefetch[0] & 0xFF) as u8;
    state.set_ccr_nz_clear_vc((val as i8) < 0, val == 0);
    state.micro.write_buffer = val as u32;
}

// ============================================================================
// Static Step Slices: MOVE.B
// ============================================================================

pub static STEPS_MOVE_B_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_move_b_dn_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_B_DN_AI: [MicroStep; 4] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_DN_PI: [MicroStep; 4] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep::alu(ea::ea_calc_dst_pi_b),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_DN_PD: [MicroStep; 4] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep::alu(ea::ea_calc_dst_pd_b),
    PREFETCH_SCRATCH, WRITE_OP_RETIRE,
];
pub static STEPS_MOVE_B_DN_D16: [MicroStep; 4] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_DN_IDX: [MicroStep; 5] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_DN_ABSW: [MicroStep; 4] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_DN_ABSL: [MicroStep; 5] = [
    MicroStep::alu(alu_move_b_src_dn), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_AI_DN: [MicroStep; 3] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_OP,
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_B_AI_AI: [MicroStep; 6] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_AI_PI: [MicroStep; 6] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_AI_PD: [MicroStep; 6] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b),
    PREFETCH_SCRATCH, WRITE_OP_RETIRE,
];
pub static STEPS_MOVE_B_AI_D16: [MicroStep; 6] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_AI_IDX: [MicroStep; 7] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_AI_ABSW: [MicroStep; 6] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_AI_ABSL: [MicroStep; 7] = [
    MicroStep::alu(ea::ea_calc_src_ai), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PI_DN: [MicroStep; 3] = [
    MicroStep::alu(ea::ea_calc_src_pi_b), READ_OP,
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_B_PI_AI: [MicroStep; 6] = [
    MicroStep::alu(ea::ea_calc_src_pi_b), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PI_PI: [MicroStep; 6] = [
    MicroStep::alu(ea::ea_calc_src_pi_b), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PI_PD: [MicroStep; 6] = [
    MicroStep::alu(ea::ea_calc_src_pi_b), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b),
    PREFETCH_SCRATCH, WRITE_OP_RETIRE,
];
pub static STEPS_MOVE_B_PI_D16: [MicroStep; 6] = [
    MicroStep::alu(ea::ea_calc_src_pi_b), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PI_IDX: [MicroStep; 7] = [
    MicroStep::alu(ea::ea_calc_src_pi_b), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PI_ABSW: [MicroStep; 6] = [
    MicroStep::alu(ea::ea_calc_src_pi_b), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PI_ABSL: [MicroStep; 7] = [
    MicroStep::alu(ea::ea_calc_src_pi_b), READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PD_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_OP,
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_B_PD_AI: [MicroStep; 6] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PD_PI: [MicroStep; 6] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PD_PD: [MicroStep; 6] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b),
    PREFETCH_SCRATCH, WRITE_OP_RETIRE,
];
pub static STEPS_MOVE_B_PD_D16: [MicroStep; 6] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PD_IDX: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PD_ABSW: [MicroStep; 6] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PD_ABSL: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_D16_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_OP,
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_B_D16_AI: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_D16_PI: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_D16_PD: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b),
    PREFETCH_SCRATCH, WRITE_OP_RETIRE,
];
pub static STEPS_MOVE_B_D16_D16: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_D16_IDX: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_D16_ABSW: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_D16_ABSL: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IDX_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_B_IDX_AI: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep::alu(ea::ea_calc_dst_ai), WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IDX_PI: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep::alu(ea::ea_calc_dst_pi_b), WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IDX_PD: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH,
    WRITE_OP_RETIRE,
];
pub static STEPS_MOVE_B_IDX_D16: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IDX_IDX: [MicroStep; 8] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT,
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IDX_ABSW: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IDX_ABSL: [MicroStep; 8] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_OP,
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_B_ABSW_AI: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_PI: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_PD: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b),
    PREFETCH_SCRATCH, WRITE_OP_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_D16: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_IDX: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_ABSW: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_ABSW_ABSL: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_OP, MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_B_ABSL_AI: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep::alu(ea::ea_calc_dst_ai), WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_PI: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep::alu(ea::ea_calc_dst_pi_b), WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_PD: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH,
    WRITE_OP_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_D16: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_IDX: [MicroStep; 8] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT,
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_ABSW: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_ABSL_ABSL: [MicroStep; 8] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_OP,
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_B_PCD16_AI: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_PI: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pi_b),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_PD: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep::alu(ea::ea_calc_dst_pd_b),
    PREFETCH_SCRATCH, WRITE_OP_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_D16: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_IDX: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_ABSW: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PCD16_ABSL: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 }, READ_OP,
    MicroStep::alu(alu_move_b_src_mem), MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_move_b_mem_dn), base_clocks: 4 },
];
pub static STEPS_MOVE_B_PCIDX_AI: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep::alu(ea::ea_calc_dst_ai), WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_PI: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep::alu(ea::ea_calc_dst_pi_b), WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_PD: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep::alu(ea::ea_calc_dst_pd_b), PREFETCH_SCRATCH,
    WRITE_OP_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_D16: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_IDX: [MicroStep; 8] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 }, FETCH_EXT,
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_ABSW: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_PCIDX_ABSL: [MicroStep; 8] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 }, FETCH_EXT,
    READ_OP, MicroStep::alu(alu_move_b_src_mem),
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IMM_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_move_b_imm_dn), base_clocks: 4 }, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IMM_AI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_move_b_src_imm), base_clocks: 4 }, MicroStep::alu(ea::ea_calc_dst_ai),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IMM_PI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_move_b_src_imm), base_clocks: 4 }, MicroStep::alu(ea::ea_calc_dst_pi_b),
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IMM_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_move_b_src_imm), base_clocks: 4 }, MicroStep::alu(ea::ea_calc_dst_pd_b),
    PREFETCH_SCRATCH, WRITE_OP_RETIRE,
];
pub static STEPS_MOVE_B_IMM_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_move_b_src_imm), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IMM_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_move_b_src_imm), base_clocks: 4 }, MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    FETCH_EXT, WRITE_OP,
    PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IMM_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_move_b_src_imm), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    WRITE_OP, PREFETCH_RETIRE,
];
pub static STEPS_MOVE_B_IMM_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_move_b_src_imm), base_clocks: 4 }, MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 }, WRITE_OP,
    PREFETCH_RETIRE,
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
