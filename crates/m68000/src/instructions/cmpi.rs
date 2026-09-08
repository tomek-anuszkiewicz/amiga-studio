//! M68000 CMPI Instruction (`CMPI #<imm>, <ea>`)
//!
//! Compares an immediate value with a destination data register or memory effective address.
//! Neither operand is modified. Extend (X) flag is unaffected.

use crate::core::{Cpu, StepResult};
use crate::micro::ea;
use crate::micro::types::{flags, MicroAction, MicroStep};
use crate::state::CpuState;
use memory_bus::MemoryBus;

// ============================================================================
// Micro-Step Callbacks
// ============================================================================

pub fn alu_cmpi_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    crate::instructions::cmp::cmp_b(state, s, d);
}

pub fn alu_cmpi_w_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.prefetch[0];
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    crate::instructions::cmp::cmp_w(state, s, d);
}

pub fn alu_cmpi_l_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.scratch[1];
    let d = state.d_long(reg_dst as usize);
    crate::instructions::cmp::cmp_l(state, s, d);
}

pub fn alu_cmpi_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.scratch[1] & 0xFF) as u8;
    let d = (state.micro.last_read & 0xFF) as u8;
    crate::instructions::cmp::cmp_b(state, s, d);
}

pub fn alu_cmpi_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.scratch[1] & 0xFFFF) as u16;
    let d = state.micro.last_read;
    crate::instructions::cmp::cmp_w(state, s, d);
}

pub fn alu_cmpi_l_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.scratch[3];
    let d = state.micro.scratch[1];
    crate::instructions::cmp::cmp_l(state, s, d);
}

// ============================================================================
// Static Micro-Step Slices: CMPI Byte
// ============================================================================

pub static STEPS_CMPI_B_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_cmpi_b_imm_dn), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_B_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b_calc_ai), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_B_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b_calc_pi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_B_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_B_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_B_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_B_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_B_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_b), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// ============================================================================
// Static Micro-Step Slices: CMPI Word
// ============================================================================

pub static STEPS_CMPI_W_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_cmpi_w_imm_dn), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_W_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w_calc_ai), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_W_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w_calc_pi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_W_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_w), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_W_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_W_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_W_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_W_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_w), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// ============================================================================
// Static Micro-Step Slices: CMPI Long
// ============================================================================

pub static STEPS_CMPI_L_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_l_imm_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_L_AI: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo_calc_ai), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_L_PI: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo_calc_pi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_L_PD: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_l), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_L_D16: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_L_IDX: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_L_ABSW: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

pub static STEPS_CMPI_L_ABSL: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(crate::instructions::addi::latch_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpi_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

/// Decodes the micro-step sequence for CMPI based on size and destination EA
pub const fn decode_cmpi_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_CMPI_B_DN),
            2 => Some(&STEPS_CMPI_B_AI),
            3 => Some(&STEPS_CMPI_B_PI),
            4 => Some(&STEPS_CMPI_B_PD),
            5 => Some(&STEPS_CMPI_B_D16),
            6 => Some(&STEPS_CMPI_B_IDX),
            7 => match reg {
                0 => Some(&STEPS_CMPI_B_ABSW),
                1 => Some(&STEPS_CMPI_B_ABSL),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_CMPI_W_DN),
            2 => Some(&STEPS_CMPI_W_AI),
            3 => Some(&STEPS_CMPI_W_PI),
            4 => Some(&STEPS_CMPI_W_PD),
            5 => Some(&STEPS_CMPI_W_D16),
            6 => Some(&STEPS_CMPI_W_IDX),
            7 => match reg {
                0 => Some(&STEPS_CMPI_W_ABSW),
                1 => Some(&STEPS_CMPI_W_ABSL),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_CMPI_L_DN),
            2 => Some(&STEPS_CMPI_L_AI),
            3 => Some(&STEPS_CMPI_L_PI),
            4 => Some(&STEPS_CMPI_L_PD),
            5 => Some(&STEPS_CMPI_L_D16),
            6 => Some(&STEPS_CMPI_L_IDX),
            7 => match reg {
                0 => Some(&STEPS_CMPI_L_ABSW),
                1 => Some(&STEPS_CMPI_L_ABSL),
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

pub fn op_cmpi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_cmpi_b_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_b_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_b_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_b_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_b_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_b_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_b_imm_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_b_imm_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_b_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_b_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }

pub fn op_cmpi_l_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_l_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_l_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_l_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_l_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_l_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_l_imm_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_l_imm_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_l_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_l_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }

pub fn op_cmpi_w_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_w_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_w_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_w_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_w_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_w_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_w_imm_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_w_imm_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_w_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
pub fn op_cmpi_w_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_cmpi(cpu, bus) }
