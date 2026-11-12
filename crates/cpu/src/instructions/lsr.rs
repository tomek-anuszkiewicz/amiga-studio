//! LSR (Logical Shift Right) instruction handlers and CCR updates
//!
//! Shifts bits to the right, shifting 0 into MSB and the LSB into C and X flags.
//! Overflow (V) flag is always cleared to 0.

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
// Leaf ALU LSR Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn lsr_b(state: &mut CpuState, count: u32, val: u8) -> u8 {
    let msb = (val & BIT_BYTE_SIGN) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let mut v = val;
    let mut last_out = false;
    for _ in 0..count {
        last_out = (v & 1) != 0;
        v >>= 1;
    }
    let new_msb = (v & BIT_BYTE_SIGN) != 0;
    state.set_ccr_xnzvc(last_out, new_msb, v == 0, false, last_out);
    v
}

#[inline(always)]
pub fn lsr_w(state: &mut CpuState, count: u32, val: u16) -> u16 {
    let msb = (val & BIT_WORD_SIGN) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let mut v = val;
    let mut last_out = false;
    for _ in 0..count {
        last_out = (v & 1) != 0;
        v >>= 1;
    }
    let new_msb = (v & BIT_WORD_SIGN) != 0;
    state.set_ccr_xnzvc(last_out, new_msb, v == 0, false, last_out);
    v
}

#[inline(always)]
pub fn lsr_l(state: &mut CpuState, count: u32, val: u32) -> u32 {
    let msb = (val & BIT_LONG_SIGN) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let mut v = val;
    let mut last_out = false;
    for _ in 0..count {
        last_out = (v & 1) != 0;
        v >>= 1;
    }
    let new_msb = (v & BIT_LONG_SIGN) != 0;
    state.set_ccr_xnzvc(last_out, new_msb, v == 0, false, last_out);
    v
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_lsr_b_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_byte(reg_dst as usize);
    let res = lsr_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    let duration = SHIFT_BYTE_WORD_BASE_CLOCKS + (SHIFT_CLOCKS_PER_BIT * count);
    state.micro_mut().clocks_remaining = duration as u16;
}

pub fn alu_lsr_w_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_word(reg_dst as usize);
    let res = lsr_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    let duration = SHIFT_BYTE_WORD_BASE_CLOCKS + (SHIFT_CLOCKS_PER_BIT * count);
    state.micro_mut().clocks_remaining = duration as u16;
}

pub fn alu_lsr_l_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_long(reg_dst as usize);
    let res = lsr_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    let duration = SHIFT_LONG_BASE_CLOCKS + (SHIFT_CLOCKS_PER_BIT * count);
    state.micro_mut().clocks_remaining = duration as u16;
}

pub fn alu_lsr_b_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & crate::micro::encoding::shift::MASK_REGISTER_COUNT;
    let val = state.d_byte(reg_dst as usize);
    let res = lsr_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    let duration = SHIFT_BYTE_WORD_BASE_CLOCKS + (SHIFT_CLOCKS_PER_BIT * count);
    state.micro_mut().clocks_remaining = duration as u16;
}

pub fn alu_lsr_w_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & crate::micro::encoding::shift::MASK_REGISTER_COUNT;
    let val = state.d_word(reg_dst as usize);
    let res = lsr_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    let duration = SHIFT_BYTE_WORD_BASE_CLOCKS + (SHIFT_CLOCKS_PER_BIT * count);
    state.micro_mut().clocks_remaining = duration as u16;
}

pub fn alu_lsr_l_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & crate::micro::encoding::shift::MASK_REGISTER_COUNT;
    let val = state.d_long(reg_dst as usize);
    let res = lsr_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    let duration = SHIFT_LONG_BASE_CLOCKS + (SHIFT_CLOCKS_PER_BIT * count);
    state.micro_mut().clocks_remaining = duration as u16;
}

pub fn alu_lsr_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = (state.micro().destination & 0xFFFF) as u16;
    let res = lsr_w(state, 1, d);
    state.micro_mut().destination = (state.micro().destination & !0xFFFF) | (res as u32);
}

// ============================================================================
// Static Micro-Step Slices: LSR Register
// ============================================================================

pub static STEPS_LSR_B_IMM: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_lsr_b_imm_dn),
];
pub static STEPS_LSR_W_IMM: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_lsr_w_imm_dn),
];
pub static STEPS_LSR_L_IMM: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_lsr_l_imm_dn),
];

pub static STEPS_LSR_B_REG: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_lsr_b_reg_dn),
];
pub static STEPS_LSR_W_REG: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_lsr_w_reg_dn),
];
pub static STEPS_LSR_L_REG: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_lsr_l_reg_dn),
];

// ============================================================================
// Static Micro-Step Slices: LSR Memory (Word only, Count = 1)
// ============================================================================

pub static STEPS_LSR_W_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_lsr_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_LSR_W_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pi_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_lsr_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_LSR_W_PD: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_lsr_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_LSR_W_D16_AN: [MicroStep; 8] = [
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
        alu_fn: Some(alu_lsr_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_LSR_W_IDX_AN: [MicroStep; 9] = [
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
        alu_fn: Some(alu_lsr_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_LSR_W_ABSW: [MicroStep; 8] = [
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
        alu_fn: Some(alu_lsr_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_LSR_W_ABSL: [MicroStep; 10] = [
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
        alu_fn: Some(alu_lsr_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

// ============================================================================
// Static Decoder Functions
// ============================================================================

/// Decodes the micro-step sequence for LSR register shift
pub const fn decode_lsr_reg_steps(is_reg_count: bool, size: u8) -> Option<&'static [MicroStep]> {
    if is_reg_count {
        match size {
            operand_size::BYTE => Some(&STEPS_LSR_B_REG),
            operand_size::WORD => Some(&STEPS_LSR_W_REG),
            operand_size::LONG => Some(&STEPS_LSR_L_REG),
            _ => None,
        }
    } else {
        match size {
            operand_size::BYTE => Some(&STEPS_LSR_B_IMM),
            operand_size::WORD => Some(&STEPS_LSR_W_IMM),
            operand_size::LONG => Some(&STEPS_LSR_L_IMM),
            _ => None,
        }
    }
}

/// Decodes the micro-step sequence for LSR memory shift (Word only, Count = 1)
pub const fn decode_lsr_mem_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        mode::INDIRECT => Some(&STEPS_LSR_W_AI),
        mode::POSTINCREMENT => Some(&STEPS_LSR_W_PI),
        mode::PREDECREMENT => Some(&STEPS_LSR_W_PD),
        mode::DISPLACEMENT => Some(&STEPS_LSR_W_D16_AN),
        mode::INDEXED => Some(&STEPS_LSR_W_IDX_AN),
        mode::EXTENDED => match reg {
            extended_mode::ABSOLUTE_WORD => Some(&STEPS_LSR_W_ABSW),
            extended_mode::ABSOLUTE_LONG => Some(&STEPS_LSR_W_ABSL),
            _ => None,
        },
        _ => None,
    }
}
