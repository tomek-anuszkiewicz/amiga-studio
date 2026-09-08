//! ROR (Rotate Right without Extend) instruction handlers and CCR updates
//!
//! Rotates bits to the right without using the Extend flag (X is unaffected).
//! C flag receives the last bit shifted out. V flag is always cleared.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::{MicroAction, MicroStep};
use crate::state::CpuState;

// ============================================================================
// Leaf ALU ROR Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn ror_b(state: &mut CpuState, count: u32, val: u8) -> u8 {
    let msb = (val & 0x80) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let k = count % 8;
    let (res, last_out) = if k == 0 {
        (val, (val & 0x80) != 0)
    } else {
        let r = val.rotate_right(k);
        (r, (r & 0x80) != 0)
    };
    state.set_ccr_nzc_clear_v((res & 0x80) != 0, res == 0, last_out);
    res
}

#[inline(always)]
pub fn ror_w(state: &mut CpuState, count: u32, val: u16) -> u16 {
    let msb = (val & 0x8000) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let k = count % 16;
    let (res, last_out) = if k == 0 {
        (val, (val & 0x8000) != 0)
    } else {
        let r = val.rotate_right(k);
        (r, (r & 0x8000) != 0)
    };
    state.set_ccr_nzc_clear_v((res & 0x8000) != 0, res == 0, last_out);
    res
}

#[inline(always)]
pub fn ror_l(state: &mut CpuState, count: u32, val: u32) -> u32 {
    let msb = (val & 0x8000_0000) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let k = count % 32;
    let (res, last_out) = if k == 0 {
        (val, (val & 0x8000_0000) != 0)
    } else {
        let r = val.rotate_right(k);
        (r, (r & 0x8000_0000) != 0)
    };
    state.set_ccr_nzc_clear_v((res & 0x8000_0000) != 0, res == 0, last_out);
    res
}

#[inline]
pub fn execute_ror(state: &mut CpuState, s: u8, count: u32, val: u32) -> u32 {
    match s {
        0 => ror_b(state, count, val as u8) as u32,
        1 => ror_w(state, count, val as u16) as u32,
        _ => ror_l(state, count, val),
    }
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_ror_b_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_byte(reg_dst as usize);
    let res = ror_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    state.micro.record_internal_clocks(2 + (2 * count as u16));
}

pub fn alu_ror_w_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_word(reg_dst as usize);
    let res = ror_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    state.micro.record_internal_clocks(2 + (2 * count as u16));
}

pub fn alu_ror_l_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_long(reg_dst as usize);
    let res = ror_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    state.micro.record_internal_clocks(4 + (2 * count as u16));
}

pub fn alu_ror_b_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_byte(reg_dst as usize);
    let res = ror_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    state.micro.record_internal_clocks(2 + (2 * count as u16));
}

pub fn alu_ror_w_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_word(reg_dst as usize);
    let res = ror_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    state.micro.record_internal_clocks(2 + (2 * count as u16));
}

pub fn alu_ror_l_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_long(reg_dst as usize);
    let res = ror_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    state.micro.record_internal_clocks(4 + (2 * count as u16));
}

pub fn alu_ror_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = state.micro.last_read;
    let res = ror_w(state, 1, d);
    state.micro.write_buffer = res as u32;
}

// ============================================================================
// Static Micro-Step Slices: ROR Register
// ============================================================================

pub static STEPS_ROR_B_IMM: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(alu_ror_b_imm_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];
pub static STEPS_ROR_W_IMM: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(alu_ror_w_imm_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];
pub static STEPS_ROR_L_IMM: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(alu_ror_l_imm_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];

pub static STEPS_ROR_B_REG: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(alu_ror_b_reg_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];
pub static STEPS_ROR_W_REG: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(alu_ror_w_reg_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];
pub static STEPS_ROR_L_REG: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(alu_ror_l_reg_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];

// ============================================================================
// Static Micro-Step Slices: ROR Memory (Word only, Count = 1)
// ============================================================================

pub static STEPS_ROR_W_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ror_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROR_W_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_pi_w), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ror_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROR_W_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_w), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ror_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROR_W_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ror_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROR_W_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ror_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROR_W_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ror_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROR_W_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ror_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];

// ============================================================================
// Static Decoder Functions
// ============================================================================

/// Decodes the micro-step sequence for ROR register rotate
pub const fn decode_ror_reg_steps(is_reg_count: bool, size: u8) -> Option<&'static [MicroStep]> {
    if is_reg_count {
        match size {
            0 => Some(&STEPS_ROR_B_REG),
            1 => Some(&STEPS_ROR_W_REG),
            2 => Some(&STEPS_ROR_L_REG),
            _ => None,
        }
    } else {
        match size {
            0 => Some(&STEPS_ROR_B_IMM),
            1 => Some(&STEPS_ROR_W_IMM),
            2 => Some(&STEPS_ROR_L_IMM),
            _ => None,
        }
    }
}

/// Decodes the micro-step sequence for ROR memory rotate (Word only, Count = 1)
pub const fn decode_ror_mem_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        2 => Some(&STEPS_ROR_W_AI),
        3 => Some(&STEPS_ROR_W_PI),
        4 => Some(&STEPS_ROR_W_PD),
        5 => Some(&STEPS_ROR_W_D16),
        6 => Some(&STEPS_ROR_W_IDX),
        7 => match reg {
            0 => Some(&STEPS_ROR_W_ABSW),
            1 => Some(&STEPS_ROR_W_ABSL),
            _ => None,
        },
        _ => None,
    }
}


