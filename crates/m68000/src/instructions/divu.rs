//! DIVU (16-bit Unsigned Divide) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions for:
//! - DIVU <ea>, Dn (Unsigned divide: 32 / 16 -> 16q:16r)
//!
//! Features:
//! - Divide-by-zero Trap Vector 5 exception processing (38 clocks total).
//! - Overflow detection (V=1, C=0, destination unchanged, 10 clocks).
//! - Data-dependent cycle timing for normal non-overflow divisions.

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
const FETCH_EXT_READ: MicroStep = common::FETCH_EXT_READ;
const FETCH_EXT_FINISH: MicroStep = common::FETCH_EXT_FINISH;

// ============================================================================
// Cycle Calculation Helper
// ============================================================================

#[inline(always)]
pub fn calc_divu_internal_clocks(dividend: u32, divisor: u16) -> u16 {
    let mut d = dividend;
    let hdivisor = (divisor as u32) << 16;
    let mut mcycles = 38_u16;

    for _ in 0..15 {
        if (d as i32) < 0 {
            d = d.wrapping_shl(1);
            d = d.wrapping_sub(hdivisor);
        } else {
            d = d.wrapping_shl(1);
            if d >= hdivisor {
                d = d.wrapping_sub(hdivisor);
                mcycles += 1;
            } else {
                mcycles += 2;
            }
        }
    }
    (2 * mcycles).saturating_sub(4)
}

// ============================================================================
// Pure ALU Callbacks: DIVU
// ============================================================================

pub fn alu_divu_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let divisor = state.d_word(reg_src as usize);
    let dividend = state.d_long(reg_dst as usize);
    execute_divu(state, dividend, divisor, reg_dst);
}

pub fn alu_divu_mem(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let divisor = state.micro.source as u16;
    let dividend = state.d_long(reg_dst as usize);
    execute_divu(state, dividend, divisor, reg_dst);
}

fn execute_divu(state: &mut CpuState, dividend: u32, divisor: u16, reg_dst: u8) {
    if divisor == 0 {
        common::trigger_divide_by_zero(state);
        return;
    }

    if (dividend >> 16) >= (divisor as u32) {
        state.set_ccr_v_clear_c();
        state.micro.clocks_remaining = 6;
        return;
    }

    let quotient = dividend / (divisor as u32);
    let remainder = dividend % (divisor as u32);
    let result = ((remainder & 0xFFFF) << 16) | (quotient & 0xFFFF);
    state.set_d_long(reg_dst as usize, result);

    let q16 = quotient as u16;
    state.set_ccr_nz_clear_vc((q16 as i16) < 0, q16 == 0);
    state.micro.clocks_remaining = calc_divu_internal_clocks(dividend, divisor);
}

// ============================================================================
// Static Micro-Step Slices: DIVU
// ============================================================================

pub static STEPS_DIVU_DN: [MicroStep; 3] = [
    MicroStep::alu(alu_divu_dn),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVU_AI: [MicroStep; 5] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    MicroStep::alu(alu_divu_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVU_PI: [MicroStep; 5] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    MicroStep::alu(alu_divu_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVU_PD: [MicroStep; 6] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divu_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVU_D16_AN: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divu_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVU_IDX_AN: [MicroStep; 8] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divu_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVU_ABSW: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divu_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVU_ABSL: [MicroStep; 9] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divu_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVU_D16_PC: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divu_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVU_IDX_PC: [MicroStep; 8] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divu_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVU_IMM: [MicroStep; 5] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep::alu(alu_divu_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

// ============================================================================
// Static Lookup Tables & Decode Functions
// ============================================================================

static DIVU_LOOKUP: [&[MicroStep]; 11] = [
    &STEPS_DIVU_DN,
    &STEPS_DIVU_AI,
    &STEPS_DIVU_PI,
    &STEPS_DIVU_PD,
    &STEPS_DIVU_D16_AN,
    &STEPS_DIVU_IDX_AN,
    &STEPS_DIVU_ABSW,
    &STEPS_DIVU_ABSL,
    &STEPS_DIVU_D16_PC,
    &STEPS_DIVU_IDX_PC,
    &STEPS_DIVU_IMM,
];

pub const fn decode_divu_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(DIVU_LOOKUP[0]),
        2 => Some(DIVU_LOOKUP[1]),
        3 => Some(DIVU_LOOKUP[2]),
        4 => Some(DIVU_LOOKUP[3]),
        5 => Some(DIVU_LOOKUP[4]),
        6 => Some(DIVU_LOOKUP[5]),
        7 => match reg {
            0 => Some(DIVU_LOOKUP[6]),
            1 => Some(DIVU_LOOKUP[7]),
            2 => Some(DIVU_LOOKUP[8]),
            3 => Some(DIVU_LOOKUP[9]),
            4 => Some(DIVU_LOOKUP[10]),
            _ => None,
        },
        _ => None,
    }
}
