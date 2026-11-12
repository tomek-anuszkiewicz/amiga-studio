//! MULS (16-bit Signed Multiply) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions for:
//! - MULS <ea>, Dn (Signed multiply: 16x16 -> 32)
//!
//! Execution timing:
//! - 38-70 clocks (DN), data-dependent based on sign/bit changes in source operand:
//!   Internal idle cycles = MULTIPLY_BASE_CLOCKS + MULTIPLY_CLOCKS_PER_BIT * popcount(((src << 1) ^ src) & 0xFFFF)

use super::{MULTIPLY_BASE_CLOCKS, MULTIPLY_CLOCKS_PER_BIT};
use crate::micro::common;
use crate::micro::ea;
use crate::micro::encoding::{extended_mode, mode};
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Atomic Micro-Step Constants
// ============================================================================

const BUS_READ_IDLE: MicroStep = common::BUS_READ_IDLE;
const READ_SRC_WORD: MicroStep = common::READ_SRC_WORD;
const PREFETCH_NEXT_READ: MicroStep = common::PREFETCH_NEXT_READ;
const PREFETCH_IRC_FINISH: MicroStep = common::PREFETCH_IRC_FINISH;
const FETCH_EXT_READ: MicroStep = common::FETCH_EXT_READ;
const FETCH_EXT_FINISH: MicroStep = common::FETCH_EXT_FINISH;

// ============================================================================
// Cycle Calculation Helper
// ============================================================================

#[inline(always)]
pub fn calc_muls_internal_clocks(src: u16) -> u16 {
    let d = ((src << 1) ^ src) & 0xFFFF;
    MULTIPLY_BASE_CLOCKS + MULTIPLY_CLOCKS_PER_BIT * (d.count_ones() as u16)
}

// ============================================================================
// Pure ALU Callbacks: MULS
// ============================================================================

pub fn alu_muls_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let src = state.d_word(reg_src as usize) as i16;
    let dst = state.d_word(reg_dst as usize) as i16;
    let res = (src as i32) * (dst as i32);
    state.set_d_long(reg_dst as usize, res as u32);
    state.set_ccr_nz_clear_vc(res < 0, res == 0);
    state.micro_mut().clocks_remaining = calc_muls_internal_clocks(src as u16);
}

pub fn alu_muls_mem(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let src = state.micro().source as i16;
    let dst = state.d_word(reg_dst as usize) as i16;
    let res = (src as i32) * (dst as i32);
    state.set_d_long(reg_dst as usize, res as u32);
    state.set_ccr_nz_clear_vc(res < 0, res == 0);
    state.micro_mut().clocks_remaining = calc_muls_internal_clocks(src as u16);
}

// ============================================================================
// Static Micro-Step Slices: MULS
// ============================================================================

pub static STEPS_MULS_DN: [MicroStep; 3] = [
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_muls_dn),
];

pub static STEPS_MULS_AI: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_muls_mem),
];

pub static STEPS_MULS_PI: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_muls_mem),
];

pub static STEPS_MULS_PD: [MicroStep; 6] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_muls_mem),
];

pub static STEPS_MULS_D16_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_muls_mem),
];

pub static STEPS_MULS_IDX_AN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_muls_mem),
];

pub static STEPS_MULS_ABSW: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_muls_mem),
];

pub static STEPS_MULS_ABSL: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_muls_mem),
];

pub static STEPS_MULS_D16_PC: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_muls_mem),
];

pub static STEPS_MULS_IDX_PC: [MicroStep; 8] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_muls_mem),
];

pub static STEPS_MULS_IMM: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_FINISH,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_muls_mem),
];

// ============================================================================
// Static Lookup Tables & Decode Functions
// ============================================================================

static MULS_LOOKUP: [&[MicroStep]; 11] = [
    &STEPS_MULS_DN,
    &STEPS_MULS_AI,
    &STEPS_MULS_PI,
    &STEPS_MULS_PD,
    &STEPS_MULS_D16_AN,
    &STEPS_MULS_IDX_AN,
    &STEPS_MULS_ABSW,
    &STEPS_MULS_ABSL,
    &STEPS_MULS_D16_PC,
    &STEPS_MULS_IDX_PC,
    &STEPS_MULS_IMM,
];

pub const fn decode_muls_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        mode::DATA_REGISTER => Some(MULS_LOOKUP[0]),
        mode::INDIRECT => Some(MULS_LOOKUP[1]),
        mode::POSTINCREMENT => Some(MULS_LOOKUP[2]),
        mode::PREDECREMENT => Some(MULS_LOOKUP[3]),
        mode::DISPLACEMENT => Some(MULS_LOOKUP[4]),
        mode::INDEXED => Some(MULS_LOOKUP[5]),
        mode::EXTENDED => match reg {
            extended_mode::ABSOLUTE_WORD => Some(MULS_LOOKUP[6]),
            extended_mode::ABSOLUTE_LONG => Some(MULS_LOOKUP[7]),
            extended_mode::PC_DISPLACEMENT => Some(MULS_LOOKUP[8]),
            extended_mode::PC_INDEXED => Some(MULS_LOOKUP[9]),
            extended_mode::IMMEDIATE => Some(MULS_LOOKUP[10]),
            _ => None,
        },
        _ => None,
    }
}
