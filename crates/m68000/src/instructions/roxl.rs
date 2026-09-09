//! ROXL (Rotate Left with Extend) instruction handlers and CCR updates
//!
//! Rotates bits to the left through the Extend (X) flag.
//! If count is 0, C is set to the X flag and X is unaffected.
//! Overflow (V) flag is always cleared to 0.

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Leaf ALU ROXL Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn roxl_b(state: &mut CpuState, count: u32, val: u8) -> u8 {
    let msb = (val & 0x80) != 0;
    if count == 0 {
        let x = state.get_x();
        state.set_ccr_nzvc(msb, val == 0, false, x);
        return val;
    }
    let mut v = val;
    let mut x = state.get_x();
    for _ in 0..count {
        let old_msb = (v & 0x80) != 0;
        v = (v << 1) | (x as u8);
        x = old_msb;
    }
    let new_msb = (v & 0x80) != 0;
    state.set_ccr_xnzvc(x, new_msb, v == 0, false, x);
    v
}

#[inline(always)]
pub fn roxl_w(state: &mut CpuState, count: u32, val: u16) -> u16 {
    let msb = (val & 0x8000) != 0;
    if count == 0 {
        let x = state.get_x();
        state.set_ccr_nzvc(msb, val == 0, false, x);
        return val;
    }
    let mut v = val;
    let mut x = state.get_x();
    for _ in 0..count {
        let old_msb = (v & 0x8000) != 0;
        v = (v << 1) | (x as u16);
        x = old_msb;
    }
    let new_msb = (v & 0x8000) != 0;
    state.set_ccr_xnzvc(x, new_msb, v == 0, false, x);
    v
}

#[inline(always)]
pub fn roxl_l(state: &mut CpuState, count: u32, val: u32) -> u32 {
    let msb = (val & 0x8000_0000) != 0;
    if count == 0 {
        let x = state.get_x();
        state.set_ccr_nzvc(msb, val == 0, false, x);
        return val;
    }
    let mut v = val;
    let mut x = state.get_x();
    for _ in 0..count {
        let old_msb = (v & 0x8000_0000) != 0;
        v = (v << 1) | (x as u32);
        x = old_msb;
    }
    let new_msb = (v & 0x8000_0000) != 0;
    state.set_ccr_xnzvc(x, new_msb, v == 0, false, x);
    v
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_roxl_b_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_byte(reg_dst as usize);
    let res = roxl_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    let duration = 2 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_roxl_w_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_word(reg_dst as usize);
    let res = roxl_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    let duration = 2 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_roxl_l_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_long(reg_dst as usize);
    let res = roxl_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    let duration = 4 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_roxl_b_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_byte(reg_dst as usize);
    let res = roxl_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    let duration = 2 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_roxl_w_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_word(reg_dst as usize);
    let res = roxl_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    let duration = 2 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_roxl_l_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_long(reg_dst as usize);
    let res = roxl_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    let duration = 4 + (2 * count);
    state.micro.clocks_remaining = duration as u16;
}

pub fn alu_roxl_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = (state.micro.destination & 0xFFFF) as u16;
    let res = roxl_w(state, 1, d);
    state.micro.destination = (state.micro.destination & !0xFFFF) | (res as u32);
}

// ============================================================================
// Static Micro-Step Slices: ROXL Register
// ============================================================================

pub static STEPS_ROXL_B_IMM: [MicroStep; 3] = [
    MicroStep::alu(alu_roxl_b_imm_dn),
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];
pub static STEPS_ROXL_W_IMM: [MicroStep; 3] = [
    MicroStep::alu(alu_roxl_w_imm_dn),
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];
pub static STEPS_ROXL_L_IMM: [MicroStep; 3] = [
    MicroStep::alu(alu_roxl_l_imm_dn),
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_ROXL_B_REG: [MicroStep; 3] = [
    MicroStep::alu(alu_roxl_b_reg_dn),
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];
pub static STEPS_ROXL_W_REG: [MicroStep; 3] = [
    MicroStep::alu(alu_roxl_w_reg_dn),
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];
pub static STEPS_ROXL_L_REG: [MicroStep; 3] = [
    MicroStep::alu(alu_roxl_l_reg_dn),
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// Static Micro-Step Slices: ROXL Memory (Word only, Count = 1)
// ============================================================================

pub static STEPS_ROXL_W_AI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_roxl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ROXL_W_PI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_roxl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ROXL_W_PD: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_roxl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ROXL_W_D16_AN: [MicroStep; 8] = [
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
        alu_fn: Some(alu_roxl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ROXL_W_IDX_AN: [MicroStep; 9] = [
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
        alu_fn: Some(alu_roxl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ROXL_W_ABSW: [MicroStep; 8] = [
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
        alu_fn: Some(alu_roxl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_ROXL_W_ABSL: [MicroStep; 10] = [
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
        alu_fn: Some(alu_roxl_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

// ============================================================================
// Static Decoder Functions
// ============================================================================

/// Decodes the micro-step sequence for ROXL register shift
pub const fn decode_roxl_reg_steps(is_reg_count: bool, size: u8) -> Option<&'static [MicroStep]> {
    if is_reg_count {
        match size {
            0 => Some(&STEPS_ROXL_B_REG),
            1 => Some(&STEPS_ROXL_W_REG),
            2 => Some(&STEPS_ROXL_L_REG),
            _ => None,
        }
    } else {
        match size {
            0 => Some(&STEPS_ROXL_B_IMM),
            1 => Some(&STEPS_ROXL_W_IMM),
            2 => Some(&STEPS_ROXL_L_IMM),
            _ => None,
        }
    }
}

/// Decodes the micro-step sequence for ROXL memory shift (Word only, Count = 1)
pub const fn decode_roxl_mem_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        2 => Some(&STEPS_ROXL_W_AI),
        3 => Some(&STEPS_ROXL_W_PI),
        4 => Some(&STEPS_ROXL_W_PD),
        5 => Some(&STEPS_ROXL_W_D16_AN),
        6 => Some(&STEPS_ROXL_W_IDX_AN),
        7 => match reg {
            0 => Some(&STEPS_ROXL_W_ABSW),
            1 => Some(&STEPS_ROXL_W_ABSL),
            _ => None,
        },
        _ => None,
    }
}
