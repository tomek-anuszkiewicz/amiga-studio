//! BCHG (Bit Change) instruction handlers and operation logic
//!
//! Tests and inverts a single bit in a Data Register (modulo 32) or memory operand (modulo 8).
//! Updates Z flag (set if bit was 0 prior to change, cleared if bit was 1). Other condition codes unaffected.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::{MicroAction, MicroStep};
use crate::state::CpuState;

// ============================================================================
// Leaf ALU BCHG Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn bchg_l(state: &mut CpuState, bit_num: u32, val: u32) -> u32 {
    let bit_idx = bit_num & 31;
    let mask = 1 << bit_idx;
    state.set_ccr_z_only((val & mask) == 0);
    val ^ mask
}

#[inline(always)]
pub fn bchg_b(state: &mut CpuState, bit_num: u32, val: u8) -> u8 {
    let bit_idx = bit_num & 7;
    let mask = 1 << bit_idx;
    state.set_ccr_z_only((val & mask) == 0);
    val ^ mask
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_bchg_l_dyn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let bit_num = state.d_long(reg_src as usize);
    let val = state.d_long(reg_dst as usize);
    let res = bchg_l(state, bit_num, val);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_bchg_b_dyn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let bit_num = state.d_long(reg_src as usize);
    let val = (state.micro.last_read & 0xFF) as u8;
    let res = bchg_b(state, bit_num, val);
    state.micro.write_buffer = res as u32;
}

pub fn alu_bchg_l_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let bit_num = state.micro.scratch[3];
    let val = state.d_long(reg_dst as usize);
    let res = bchg_l(state, bit_num, val);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_bchg_b_imm_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let bit_num = state.micro.scratch[3];
    let val = (state.micro.last_read & 0xFF) as u8;
    let res = bchg_b(state, bit_num, val);
    state.micro.write_buffer = res as u32;
}

// ============================================================================
// Static Micro-Step Slices: Dynamic BCHG Dn, <ea>
// ============================================================================

pub static STEPS_BCHG_DYN_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_bchg_l_dyn_dn), base_clocks: 4 },
];
pub static STEPS_BCHG_DYN_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_dyn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_BCHG_DYN_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_pi_b), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_dyn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_BCHG_DYN_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_dyn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_BCHG_DYN_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_dyn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_BCHG_DYN_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_dyn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_BCHG_DYN_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_dyn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_BCHG_DYN_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_dyn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];

// ============================================================================
// Static Micro-Step Slices: Static BCHG #imm, <ea>
// ============================================================================

pub static STEPS_BCHG_STAT_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::btst::latch_bit_imm), base_clocks: 4 },
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_bchg_l_imm_dn), base_clocks: 4 },
];
pub static STEPS_BCHG_STAT_AI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::btst::latch_bit_imm_calc_ai), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_imm_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_BCHG_STAT_PI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::btst::latch_bit_imm_calc_pi), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_imm_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_BCHG_STAT_PD: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::btst::latch_bit_imm), base_clocks: 4 },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_imm_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_BCHG_STAT_D16: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::btst::latch_bit_imm), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_imm_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_BCHG_STAT_IDX: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::btst::latch_bit_imm), base_clocks: 4 },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_imm_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_BCHG_STAT_ABSW: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::btst::latch_bit_imm), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_imm_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_BCHG_STAT_ABSL: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::btst::latch_bit_imm), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_bchg_b_imm_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];

// ============================================================================
// Static Decoder Functions
// ============================================================================

/// Decodes the micro-step sequence for dynamic BCHG Dn, <ea>
pub const fn decode_bchg_dyn_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(&STEPS_BCHG_DYN_DN),
        2 => Some(&STEPS_BCHG_DYN_AI),
        3 => Some(&STEPS_BCHG_DYN_PI),
        4 => Some(&STEPS_BCHG_DYN_PD),
        5 => Some(&STEPS_BCHG_DYN_D16),
        6 => Some(&STEPS_BCHG_DYN_IDX),
        7 => match reg {
            0 => Some(&STEPS_BCHG_DYN_ABSW),
            1 => Some(&STEPS_BCHG_DYN_ABSL),
            _ => None,
        },
        _ => None,
    }
}

/// Decodes the micro-step sequence for static BCHG #imm, <ea>
pub const fn decode_bchg_stat_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(&STEPS_BCHG_STAT_DN),
        2 => Some(&STEPS_BCHG_STAT_AI),
        3 => Some(&STEPS_BCHG_STAT_PI),
        4 => Some(&STEPS_BCHG_STAT_PD),
        5 => Some(&STEPS_BCHG_STAT_D16),
        6 => Some(&STEPS_BCHG_STAT_IDX),
        7 => match reg {
            0 => Some(&STEPS_BCHG_STAT_ABSW),
            1 => Some(&STEPS_BCHG_STAT_ABSL),
            _ => None,
        },
        _ => None,
    }
}


