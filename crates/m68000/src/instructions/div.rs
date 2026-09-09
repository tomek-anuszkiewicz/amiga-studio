//! DIVU / DIVS (16-bit Divide) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions for:
//! - DIVU <ea>, Dn (Unsigned divide: 32 / 16 -> 16q:16r)
//! - DIVS <ea>, Dn (Signed divide: 32 / 16 -> 16q:16r)
//!
//! Features:
//! - Divide-by-zero Trap Vector 5 exception processing (38 clocks total).
//! - Overflow detection (V=1, C=0, destination unchanged, 10 or 16-18 clocks).
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
// Divide-by-Zero Exception Pipeline (Vector 5, 38 CPU clocks total)
// ============================================================================

pub static STEPS_DIV_ZERO: [MicroStep; 16] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 8,
    },
    common::EXCEPTION_PUSH_PCLO_IDLE,
    common::EXCEPTION_PUSH_PCLO_WRITE,
    common::EXCEPTION_PUSH_SR_IDLE,
    common::EXCEPTION_PUSH_SR_WRITE,
    common::EXCEPTION_PUSH_PCHI_IDLE,
    common::EXCEPTION_PUSH_PCHI_WRITE,
    common::READ_VECTOR_HIGH_READ,
    common::BUS_READ_IDLE,
    common::READ_VECTOR_LOW_READ,
    common::READ_VECTOR_LOW_FINISH,
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

pub fn trigger_divide_by_zero(state: &mut CpuState) {
    let old_sr = state.sr;
    let updated_sr = old_sr & !0x000F;
    state.sr = updated_sr;
    state.set_supervisor(true);
    state.sr &= !0x8000;

    state.micro.source = state.instruction_pc;
    state.micro.destination = updated_sr as u32;
    state.micro.ea_addr = 0x0000_0014; // Vector 5 (address 20)
    state.micro.current_steps = &STEPS_DIV_ZERO;
    state.micro.micro_step = 0;
    state.micro.clocks_remaining = 0;
}

// ============================================================================
// Cycle Calculation Helpers
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

#[inline(always)]
pub fn calc_divs_internal_clocks(dividend: i32, divisor: i16) -> u16 {
    let mut mcycles = if dividend < 0 { 7_u16 } else { 6_u16 };
    let abs_dividend = (dividend as i64).unsigned_abs() as u32;
    let abs_divisor = (divisor as i32).unsigned_abs();

    mcycles += 55;
    if divisor >= 0 {
        if dividend < 0 {
            mcycles += 1;
        } else {
            mcycles -= 1;
        }
    }

    let mut aquot = abs_dividend / abs_divisor;
    for _ in 0..15 {
        if (aquot as i16) >= 0 {
            mcycles += 1;
        }
        aquot = (aquot << 1) & 0xFFFF;
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
        trigger_divide_by_zero(state);
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
// Pure ALU Callbacks: DIVS
// ============================================================================

pub fn alu_divs_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let divisor = state.d_word(reg_src as usize) as i16;
    let dividend = state.d_long(reg_dst as usize) as i32;
    execute_divs(state, dividend, divisor, reg_dst);
}

pub fn alu_divs_mem(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let divisor = state.micro.source as i16;
    let dividend = state.d_long(reg_dst as usize) as i32;
    execute_divs(state, dividend, divisor, reg_dst);
}

fn execute_divs(state: &mut CpuState, dividend: i32, divisor: i16, reg_dst: u8) {
    if divisor == 0 {
        trigger_divide_by_zero(state);
        return;
    }

    let d64 = dividend as i64;
    let div64 = divisor as i64;
    let quotient = d64 / div64;
    let remainder = d64 % div64;

    let overflow = quotient < -32768 || quotient > 32767;
    if overflow {
        state.set_ccr_v_clear_c();
        let mcycles = if dividend < 0 { 7_u16 } else { 6_u16 };
        state.micro.clocks_remaining = ((mcycles + 2) * 2).saturating_sub(4);
        return;
    }

    let q16 = (quotient as i16) as u16;
    let r16 = (remainder as i16) as u16;
    let result = ((r16 as u32) << 16) | (q16 as u32);
    state.set_d_long(reg_dst as usize, result);

    state.set_ccr_nz_clear_vc((q16 as i16) < 0, q16 == 0);
    state.micro.clocks_remaining = calc_divs_internal_clocks(dividend, divisor);
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
// Static Micro-Step Slices: DIVS
// ============================================================================

pub static STEPS_DIVS_DN: [MicroStep; 3] = [
    MicroStep::alu(alu_divs_dn),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_AI: [MicroStep; 5] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_PI: [MicroStep; 5] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_PD: [MicroStep; 6] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_D16_AN: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
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
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
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
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
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
    MicroStep::alu(alu_divs_mem),
    PREFETCH_NEXT_READ,
    BUS_READ_IDLE,
];

pub static STEPS_DIVS_D16_PC: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
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
        step_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
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
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep::alu(alu_divs_mem),
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

pub const fn decode_divs_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(DIVS_LOOKUP[0]),
        2 => Some(DIVS_LOOKUP[1]),
        3 => Some(DIVS_LOOKUP[2]),
        4 => Some(DIVS_LOOKUP[3]),
        5 => Some(DIVS_LOOKUP[4]),
        6 => Some(DIVS_LOOKUP[5]),
        7 => match reg {
            0 => Some(DIVS_LOOKUP[6]),
            1 => Some(DIVS_LOOKUP[7]),
            2 => Some(DIVS_LOOKUP[8]),
            3 => Some(DIVS_LOOKUP[9]),
            4 => Some(DIVS_LOOKUP[10]),
            _ => None,
        },
        _ => None,
    }
}
