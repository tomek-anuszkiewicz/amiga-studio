//! M68000 EOR Instruction (`EOR Dn, <ea>`)
//!
//! Evaluates bitwise exclusive OR between source data register and destination,
//! updating CCR flags (N, Z set according to result, V and C cleared, X unaffected).

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::{MicroAction, MicroStep, Size};
use crate::state::CpuState;

// ============================================================================
// Leaf ALU EOR Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn eor_b(state: &mut CpuState, s: u8, d: u8) -> u8 {
    let res = d ^ s;
    state.set_ccr_nz_clear_vc((res & 0x80) != 0, res == 0);
    res
}

#[inline(always)]
pub fn eor_w(state: &mut CpuState, s: u16, d: u16) -> u16 {
    let res = d ^ s;
    state.set_ccr_nz_clear_vc((res & 0x8000) != 0, res == 0);
    res
}

#[inline(always)]
pub fn eor_l(state: &mut CpuState, s: u32, d: u32) -> u32 {
    let res = d ^ s;
    state.set_ccr_nz_clear_vc((res & 0x8000_0000) != 0, res == 0);
    res
}

pub fn execute_eor(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    match size {
        Size::Byte => {
            let res = eor_b(state, (src & 0xFF) as u8, (dst & 0xFF) as u8);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let res = eor_w(state, (src & 0xFFFF) as u16, (dst & 0xFFFF) as u16);
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => eor_l(state, src, dst),
    }
}

// ============================================================================
// Micro-Step ALU Callbacks: Dn, <ea>
// ============================================================================

pub fn alu_eor_b_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = eor_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_eor_w_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = eor_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_eor_l_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = eor_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_eor_b_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.micro.last_read & 0xFF) as u8;
    let res = eor_b(state, s, d);
    state.micro.write_buffer = res as u32;
}

pub fn alu_eor_w_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = state.micro.last_read;
    let res = eor_w(state, s, d);
    state.micro.write_buffer = res as u32;
}

pub fn alu_eor_l_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.micro.scratch[1];
    let res = eor_l(state, s, d);
    state.micro.write_buffer = res;
}

// ============================================================================
// Static Micro-Step Slices: EOR Dn, <ea>
// ============================================================================

// Byte
pub static STEPS_EOR_B_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_eor_b_dn_dn), base_clocks: 4 },
];
pub static STEPS_EOR_B_DN_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_b_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_EOR_B_DN_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_pi_b), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_b_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_EOR_B_DN_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_b_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_EOR_B_DN_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_b_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_EOR_B_DN_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_b_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_EOR_B_DN_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_b_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_EOR_B_DN_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_b_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];

// Word
pub static STEPS_EOR_W_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_eor_w_dn_dn), base_clocks: 4 },
];
pub static STEPS_EOR_W_DN_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_w_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_EOR_W_DN_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_pi_w), base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_w_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_EOR_W_DN_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_w), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_w_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_EOR_W_DN_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_w_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_EOR_W_DN_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_w_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_EOR_W_DN_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_w_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_EOR_W_DN_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_w_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];

// Long
pub static STEPS_EOR_L_DN_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_eor_l_dn_dn), base_clocks: 4 },
];
pub static STEPS_EOR_L_DN_AI: [MicroStep; 5] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_l_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_EOR_L_DN_PI: [MicroStep; 5] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_dst_pi_l), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_l_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_EOR_L_DN_PD: [MicroStep; 6] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_l), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_l_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_EOR_L_DN_D16: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_l_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_EOR_L_DN_IDX: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_l_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_EOR_L_DN_ABSW: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_l_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_EOR_L_DN_ABSL: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_eor_l_dn_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

// ============================================================================
// Static Decoder Function
// ============================================================================

/// Decodes the micro-step sequence for EOR based on size, mode, and reg
pub const fn decode_eor_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_EOR_B_DN_DN),
            2 => Some(&STEPS_EOR_B_DN_AI),
            3 => Some(&STEPS_EOR_B_DN_PI),
            4 => Some(&STEPS_EOR_B_DN_PD),
            5 => Some(&STEPS_EOR_B_DN_D16),
            6 => Some(&STEPS_EOR_B_DN_IDX),
            7 => match reg {
                0 => Some(&STEPS_EOR_B_DN_ABSW),
                1 => Some(&STEPS_EOR_B_DN_ABSL),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_EOR_W_DN_DN),
            2 => Some(&STEPS_EOR_W_DN_AI),
            3 => Some(&STEPS_EOR_W_DN_PI),
            4 => Some(&STEPS_EOR_W_DN_PD),
            5 => Some(&STEPS_EOR_W_DN_D16),
            6 => Some(&STEPS_EOR_W_DN_IDX),
            7 => match reg {
                0 => Some(&STEPS_EOR_W_DN_ABSW),
                1 => Some(&STEPS_EOR_W_DN_ABSL),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_EOR_L_DN_DN),
            2 => Some(&STEPS_EOR_L_DN_AI),
            3 => Some(&STEPS_EOR_L_DN_PI),
            4 => Some(&STEPS_EOR_L_DN_PD),
            5 => Some(&STEPS_EOR_L_DN_D16),
            6 => Some(&STEPS_EOR_L_DN_IDX),
            7 => match reg {
                0 => Some(&STEPS_EOR_L_DN_ABSW),
                1 => Some(&STEPS_EOR_L_DN_ABSL),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
