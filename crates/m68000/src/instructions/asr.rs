//! ASR (Arithmetic Shift Right) instruction handlers and CCR updates
//!
//! Shifts bits to the right, replicating the MSB (sign bit) and shifting the LSB into C and X flags.
//! Overflow (V) flag is always cleared to 0.

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Leaf ALU ASR Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn asr_b(state: &mut CpuState, count: u32, val: u8) -> u8 {
    let msb = (val & 0x80) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let mut v = val as i8;
    let mut last_out = false;
    for _ in 0..count {
        last_out = (v & 1) != 0;
        v >>= 1;
    }
    if count > 8 {
        last_out = false;
    }
    let res = v as u8;
    state.set_ccr_xnzvc(last_out, (res & 0x80) != 0, res == 0, false, last_out);
    res
}

#[inline(always)]
pub fn asr_w(state: &mut CpuState, count: u32, val: u16) -> u16 {
    let msb = (val & 0x8000) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let mut v = val as i16;
    let mut last_out = false;
    for _ in 0..count {
        last_out = (v & 1) != 0;
        v >>= 1;
    }
    if count > 16 {
        last_out = false;
    }
    let res = v as u16;
    state.set_ccr_xnzvc(last_out, (res & 0x8000) != 0, res == 0, false, last_out);
    res
}

#[inline(always)]
pub fn asr_l(state: &mut CpuState, count: u32, val: u32) -> u32 {
    let msb = (val & 0x8000_0000) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let mut v = val as i32;
    let mut last_out = false;
    for _ in 0..count {
        last_out = (v & 1) != 0;
        v >>= 1;
    }
    if count > 32 {
        last_out = false;
    }
    let res = v as u32;
    state.set_ccr_xnzvc(
        last_out,
        (res & 0x8000_0000) != 0,
        res == 0,
        false,
        last_out,
    );
    res
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_asr_b_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_byte(reg_dst as usize);
    let res = asr_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    let duration = 2 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_asr_w_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_word(reg_dst as usize);
    let res = asr_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    let duration = 2 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_asr_l_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_long(reg_dst as usize);
    let res = asr_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    let duration = 4 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_asr_b_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_byte(reg_dst as usize);
    let res = asr_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    let duration = 2 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_asr_w_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_word(reg_dst as usize);
    let res = asr_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    let duration = 2 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_asr_l_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_long(reg_dst as usize);
    let res = asr_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    let duration = 4 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_asr_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = (state.micro.destination & 0xFFFF) as u16;
    let res = asr_w(state, 1, d);
    state.micro.destination = (state.micro.destination & !0xFFFF) | (res as u32);
}

// ============================================================================
// Static Micro-Step Slices: ASR Register
// ============================================================================

pub static STEPS_ASR_B_IMM: [MicroStep; 3] = [
    MicroStep::alu(alu_asr_b_imm_dn),
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];
pub static STEPS_ASR_W_IMM: [MicroStep; 3] = [
    MicroStep::alu(alu_asr_w_imm_dn),
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];
pub static STEPS_ASR_L_IMM: [MicroStep; 3] = [
    MicroStep::alu(alu_asr_l_imm_dn),
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_ASR_B_REG: [MicroStep; 3] = [
    MicroStep::alu(alu_asr_b_reg_dn),
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];
pub static STEPS_ASR_W_REG: [MicroStep; 3] = [
    MicroStep::alu(alu_asr_w_reg_dn),
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];
pub static STEPS_ASR_L_REG: [MicroStep; 3] = [
    MicroStep::alu(alu_asr_l_reg_dn),
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// Static Micro-Step Slices: ASR Memory (Word only, Count = 1)
// ============================================================================

pub static STEPS_ASR_W_AI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_asr_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ASR_W_PI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_asr_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ASR_W_PD: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_asr_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ASR_W_D16_AN: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_asr_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ASR_W_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_asr_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ASR_W_ABSW: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_asr_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ASR_W_ABSL: [MicroStep; 10] = [
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
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_asr_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

// ============================================================================
// Static Decoder Functions
// ============================================================================

/// Decodes the micro-step sequence for ASR register shift
pub const fn decode_asr_reg_steps(is_reg_count: bool, size: u8) -> Option<&'static [MicroStep]> {
    if is_reg_count {
        match size {
            0 => Some(&STEPS_ASR_B_REG),
            1 => Some(&STEPS_ASR_W_REG),
            2 => Some(&STEPS_ASR_L_REG),
            _ => None,
        }
    } else {
        match size {
            0 => Some(&STEPS_ASR_B_IMM),
            1 => Some(&STEPS_ASR_W_IMM),
            2 => Some(&STEPS_ASR_L_IMM),
            _ => None,
        }
    }
}

/// Decodes the micro-step sequence for ASR memory shift (Word only, Count = 1)
pub const fn decode_asr_mem_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        2 => Some(&STEPS_ASR_W_AI),
        3 => Some(&STEPS_ASR_W_PI),
        4 => Some(&STEPS_ASR_W_PD),
        5 => Some(&STEPS_ASR_W_D16_AN),
        6 => Some(&STEPS_ASR_W_IDX_AN),
        7 => match reg {
            0 => Some(&STEPS_ASR_W_ABSW),
            1 => Some(&STEPS_ASR_W_ABSL),
            _ => None,
        },
        _ => None,
    }
}
