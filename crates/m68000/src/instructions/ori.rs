//! M68000 ORI Instruction (`ORI #<imm>, <ea>`)
//!
//! Performs bitwise OR of an immediate operand with a destination data register or
//! memory effective address.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::{MicroAction, MicroStep};
use crate::state::CpuState;

// ============================================================================
// Micro-Step Callbacks
// ============================================================================

pub fn alu_ori_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = crate::instructions::or::or_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_ori_w_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.prefetch[0];
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = crate::instructions::or::or_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_ori_l_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.scratch[1];
    let d = state.d_long(reg_dst as usize);
    let res = crate::instructions::or::or_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_ori_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.scratch[1] & 0xFF) as u8;
    let d = (state.micro.last_read & 0xFF) as u8;
    let res = crate::instructions::or::or_b(state, s, d);
    state.micro.write_buffer = res as u32;
}

pub fn alu_ori_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.scratch[1] & 0xFFFF) as u16;
    let d = state.micro.last_read;
    let res = crate::instructions::or::or_w(state, s, d);
    state.micro.write_buffer = res as u32;
}

pub fn alu_ori_l_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.scratch[3];
    let d = state.micro.scratch[1];
    let res = crate::instructions::or::or_l(state, s, d);
    state.micro.write_buffer = res;
}

// ============================================================================
// Static Micro-Step Slices: ORI Byte
// ============================================================================

pub static STEPS_ORI_B_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_ori_b_imm_dn), base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_B_AI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b_calc_ai), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_b_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_B_PI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b_calc_pi), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_b_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_B_PD: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b), base_clocks: 4 },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_b_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_B_D16: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_b_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_B_IDX: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b), base_clocks: 4 },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_b_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_B_ABSW: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_b_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_B_ABSL: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_b_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4 },
];

// ============================================================================
// Static Micro-Step Slices: ORI Word
// ============================================================================

pub static STEPS_ORI_W_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_ori_w_imm_dn), base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_W_AI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w_calc_ai), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_w_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_W_PI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w_calc_pi), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_w_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_W_PD: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w), base_clocks: 4 },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_w), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_w_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_W_D16: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_w_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_W_IDX: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w), base_clocks: 4 },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_w_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_W_ABSW: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_w_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4 },
];

pub static STEPS_ORI_W_ABSL: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_w_mem), base_clocks: 4 },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4 },
];

// ============================================================================
// Static Micro-Step Slices: ORI Long
// ============================================================================

pub static STEPS_ORI_L_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_ori_l_imm_dn), base_clocks: 4 },
];

pub static STEPS_ORI_L_AI: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo_calc_ai), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

pub static STEPS_ORI_L_PI: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo_calc_pi), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

pub static STEPS_ORI_L_PD: [MicroStep; 8] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_l), base_clocks: 2 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

pub static STEPS_ORI_L_D16: [MicroStep; 8] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

pub static STEPS_ORI_L_IDX: [MicroStep; 9] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

pub static STEPS_ORI_L_ABSW: [MicroStep; 8] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

pub static STEPS_ORI_L_ABSL: [MicroStep; 9] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4 },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_ori_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

// ============================================================================
// Static Decoder Function
// ============================================================================

/// Decodes the micro-step sequence for ORI based on size, mode, and reg
pub const fn decode_ori_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_ORI_B_DN),
            2 => Some(&STEPS_ORI_B_AI),
            3 => Some(&STEPS_ORI_B_PI),
            4 => Some(&STEPS_ORI_B_PD),
            5 => Some(&STEPS_ORI_B_D16),
            6 => Some(&STEPS_ORI_B_IDX),
            7 => match reg {
                0 => Some(&STEPS_ORI_B_ABSW),
                1 => Some(&STEPS_ORI_B_ABSL),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_ORI_W_DN),
            2 => Some(&STEPS_ORI_W_AI),
            3 => Some(&STEPS_ORI_W_PI),
            4 => Some(&STEPS_ORI_W_PD),
            5 => Some(&STEPS_ORI_W_D16),
            6 => Some(&STEPS_ORI_W_IDX),
            7 => match reg {
                0 => Some(&STEPS_ORI_W_ABSW),
                1 => Some(&STEPS_ORI_W_ABSL),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_ORI_L_DN),
            2 => Some(&STEPS_ORI_L_AI),
            3 => Some(&STEPS_ORI_L_PI),
            4 => Some(&STEPS_ORI_L_PD),
            5 => Some(&STEPS_ORI_L_D16),
            6 => Some(&STEPS_ORI_L_IDX),
            7 => match reg {
                0 => Some(&STEPS_ORI_L_ABSW),
                1 => Some(&STEPS_ORI_L_ABSL),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

pub static STEPS_ORI_CCR: [MicroStep; 1] = [MicroStep { action: MicroAction::OriToCcr, alu_fn: None, base_clocks: 0 }];
pub static STEPS_ORI_SR: [MicroStep; 1] = [MicroStep { action: MicroAction::OriToSr, alu_fn: None, base_clocks: 0 }];

