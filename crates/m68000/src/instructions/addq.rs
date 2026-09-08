//! M68000 ADDQ Instruction (`ADDQ #<data>, <ea>`)
//!
//! Adds an immediate 3-bit value (1..8) to a register or memory location.
//! When target is an address register An, CCR flags are unaffected.

use crate::core::{Cpu, StepResult};
use crate::instructions::add::{add_b, add_l, add_w};
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::{flags, MicroAction, MicroStep};
use crate::state::CpuState;
use memory_bus::MemoryBus;

// ============================================================================
// Micro-Step ALU Callbacks (AluFn)
// ============================================================================

pub fn alu_addq_b_imm_dn(state: &mut CpuState, imm: u8, reg_dst: u8) {
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = add_b(state, imm, d, true);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_addq_w_imm_dn(state: &mut CpuState, imm: u8, reg_dst: u8) {
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = add_w(state, imm as u16, d, true);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_addq_l_imm_dn(state: &mut CpuState, imm: u8, reg_dst: u8) {
    let d = state.d_long(reg_dst as usize);
    let res = add_l(state, imm as u32, d, true);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_addq_w_imm_an(state: &mut CpuState, imm: u8, reg_dst: u8) {
    let a = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, a.wrapping_add(imm as u32));
}

pub fn alu_addq_l_imm_an(state: &mut CpuState, imm: u8, reg_dst: u8) {
    let a = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, a.wrapping_add(imm as u32));
}

pub fn alu_addq_b_imm_mem(state: &mut CpuState, imm: u8, _reg_dst: u8) {
    let d = (state.micro.last_read & 0xFF) as u8;
    let res = add_b(state, imm, d, true);
    state.micro.write_buffer = res as u32;
}

pub fn alu_addq_w_imm_mem(state: &mut CpuState, imm: u8, _reg_dst: u8) {
    let d = state.micro.last_read;
    let res = add_w(state, imm as u16, d, true);
    state.micro.write_buffer = res as u32;
}

pub fn alu_addq_l_imm_mem(state: &mut CpuState, imm: u8, _reg_dst: u8) {
    let d = state.micro.scratch[1];
    let res = add_l(state, imm as u32, d, true);
    state.micro.write_buffer = res;
}

// ============================================================================
// Static Micro-Step Slices: ADDQ
// ============================================================================

// Register Direct
pub static STEPS_ADDQ_B_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_addq_b_imm_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADDQ_W_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_addq_w_imm_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADDQ_L_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_addq_l_imm_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADDQ_W_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_addq_w_imm_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADDQ_L_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_addq_l_imm_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// Byte RMW
pub static STEPS_ADDQ_B_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_ADDQ_B_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_pi_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_ADDQ_B_PD: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_ADDQ_B_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_ADDQ_B_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_ADDQ_B_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_ADDQ_B_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];

// Word RMW
pub static STEPS_ADDQ_W_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_w_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ADDQ_W_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_pi_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_w_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ADDQ_W_PD: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_pd_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_w_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ADDQ_W_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_w_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ADDQ_W_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_w_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ADDQ_W_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_w_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ADDQ_W_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_w_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];

// Long RMW
pub static STEPS_ADDQ_L_AI: [MicroStep; 5] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_l_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_ADDQ_L_PI: [MicroStep; 5] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_dst_pi_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_l_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_ADDQ_L_PD: [MicroStep; 5] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_dst_pd_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_l_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_ADDQ_L_D16: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_l_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_ADDQ_L_IDX: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_l_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_ADDQ_L_ABSW: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_l_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_ADDQ_L_ABSL: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_addq_l_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

// ============================================================================
// Opcode Descriptor Decoder Helper for ADDQ
// ============================================================================

/// Maps an ADDQ opcode's bit fields to its static micro-step sequence
pub const fn decode_addq_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_ADDQ_B_DN),
            2 => Some(&STEPS_ADDQ_B_AI),
            3 => Some(&STEPS_ADDQ_B_PI),
            4 => Some(&STEPS_ADDQ_B_PD),
            5 => Some(&STEPS_ADDQ_B_D16),
            6 => Some(&STEPS_ADDQ_B_IDX),
            7 => match reg {
                0 => Some(&STEPS_ADDQ_B_ABSW),
                1 => Some(&STEPS_ADDQ_B_ABSL),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_ADDQ_W_DN),
            1 => Some(&STEPS_ADDQ_W_AN),
            2 => Some(&STEPS_ADDQ_W_AI),
            3 => Some(&STEPS_ADDQ_W_PI),
            4 => Some(&STEPS_ADDQ_W_PD),
            5 => Some(&STEPS_ADDQ_W_D16),
            6 => Some(&STEPS_ADDQ_W_IDX),
            7 => match reg {
                0 => Some(&STEPS_ADDQ_W_ABSW),
                1 => Some(&STEPS_ADDQ_W_ABSL),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_ADDQ_L_DN),
            1 => Some(&STEPS_ADDQ_L_AN),
            2 => Some(&STEPS_ADDQ_L_AI),
            3 => Some(&STEPS_ADDQ_L_PI),
            4 => Some(&STEPS_ADDQ_L_PD),
            5 => Some(&STEPS_ADDQ_L_D16),
            6 => Some(&STEPS_ADDQ_L_IDX),
            7 => match reg {
                0 => Some(&STEPS_ADDQ_L_ABSW),
                1 => Some(&STEPS_ADDQ_L_ABSL),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

// ============================================================================
// Legacy compatibility forwarders
// ============================================================================
pub fn op_addq(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_b_imm_absl(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_b_imm_absw(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_b_imm_ai(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_b_imm_disp(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_b_imm_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_b_imm_idx(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_b_imm_pd(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_b_imm_pi(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_l_imm_absl(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_l_imm_absw(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_l_imm_ai(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_l_imm_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_l_imm_disp(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_l_imm_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_l_imm_idx(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_l_imm_pd(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_l_imm_pi(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_w_imm_absl(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_w_imm_absw(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_w_imm_ai(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_w_imm_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_w_imm_disp(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_w_imm_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_w_imm_idx(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_w_imm_pd(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_addq_w_imm_pi(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
