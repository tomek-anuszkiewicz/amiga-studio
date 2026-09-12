//! MULU (16-bit Unsigned Multiply) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions for:
//! - MULU <ea>, Dn (Unsigned multiply: 16x16 -> 32)
//!
//! Execution timing:
//! - 38-54 clocks (DN), data-dependent based on number of 1-bits in source operand:
//!   Internal idle cycles = 34 + 2 * popcount(src)

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

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
pub fn calc_mulu_internal_clocks(src: u16) -> u16 {
    34 + 2 * (src.count_ones() as u16)
}

// ============================================================================
// Pure ALU Callbacks: MULU
// ============================================================================

pub fn alu_mulu_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let src = state.d_word(reg_src as usize);
    let dst = state.d_word(reg_dst as usize);
    let res = (src as u32) * (dst as u32);
    state.set_d_long(reg_dst as usize, res);
    state.set_ccr_nz_clear_vc((res as i32) < 0, res == 0);
    state.micro.clocks_remaining = calc_mulu_internal_clocks(src);
}

pub fn alu_mulu_mem(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let src = state.micro.source as u16;
    let dst = state.d_word(reg_dst as usize);
    let res = (src as u32) * (dst as u32);
    state.set_d_long(reg_dst as usize, res);
    state.set_ccr_nz_clear_vc((res as i32) < 0, res == 0);
    state.micro.clocks_remaining = calc_mulu_internal_clocks(src);
}

// ============================================================================
// Static Micro-Step Slices: MULU
// ============================================================================

pub static STEPS_MULU_DN: [MicroStep; 3] = [
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_mulu_dn),
];

pub static STEPS_MULU_AI: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_mulu_mem),
];

pub static STEPS_MULU_PI: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_mulu_mem),
];

pub static STEPS_MULU_PD: [MicroStep; 6] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_mulu_mem),
];

pub static STEPS_MULU_D16_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_mulu_mem),
];

pub static STEPS_MULU_IDX_AN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_mulu_mem),
];

pub static STEPS_MULU_ABSW: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_mulu_mem),
];

pub static STEPS_MULU_ABSL: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_mulu_mem),
];

pub static STEPS_MULU_D16_PC: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_mulu_mem),
];

pub static STEPS_MULU_IDX_PC: [MicroStep; 8] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_mulu_mem),
];

pub static STEPS_MULU_IMM: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    PREFETCH_NEXT_READ,
    PREFETCH_IRC_FINISH,
    MicroStep::alu(alu_mulu_mem),
];

// ============================================================================
// Static Lookup Tables & Decode Functions
// ============================================================================

static MULU_LOOKUP: [&[MicroStep]; 11] = [
    &STEPS_MULU_DN,
    &STEPS_MULU_AI,
    &STEPS_MULU_PI,
    &STEPS_MULU_PD,
    &STEPS_MULU_D16_AN,
    &STEPS_MULU_IDX_AN,
    &STEPS_MULU_ABSW,
    &STEPS_MULU_ABSL,
    &STEPS_MULU_D16_PC,
    &STEPS_MULU_IDX_PC,
    &STEPS_MULU_IMM,
];

pub const fn decode_mulu_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(MULU_LOOKUP[0]),
        2 => Some(MULU_LOOKUP[1]),
        3 => Some(MULU_LOOKUP[2]),
        4 => Some(MULU_LOOKUP[3]),
        5 => Some(MULU_LOOKUP[4]),
        6 => Some(MULU_LOOKUP[5]),
        7 => match reg {
            0 => Some(MULU_LOOKUP[6]),
            1 => Some(MULU_LOOKUP[7]),
            2 => Some(MULU_LOOKUP[8]),
            3 => Some(MULU_LOOKUP[9]),
            4 => Some(MULU_LOOKUP[10]),
            _ => None,
        },
        _ => None,
    }
}
