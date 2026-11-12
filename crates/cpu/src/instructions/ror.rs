//! ROR (Rotate Right) instruction handlers and CCR updates
//!
//! Rotates bits to the right, rotating LSB into MSB and into the C flag.
//! The Extend (X) flag is unaffected. Overflow (V) flag is always cleared to 0.

use super::{
    BIT_BYTE_SIGN, BIT_LONG_SIGN, BIT_WORD_SIGN, SHIFT_BYTE_WORD_BASE_CLOCKS, SHIFT_CLOCKS_PER_BIT,
    SHIFT_LONG_BASE_CLOCKS,
};
use crate::micro::common;
use crate::micro::ea;
use crate::micro::encoding::{extended_mode, mode, operand_size};
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Leaf ALU ROR Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn ror_b(state: &mut CpuState, count: u32, val: u8) -> u8 {
    let msb = (val & BIT_BYTE_SIGN) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let mut v = val;
    let mut last_out = false;
    for _ in 0..count {
        last_out = (v & 1) != 0;
        v = (v >> 1) | if last_out { BIT_BYTE_SIGN } else { 0 };
    }
    let new_msb = (v & BIT_BYTE_SIGN) != 0;
    state.set_ccr_nzvc(new_msb, v == 0, false, last_out);
    v
}

#[inline(always)]
pub fn ror_w(state: &mut CpuState, count: u32, val: u16) -> u16 {
    let msb = (val & BIT_WORD_SIGN) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let mut v = val;
    let mut last_out = false;
    for _ in 0..count {
        last_out = (v & 1) != 0;
        v = (v >> 1) | if last_out { BIT_WORD_SIGN } else { 0 };
    }
    let new_msb = (v & BIT_WORD_SIGN) != 0;
    state.set_ccr_nzvc(new_msb, v == 0, false, last_out);
    v
}

#[inline(always)]
pub fn ror_l(state: &mut CpuState, count: u32, val: u32) -> u32 {
    let msb = (val & BIT_LONG_SIGN) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let mut v = val;
    let mut last_out = false;
    for _ in 0..count {
        last_out = (v & 1) != 0;
        v = (v >> 1) | if last_out { BIT_LONG_SIGN } else { 0 };
    }
    let new_msb = (v & BIT_LONG_SIGN) != 0;
    state.set_ccr_nzvc(new_msb, v == 0, false, last_out);
    v
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_ror_b_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_byte(reg_dst as usize);
    let res = ror_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    let duration = SHIFT_BYTE_WORD_BASE_CLOCKS + (SHIFT_CLOCKS_PER_BIT * count);
    state.micro_mut().clocks_remaining = duration as u16;
}

pub fn alu_ror_w_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_word(reg_dst as usize);
    let res = ror_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    let duration = SHIFT_BYTE_WORD_BASE_CLOCKS + (SHIFT_CLOCKS_PER_BIT * count);
    state.micro_mut().clocks_remaining = duration as u16;
}

pub fn alu_ror_l_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_long(reg_dst as usize);
    let res = ror_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    let duration = SHIFT_LONG_BASE_CLOCKS + (SHIFT_CLOCKS_PER_BIT * count);
    state.micro_mut().clocks_remaining = duration as u16;
}

pub fn alu_ror_b_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & crate::micro::encoding::shift::MASK_REGISTER_COUNT;
    let val = state.d_byte(reg_dst as usize);
    let res = ror_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    let duration = SHIFT_BYTE_WORD_BASE_CLOCKS + (SHIFT_CLOCKS_PER_BIT * count);
    state.micro_mut().clocks_remaining = duration as u16;
}

pub fn alu_ror_w_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & crate::micro::encoding::shift::MASK_REGISTER_COUNT;
    let val = state.d_word(reg_dst as usize);
    let res = ror_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    let duration = SHIFT_BYTE_WORD_BASE_CLOCKS + (SHIFT_CLOCKS_PER_BIT * count);
    state.micro_mut().clocks_remaining = duration as u16;
}

pub fn alu_ror_l_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & crate::micro::encoding::shift::MASK_REGISTER_COUNT;
    let val = state.d_long(reg_dst as usize);
    let res = ror_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    let duration = SHIFT_LONG_BASE_CLOCKS + (SHIFT_CLOCKS_PER_BIT * count);
    state.micro_mut().clocks_remaining = duration as u16;
}

pub fn alu_ror_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = (state.micro().destination & 0xFFFF) as u16;
    let res = ror_w(state, 1, d);
    state.micro_mut().destination = (state.micro().destination & !0xFFFF) | (res as u32);
}

// ============================================================================
// Static Micro-Step Slices: ROR Register
// ============================================================================

pub static STEPS_ROR_B_IMM: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_ror_b_imm_dn),
];
pub static STEPS_ROR_W_IMM: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_ror_w_imm_dn),
];
pub static STEPS_ROR_L_IMM: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_ror_l_imm_dn),
];

pub static STEPS_ROR_B_REG: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_ror_b_reg_dn),
];
pub static STEPS_ROR_W_REG: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_ror_w_reg_dn),
];
pub static STEPS_ROR_L_REG: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_ror_l_reg_dn),
];

// ============================================================================
// Static Micro-Step Slices: ROR Memory (Word only, Count = 1)
// ============================================================================

pub static STEPS_ROR_W_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ror_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ROR_W_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pi_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ror_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ROR_W_PD: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ror_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ROR_W_D16_AN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ror_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ROR_W_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ror_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ROR_W_ABSW: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ror_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ROR_W_ABSL: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ror_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

// ============================================================================
// Static Decoder Functions
// ============================================================================

/// Decodes the micro-step sequence for ROR register shift
pub const fn decode_ror_reg_steps(is_reg_count: bool, size: u8) -> Option<&'static [MicroStep]> {
    if is_reg_count {
        match size {
            operand_size::BYTE => Some(&STEPS_ROR_B_REG),
            operand_size::WORD => Some(&STEPS_ROR_W_REG),
            operand_size::LONG => Some(&STEPS_ROR_L_REG),
            _ => None,
        }
    } else {
        match size {
            operand_size::BYTE => Some(&STEPS_ROR_B_IMM),
            operand_size::WORD => Some(&STEPS_ROR_W_IMM),
            operand_size::LONG => Some(&STEPS_ROR_L_IMM),
            _ => None,
        }
    }
}

/// Decodes the micro-step sequence for ROR memory shift (Word only, Count = 1)
pub const fn decode_ror_mem_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        mode::INDIRECT => Some(&STEPS_ROR_W_AI),
        mode::POSTINCREMENT => Some(&STEPS_ROR_W_PI),
        mode::PREDECREMENT => Some(&STEPS_ROR_W_PD),
        mode::DISPLACEMENT => Some(&STEPS_ROR_W_D16_AN),
        mode::INDEXED => Some(&STEPS_ROR_W_IDX_AN),
        mode::EXTENDED => match reg {
            extended_mode::ABSOLUTE_WORD => Some(&STEPS_ROR_W_ABSW),
            extended_mode::ABSOLUTE_LONG => Some(&STEPS_ROR_W_ABSL),
            _ => None,
        },
        _ => None,
    }
}
