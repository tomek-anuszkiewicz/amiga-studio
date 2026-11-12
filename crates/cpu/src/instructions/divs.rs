//! DIVS (16-bit Signed Divide) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions for:
//! - DIVS <ea>, Dn (Signed divide: 32 / 16 -> 16q:16r)
//!
//! Features:
//! - Divide-by-zero Trap Vector 5 exception processing (38 clocks total).
//! - Overflow detection (V=1, C=0, destination unchanged, 16-18 clocks).
//! - Data-dependent cycle timing for normal non-overflow divisions.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::encoding::{extended_mode, mode};
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

const NEGATIVE_DIVIDEND_MACHINE_CYCLES: u16 = 7;
const POSITIVE_DIVIDEND_MACHINE_CYCLES: u16 = 6;
const QUOTIENT_MACHINE_CYCLES: u16 = 55;
const OVERFLOW_MACHINE_CYCLES: u16 = 2;

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
pub fn calc_divs_internal_clocks(dividend: i32, divisor: i16) -> u16 {
    let mut mcycles = if dividend < 0 {
        NEGATIVE_DIVIDEND_MACHINE_CYCLES
    } else {
        POSITIVE_DIVIDEND_MACHINE_CYCLES
    };
    let abs_dividend = (dividend as i64).unsigned_abs() as u32;
    let abs_divisor = (divisor as i32).unsigned_abs();

    mcycles += QUOTIENT_MACHINE_CYCLES;
    if divisor >= 0 {
        if dividend < 0 {
            mcycles += 1;
        } else {
            mcycles -= 1;
        }
    }

    let mut aquot = abs_dividend / abs_divisor;
    for _ in 0..super::DIVISION_TIMING_ITERATIONS {
        if (aquot as i16) >= 0 {
            mcycles += 1;
        }
        aquot = (aquot << 1) & 0xFFFF;
    }

    (super::DIVISION_MACHINE_CYCLE_CLOCKS * mcycles).saturating_sub(super::DIVISION_PREFETCH_CLOCKS)
}

// ============================================================================
// Pure ALU Callbacks: DIVS
// ============================================================================

pub fn alu_divs_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let divisor = state.d_word(reg_src as usize) as i16;
    let dividend = state.d_long(reg_dst as usize) as i32;
    execute_divs(state, dividend, divisor, reg_dst);
}

pub fn alu_divs_mem(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let divisor = state.micro().source as i16;
    let dividend = state.d_long(reg_dst as usize) as i32;
    execute_divs(state, dividend, divisor, reg_dst);
}

fn execute_divs(state: &mut CpuState, dividend: i32, divisor: i16, reg_dst: u8) {
    if divisor == 0 {
        common::trigger_divide_by_zero(state);
        return;
    }

    let d64 = dividend as i64;
    let div64 = divisor as i64;
    let quotient = d64 / div64;
    let remainder = d64 % div64;

    let overflow = quotient < i64::from(i16::MIN) || quotient > i64::from(i16::MAX);
    if overflow {
        state.set_ccr_v_clear_c();
        let mcycles = if dividend < 0 {
            NEGATIVE_DIVIDEND_MACHINE_CYCLES
        } else {
            POSITIVE_DIVIDEND_MACHINE_CYCLES
        };
        state.micro_mut().clocks_remaining = ((mcycles + OVERFLOW_MACHINE_CYCLES)
            * super::DIVISION_MACHINE_CYCLE_CLOCKS)
            .saturating_sub(super::DIVISION_PREFETCH_CLOCKS);
        return;
    }

    let q16 = (quotient as i16) as u16;
    let r16 = (remainder as i16) as u16;
    let result = ((r16 as u32) << 16) | (q16 as u32);
    state.set_d_long(reg_dst as usize, result);

    state.set_ccr_nz_clear_vc((q16 as i16) < 0, q16 == 0);
    state.micro_mut().clocks_remaining = calc_divs_internal_clocks(dividend, divisor);
}

// ============================================================================
// Static Micro-Step Slices: DIVS
// ============================================================================

pub static STEPS_DIVS_DN: [MicroStep; 3] = [
    MicroStep::alu(alu_divs_dn),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_AI: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    BUS_READ_IDLE,
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_PI: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    BUS_READ_IDLE,
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_PD: [MicroStep; 6] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_D16_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_IDX_AN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_ABSW: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_ABSL: [MicroStep; 9] = [
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
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_D16_PC: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_IDX_PC: [MicroStep; 8] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_IMM: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    FETCH_EXT_FINISH,
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

// ============================================================================
// Static Lookup Tables & Decode Functions
// ============================================================================

static DIVS_LOOKUP: [&[MicroStep]; 11] = [
    &STEPS_DIVS_DN,
    &STEPS_DIVS_AI,
    &STEPS_DIVS_PI,
    &STEPS_DIVS_PD,
    &STEPS_DIVS_D16_AN,
    &STEPS_DIVS_IDX_AN,
    &STEPS_DIVS_ABSW,
    &STEPS_DIVS_ABSL,
    &STEPS_DIVS_D16_PC,
    &STEPS_DIVS_IDX_PC,
    &STEPS_DIVS_IMM,
];

pub const fn decode_divs_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        mode::DATA_REGISTER => Some(DIVS_LOOKUP[0]),
        mode::INDIRECT => Some(DIVS_LOOKUP[1]),
        mode::POSTINCREMENT => Some(DIVS_LOOKUP[2]),
        mode::PREDECREMENT => Some(DIVS_LOOKUP[3]),
        mode::DISPLACEMENT => Some(DIVS_LOOKUP[4]),
        mode::INDEXED => Some(DIVS_LOOKUP[5]),
        mode::EXTENDED => match reg {
            extended_mode::ABSOLUTE_WORD => Some(DIVS_LOOKUP[6]),
            extended_mode::ABSOLUTE_LONG => Some(DIVS_LOOKUP[7]),
            extended_mode::PC_DISPLACEMENT => Some(DIVS_LOOKUP[8]),
            extended_mode::PC_INDEXED => Some(DIVS_LOOKUP[9]),
            extended_mode::IMMEDIATE => Some(DIVS_LOOKUP[10]),
            _ => None,
        },
        _ => None,
    }
}
