//! M68000 ADDI Instruction (`ADDI #<imm>, <ea>`)
//!
//! Adds an immediate value to a destination data register or memory effective address.

use crate::core::{Cpu, StepResult};
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::{flags, MicroAction, MicroStep};
use crate::state::CpuState;
use memory_bus::MemoryBus;

// ============================================================================
// Micro-Step Callbacks
// ============================================================================

pub fn alu_addi_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = crate::instructions::add::add_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_addi_w_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.prefetch[0];
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = crate::instructions::add::add_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_addi_l_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.scratch[1];
    let d = state.d_long(reg_dst as usize);
    let res = crate::instructions::add::add_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn latch_imm_b(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.scratch[1] = (state.prefetch[0] & 0xFF) as u32;
}

pub fn latch_imm_w(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.scratch[1] = state.prefetch[0] as u32;
}

pub fn latch_imm_b_calc_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[1] = (state.prefetch[0] & 0xFF) as u32;
    ea::ea_calc_dst_ai(state, 0, reg_dst);
}

pub fn latch_imm_b_calc_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[1] = (state.prefetch[0] & 0xFF) as u32;
    ea::ea_calc_dst_pi_b(state, 0, reg_dst);
}

pub fn latch_imm_w_calc_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[1] = state.prefetch[0] as u32;
    ea::ea_calc_dst_ai(state, 0, reg_dst);
}

pub fn latch_imm_w_calc_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[1] = state.prefetch[0] as u32;
    ea::ea_calc_dst_pi_w(state, 0, reg_dst);
}

pub fn latch_imm_l_lo_calc_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[3] = state.micro.scratch[0] | (state.prefetch[0] as u32);
    ea::ea_calc_dst_ai(state, 0, reg_dst);
}

pub fn latch_imm_l_lo_calc_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[3] = state.micro.scratch[0] | (state.prefetch[0] as u32);
    ea::ea_calc_dst_pi_l(state, 0, reg_dst);
}

pub fn latch_imm_l_lo(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.scratch[3] = state.micro.scratch[0] | (state.prefetch[0] as u32);
}

pub fn alu_addi_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.scratch[1] & 0xFF) as u8;
    let d = (state.micro.last_read & 0xFF) as u8;
    let res = crate::instructions::add::add_b(state, s, d);
    state.micro.write_buffer = res as u32;
}

pub fn alu_addi_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.scratch[1] & 0xFFFF) as u16;
    let d = state.micro.last_read;
    let res = crate::instructions::add::add_w(state, s, d);
    state.micro.write_buffer = res as u32;
}

pub fn alu_addi_l_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.scratch[3];
    let d = state.micro.scratch[1];
    let res = crate::instructions::add::add_l(state, s, d);
    state.micro.write_buffer = res;
}

pub fn set_write_l_hi(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.write_buffer = (state.micro.scratch[1] >> 16) & 0xFFFF;
}

// ============================================================================
// Static Micro-Step Slices: ADDI Byte
// ============================================================================

pub static STEPS_ADDI_B_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_addi_b_imm_dn), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_ADDI_B_AI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_b_calc_ai), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_ADDI_B_PI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_b_calc_pi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_ADDI_B_PD: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_b), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_ADDI_B_D16: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_b), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_ADDI_B_IDX: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_b), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_ADDI_B_ABSW: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_b), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_ADDI_B_ABSL: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_b), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteByteAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

// ============================================================================
// Static Micro-Step Slices: ADDI Word
// ============================================================================

pub static STEPS_ADDI_W_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_addi_w_imm_dn), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_ADDI_W_AI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_w_calc_ai), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_ADDI_W_PI: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_w_calc_pi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_ADDI_W_PD: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_w), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_w), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_ADDI_W_D16: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_w), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_ADDI_W_IDX: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_w), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_ADDI_W_ABSW: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_w), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

pub static STEPS_ADDI_W_ABSL: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_w), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusWriteWordAndRetire, alu_fn: None, base_clocks: 4, flags: flags::WRITE | flags::DATA_SPACE },
];

// ============================================================================
// Static Micro-Step Slices: ADDI Long
// ============================================================================

pub static STEPS_ADDI_L_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_addi_l_imm_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_ADDI_L_AI: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_l_lo_calc_ai), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

pub static STEPS_ADDI_L_PI: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_l_lo_calc_pi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

pub static STEPS_ADDI_L_PD: [MicroStep; 8] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_l), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

pub static STEPS_ADDI_L_D16: [MicroStep; 8] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

pub static STEPS_ADDI_L_IDX: [MicroStep; 9] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

pub static STEPS_ADDI_L_ABSW: [MicroStep; 8] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

pub static STEPS_ADDI_L_ABSL: [MicroStep; 9] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

/// Decodes the micro-step sequence for ADDI based on size and destination EA
pub const fn decode_addi_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_ADDI_B_DN),
            2 => Some(&STEPS_ADDI_B_AI),
            3 => Some(&STEPS_ADDI_B_PI),
            4 => Some(&STEPS_ADDI_B_PD),
            5 => Some(&STEPS_ADDI_B_D16),
            6 => Some(&STEPS_ADDI_B_IDX),
            7 => match reg {
                0 => Some(&STEPS_ADDI_B_ABSW),
                1 => Some(&STEPS_ADDI_B_ABSL),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_ADDI_W_DN),
            2 => Some(&STEPS_ADDI_W_AI),
            3 => Some(&STEPS_ADDI_W_PI),
            4 => Some(&STEPS_ADDI_W_PD),
            5 => Some(&STEPS_ADDI_W_D16),
            6 => Some(&STEPS_ADDI_W_IDX),
            7 => match reg {
                0 => Some(&STEPS_ADDI_W_ABSW),
                1 => Some(&STEPS_ADDI_W_ABSL),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_ADDI_L_DN),
            2 => Some(&STEPS_ADDI_L_AI),
            3 => Some(&STEPS_ADDI_L_PI),
            4 => Some(&STEPS_ADDI_L_PD),
            5 => Some(&STEPS_ADDI_L_D16),
            6 => Some(&STEPS_ADDI_L_IDX),
            7 => match reg {
                0 => Some(&STEPS_ADDI_L_ABSW),
                1 => Some(&STEPS_ADDI_L_ABSL),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

// ============================================================================
// Legacy Stubs (to be removed in Phase 7)
// ============================================================================

pub fn op_addi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_addi_b_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_b_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_b_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_b_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_b_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_b_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_b_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_b_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }

pub fn op_addi_l_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_l_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_l_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_l_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_l_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_l_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_l_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_l_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }

pub fn op_addi_w_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_w_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_w_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_w_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_w_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_w_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_w_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
pub fn op_addi_w_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_addi(cpu, bus) }
