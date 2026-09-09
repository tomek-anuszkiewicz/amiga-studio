//! M68000 CMPI Instruction (`CMPI #<imm>, <ea>`)
//!
//! Compares immediate operand with effective address operand: `<ea> - #<imm>`.
//! Condition codes (N, Z, V, C) are set according to the result.
//! Neither operand is altered. Extend (X) flag is unaffected.

pub mod byte_word;
pub mod long;

pub use byte_word::*;
pub use long::*;

use crate::instructions::cmpm::{cmp_b, cmp_l, cmp_w};
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_cmpi_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    cmp_b(state, s, d);
}

pub fn alu_cmpi_w_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.prefetch[0];
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    cmp_w(state, s, d);
}

pub fn alu_cmpi_l_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.source;
    let d = state.d_long(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn latch_imm_b(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.source = (state.prefetch[0] & 0xFF) as u32;
}

pub fn latch_imm_w(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.source = state.prefetch[0] as u32;
}

pub fn latch_imm_b_calc_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.source = (state.prefetch[0] & 0xFF) as u32;
    ea::ea_calc_dst_ai(state, 0, reg_dst);
}

pub fn latch_imm_b_calc_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.source = (state.prefetch[0] & 0xFF) as u32;
    ea::ea_calc_dst_pi_b(state, 0, reg_dst);
}

pub fn latch_imm_w_calc_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.source = state.prefetch[0] as u32;
    ea::ea_calc_dst_ai(state, 0, reg_dst);
}

pub fn latch_imm_w_calc_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.source = state.prefetch[0] as u32;
    ea::ea_calc_dst_pi_w(state, 0, reg_dst);
}

pub fn latch_imm_l_hi(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.source = (state.prefetch[0] as u32) << 16;
}

pub fn latch_imm_l_lo_calc_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.source |= state.prefetch[0] as u32;
    ea::ea_calc_dst_ai(state, 0, reg_dst);
}

pub fn latch_imm_l_lo_calc_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.source |= state.prefetch[0] as u32;
    ea::ea_calc_dst_pi_l(state, 0, reg_dst);
}

pub fn latch_imm_l_lo(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.source |= state.prefetch[0] as u32;
}

pub fn alu_cmpi_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.source & 0xFF) as u8;
    let d = (state.micro.destination & 0xFF) as u8;
    cmp_b(state, s, d);
}

pub fn alu_cmpi_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.source & 0xFFFF) as u16;
    let d = (state.micro.destination & 0xFFFF) as u16;
    cmp_w(state, s, d);
}

pub fn alu_cmpi_l_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.source;
    let d = state.micro.destination;
    cmp_l(state, s, d);
}

// ============================================================================
// Instruction Decoder Callback
// ============================================================================

pub const fn decode_cmpi_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_CMPI_B_DN),
            2 => Some(&STEPS_CMPI_B_AI),
            3 => Some(&STEPS_CMPI_B_PI),
            4 => Some(&STEPS_CMPI_B_PD),
            5 => Some(&STEPS_CMPI_B_D16_AN),
            6 => Some(&STEPS_CMPI_B_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_CMPI_B_ABSW),
                1 => Some(&STEPS_CMPI_B_ABSL),
                2 => Some(&STEPS_CMPI_B_PC_D16),
                3 => Some(&STEPS_CMPI_B_PC_IDX),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_CMPI_W_DN),
            2 => Some(&STEPS_CMPI_W_AI),
            3 => Some(&STEPS_CMPI_W_PI),
            4 => Some(&STEPS_CMPI_W_PD),
            5 => Some(&STEPS_CMPI_W_D16_AN),
            6 => Some(&STEPS_CMPI_W_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_CMPI_W_ABSW),
                1 => Some(&STEPS_CMPI_W_ABSL),
                2 => Some(&STEPS_CMPI_W_PC_D16),
                3 => Some(&STEPS_CMPI_W_PC_IDX),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_CMPI_L_DN),
            2 => Some(&STEPS_CMPI_L_AI),
            3 => Some(&STEPS_CMPI_L_PI),
            4 => Some(&STEPS_CMPI_L_PD),
            5 => Some(&STEPS_CMPI_L_D16_AN),
            6 => Some(&STEPS_CMPI_L_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_CMPI_L_ABSW),
                1 => Some(&STEPS_CMPI_L_ABSL),
                2 => Some(&STEPS_CMPI_L_PC_D16),
                3 => Some(&STEPS_CMPI_L_PC_IDX),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
