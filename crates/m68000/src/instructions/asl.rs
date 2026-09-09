//! ASL (Arithmetic Shift Left) instruction handlers and CCR updates
//!
//! Shifts bits to the left, shifting 0 into LSB and the MSB into C and X flags.
//! Detects arithmetic overflow (V flag set if most significant bit changes at any time).

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Leaf ALU ASL Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn asl_b(state: &mut CpuState, count: u32, val: u8) -> u8 {
    let msb = (val & 0x80) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let mut v = val;
    let mut last_out = false;
    let mut overflow = false;
    for _ in 0..count {
        let old_msb = (v & 0x80) != 0;
        last_out = old_msb;
        v <<= 1;
        let new_msb = (v & 0x80) != 0;
        if old_msb != new_msb {
            overflow = true;
        }
    }
    let new_msb = (v & 0x80) != 0;
    state.set_ccr_xnzvc(last_out, new_msb, v == 0, overflow, last_out);
    v
}

#[inline(always)]
pub fn asl_w(state: &mut CpuState, count: u32, val: u16) -> u16 {
    let msb = (val & 0x8000) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let mut v = val;
    let mut last_out = false;
    let mut overflow = false;
    for _ in 0..count {
        let old_msb = (v & 0x8000) != 0;
        last_out = old_msb;
        v <<= 1;
        let new_msb = (v & 0x8000) != 0;
        if old_msb != new_msb {
            overflow = true;
        }
    }
    let new_msb = (v & 0x8000) != 0;
    state.set_ccr_xnzvc(last_out, new_msb, v == 0, overflow, last_out);
    v
}

#[inline(always)]
pub fn asl_l(state: &mut CpuState, count: u32, val: u32) -> u32 {
    let msb = (val & 0x8000_0000) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let mut v = val;
    let mut last_out = false;
    let mut overflow = false;
    for _ in 0..count {
        let old_msb = (v & 0x8000_0000) != 0;
        last_out = old_msb;
        v <<= 1;
        let new_msb = (v & 0x8000_0000) != 0;
        if old_msb != new_msb {
            overflow = true;
        }
    }
    let new_msb = (v & 0x8000_0000) != 0;
    state.set_ccr_xnzvc(last_out, new_msb, v == 0, overflow, last_out);
    v
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_asl_b_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_byte(reg_dst as usize);
    let res = asl_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    let duration = 2 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_asl_w_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_word(reg_dst as usize);
    let res = asl_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    let duration = 2 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_asl_l_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_long(reg_dst as usize);
    let res = asl_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    let duration = 4 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_asl_b_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_byte(reg_dst as usize);
    let res = asl_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    let duration = 2 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_asl_w_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_word(reg_dst as usize);
    let res = asl_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    let duration = 2 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_asl_l_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_long(reg_dst as usize);
    let res = asl_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    let duration = 4 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_asl_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = (state.micro.destination & 0xFFFF) as u16;
    let res = asl_w(state, 1, d);
    state.micro.destination = (state.micro.destination & !0xFFFF) | (res as u32);
}

// ============================================================================
// Static Micro-Step Slices: ASL Register
// ============================================================================

pub static STEPS_ASL_B_IMM: [MicroStep; 3] = [
    MicroStep::alu(alu_asl_b_imm_dn),
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ASL_W_IMM: [MicroStep; 3] = [
    MicroStep::alu(alu_asl_w_imm_dn),
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ASL_L_IMM: [MicroStep; 3] = [
    MicroStep::alu(alu_asl_l_imm_dn),
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_ASL_B_REG: [MicroStep; 3] = [
    MicroStep::alu(alu_asl_b_reg_dn),
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ASL_W_REG: [MicroStep; 3] = [
    MicroStep::alu(alu_asl_w_reg_dn),
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ASL_L_REG: [MicroStep; 3] = [
    MicroStep::alu(alu_asl_l_reg_dn),
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];

// ============================================================================
// Static Micro-Step Slices: ASL Memory (Word only, Count = 1)
// ============================================================================

pub static STEPS_ASL_W_AI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_irc_read),
        alu_fn: Some(alu_asl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];
pub static STEPS_ASL_W_PI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pi_w),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_irc_read),
        alu_fn: Some(alu_asl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];
pub static STEPS_ASL_W_PD: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_irc_read),
        alu_fn: Some(alu_asl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];
pub static STEPS_ASL_W_D16_AN: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_irc_read),
        alu_fn: Some(alu_asl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];
pub static STEPS_ASL_W_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_irc_read),
        alu_fn: Some(alu_asl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];
pub static STEPS_ASL_W_ABSW: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_irc_read),
        alu_fn: Some(alu_asl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];
pub static STEPS_ASL_W_ABSL: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_irc_read),
        alu_fn: Some(alu_asl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];

// ============================================================================
// Static Decoder Functions
// ============================================================================

/// Decodes the micro-step sequence for ASL register shift
pub const fn decode_asl_reg_steps(is_reg_count: bool, size: u8) -> Option<&'static [MicroStep]> {
    if is_reg_count {
        match size {
            0 => Some(&STEPS_ASL_B_REG),
            1 => Some(&STEPS_ASL_W_REG),
            2 => Some(&STEPS_ASL_L_REG),
            _ => None,
        }
    } else {
        match size {
            0 => Some(&STEPS_ASL_B_IMM),
            1 => Some(&STEPS_ASL_W_IMM),
            2 => Some(&STEPS_ASL_L_IMM),
            _ => None,
        }
    }
}

/// Decodes the micro-step sequence for ASL memory shift (Word only, Count = 1)
pub const fn decode_asl_mem_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        2 => Some(&STEPS_ASL_W_AI),
        3 => Some(&STEPS_ASL_W_PI),
        4 => Some(&STEPS_ASL_W_PD),
        5 => Some(&STEPS_ASL_W_D16_AN),
        6 => Some(&STEPS_ASL_W_IDX_AN),
        7 => match reg {
            0 => Some(&STEPS_ASL_W_ABSW),
            1 => Some(&STEPS_ASL_W_ABSL),
            _ => None,
        },
        _ => None,
    }
}
