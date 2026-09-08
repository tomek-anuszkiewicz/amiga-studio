//! ROXL (Rotate with Extend Left) instruction handlers and CCR updates
//!
//! Rotates bits to the left through the Extend (X) flag.
//! C flag receives the final state of the X flag. V flag is always cleared.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::{MicroAction, MicroStep};
use crate::state::CpuState;

// ============================================================================
// Leaf ALU ROXL Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn roxl_b(state: &mut CpuState, count: u32, val: u8) -> u8 {
    let msb = (val & 0x80) != 0;
    if count == 0 {
        let x = state.get_x();
        state.set_ccr_nzc_clear_v(msb, val == 0, x);
        return val;
    }
    let mut x = state.get_x();
    let mut v = val;
    for _ in 0..count {
        let carry_in = if x { 1 } else { 0 };
        let carry_out = (v & 0x80) != 0;
        v = (v << 1) | carry_in;
        x = carry_out;
    }
    let new_msb = (v & 0x80) != 0;
    state.set_ccr_xnzvc(x, new_msb, v == 0, false, x);
    v
}

#[inline(always)]
pub fn roxl_w(state: &mut CpuState, count: u32, val: u16) -> u16 {
    let msb = (val & 0x8000) != 0;
    if count == 0 {
        let x = state.get_x();
        state.set_ccr_nzc_clear_v(msb, val == 0, x);
        return val;
    }
    let mut x = state.get_x();
    let mut v = val;
    for _ in 0..count {
        let carry_in = if x { 1 } else { 0 };
        let carry_out = (v & 0x8000) != 0;
        v = (v << 1) | carry_in;
        x = carry_out;
    }
    let new_msb = (v & 0x8000) != 0;
    state.set_ccr_xnzvc(x, new_msb, v == 0, false, x);
    v
}

#[inline(always)]
pub fn roxl_l(state: &mut CpuState, count: u32, val: u32) -> u32 {
    let msb = (val & 0x8000_0000) != 0;
    if count == 0 {
        let x = state.get_x();
        state.set_ccr_nzc_clear_v(msb, val == 0, x);
        return val;
    }
    let mut x = state.get_x();
    let mut v = val;
    for _ in 0..count {
        let carry_in = if x { 1 } else { 0 };
        let carry_out = (v & 0x8000_0000) != 0;
        v = (v << 1) | carry_in;
        x = carry_out;
    }
    let new_msb = (v & 0x8000_0000) != 0;
    state.set_ccr_xnzvc(x, new_msb, v == 0, false, x);
    v
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_roxl_b_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_byte(reg_dst as usize);
    let res = roxl_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    state.micro.record_internal_clocks(2 + (2 * count as u16));
}

pub fn alu_roxl_w_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_word(reg_dst as usize);
    let res = roxl_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    state.micro.record_internal_clocks(2 + (2 * count as u16));
}

pub fn alu_roxl_l_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_long(reg_dst as usize);
    let res = roxl_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    state.micro.record_internal_clocks(4 + (2 * count as u16));
}

pub fn alu_roxl_b_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_byte(reg_dst as usize);
    let res = roxl_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    state.micro.record_internal_clocks(2 + (2 * count as u16));
}

pub fn alu_roxl_w_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_word(reg_dst as usize);
    let res = roxl_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    state.micro.record_internal_clocks(2 + (2 * count as u16));
}

pub fn alu_roxl_l_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_long(reg_dst as usize);
    let res = roxl_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    state.micro.record_internal_clocks(4 + (2 * count as u16));
}

pub fn alu_roxl_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = state.micro.last_read;
    let res = roxl_w(state, 1, d);
    state.micro.write_buffer = res as u32;
}

// ============================================================================
// Static Micro-Step Slices: ROXL Register
// ============================================================================

pub static STEPS_ROXL_B_IMM: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(alu_roxl_b_imm_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];
pub static STEPS_ROXL_W_IMM: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(alu_roxl_w_imm_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];
pub static STEPS_ROXL_L_IMM: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(alu_roxl_l_imm_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];

pub static STEPS_ROXL_B_REG: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(alu_roxl_b_reg_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];
pub static STEPS_ROXL_W_REG: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(alu_roxl_w_reg_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];
pub static STEPS_ROXL_L_REG: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(alu_roxl_l_reg_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];

// ============================================================================
// Static Micro-Step Slices: ROXL Memory (Word only, Count = 1)
// ============================================================================

pub static STEPS_ROXL_W_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_roxl_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROXL_W_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_pi_w), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_roxl_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROXL_W_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_w), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_roxl_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROXL_W_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_roxl_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROXL_W_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_roxl_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROXL_W_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_roxl_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROXL_W_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_roxl_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];

// ============================================================================
// Static Decoder Functions
// ============================================================================

/// Decodes the micro-step sequence for ROXL register rotate
pub const fn decode_roxl_reg_steps(is_reg_count: bool, size: u8) -> Option<&'static [MicroStep]> {
    if is_reg_count {
        match size {
            0 => Some(&STEPS_ROXL_B_REG),
            1 => Some(&STEPS_ROXL_W_REG),
            2 => Some(&STEPS_ROXL_L_REG),
            _ => None,
        }
    } else {
        match size {
            0 => Some(&STEPS_ROXL_B_IMM),
            1 => Some(&STEPS_ROXL_W_IMM),
            2 => Some(&STEPS_ROXL_L_IMM),
            _ => None,
        }
    }
}

/// Decodes the micro-step sequence for ROXL memory rotate (Word only, Count = 1)
pub const fn decode_roxl_mem_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        2 => Some(&STEPS_ROXL_W_AI),
        3 => Some(&STEPS_ROXL_W_PI),
        4 => Some(&STEPS_ROXL_W_PD),
        5 => Some(&STEPS_ROXL_W_D16),
        6 => Some(&STEPS_ROXL_W_IDX),
        7 => match reg {
            0 => Some(&STEPS_ROXL_W_ABSW),
            1 => Some(&STEPS_ROXL_W_ABSL),
            _ => None,
        },
        _ => None,
    }
}


