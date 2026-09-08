//! ROL (Rotate Left without Extend) instruction handlers and CCR updates
//!
//! Rotates bits to the left without using the Extend flag (X is unaffected).
//! C flag receives the last bit shifted out. V flag is always cleared.

use crate::micro::common;
use crate::micro::ea;
use crate::core::Cpu;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Leaf ALU ROL Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn rol_b(state: &mut CpuState, count: u32, val: u8) -> u8 {
    let msb = (val & 0x80) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let k = count % 8;
    let (res, last_out) = if k == 0 {
        (val, (val & 1) != 0)
    } else {
        let r = val.rotate_left(k);
        (r, (r & 1) != 0)
    };
    state.set_ccr_nzc_clear_v((res & 0x80) != 0, res == 0, last_out);
    res
}

#[inline(always)]
pub fn rol_w(state: &mut CpuState, count: u32, val: u16) -> u16 {
    let msb = (val & 0x8000) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let k = count % 16;
    let (res, last_out) = if k == 0 {
        (val, (val & 1) != 0)
    } else {
        let r = val.rotate_left(k);
        (r, (r & 1) != 0)
    };
    state.set_ccr_nzc_clear_v((res & 0x8000) != 0, res == 0, last_out);
    res
}

#[inline(always)]
pub fn rol_l(state: &mut CpuState, count: u32, val: u32) -> u32 {
    let msb = (val & 0x8000_0000) != 0;
    if count == 0 {
        state.set_ccr_nz_clear_vc(msb, val == 0);
        return val;
    }
    let k = count % 32;
    let (res, last_out) = if k == 0 {
        (val, (val & 1) != 0)
    } else {
        let r = val.rotate_left(k);
        (r, (r & 1) != 0)
    };
    state.set_ccr_nzc_clear_v((res & 0x8000_0000) != 0, res == 0, last_out);
    res
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_rol_b_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_byte(reg_dst as usize);
    let res = rol_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    state.micro.record_internal_clocks(2 + (2 * count as u16));
}

pub fn alu_rol_w_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_word(reg_dst as usize);
    let res = rol_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    state.micro.record_internal_clocks(2 + (2 * count as u16));
}

pub fn alu_rol_l_imm_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = reg_src as u32;
    let val = state.d_long(reg_dst as usize);
    let res = rol_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    state.micro.record_internal_clocks(4 + (2 * count as u16));
}

pub fn alu_rol_b_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_byte(reg_dst as usize);
    let res = rol_b(state, count, val);
    state.set_d_byte(reg_dst as usize, res);
    state.micro.record_internal_clocks(2 + (2 * count as u16));
}

pub fn alu_rol_w_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_word(reg_dst as usize);
    let res = rol_w(state, count, val);
    state.set_d_word(reg_dst as usize, res);
    state.micro.record_internal_clocks(2 + (2 * count as u16));
}

pub fn alu_rol_l_reg_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let count = state.d_long(reg_src as usize) & 63;
    let val = state.d_long(reg_dst as usize);
    let res = rol_l(state, count, val);
    state.set_d_long(reg_dst as usize, res);
    state.micro.record_internal_clocks(4 + (2 * count as u16));
}

pub fn alu_rol_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = state.micro.last_read;
    let res = rol_w(state, 1, d);
    state.micro.write_buffer = res as u32;
}

// ============================================================================
// Static Micro-Step Slices: ROL Register
// ============================================================================

pub static STEPS_ROL_B_IMM: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(alu_rol_b_imm_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];
pub static STEPS_ROL_W_IMM: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(alu_rol_w_imm_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];
pub static STEPS_ROL_L_IMM: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(alu_rol_l_imm_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];

pub static STEPS_ROL_B_REG: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(alu_rol_b_reg_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];
pub static STEPS_ROL_W_REG: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(alu_rol_w_reg_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];
pub static STEPS_ROL_L_REG: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(alu_rol_l_reg_dn), base_clocks: 0 },
    common::RETIRE_STANDARD,
];

// ============================================================================
// Static Micro-Step Slices: ROL Memory (Word only, Count = 1)
// ============================================================================

pub static STEPS_ROL_W_AI: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_rol_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROL_W_PI: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: Some(ea::ea_calc_dst_pi_w), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_rol_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROL_W_PD: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_pd_w), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_rol_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROL_W_D16: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_rol_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROL_W_IDX: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_rol_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROL_W_ABSW: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_rol_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ROL_W_ABSL: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_rol_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];

// ============================================================================
// Static Decoder Functions
// ============================================================================

/// Decodes the micro-step sequence for ROL register rotate
pub const fn decode_rol_reg_steps(is_reg_count: bool, size: u8) -> Option<&'static [MicroStep]> {
    if is_reg_count {
        match size {
            0 => Some(&STEPS_ROL_B_REG),
            1 => Some(&STEPS_ROL_W_REG),
            2 => Some(&STEPS_ROL_L_REG),
            _ => None,
        }
    } else {
        match size {
            0 => Some(&STEPS_ROL_B_IMM),
            1 => Some(&STEPS_ROL_W_IMM),
            2 => Some(&STEPS_ROL_L_IMM),
            _ => None,
        }
    }
}

/// Decodes the micro-step sequence for ROL memory rotate (Word only, Count = 1)
pub const fn decode_rol_mem_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        2 => Some(&STEPS_ROL_W_AI),
        3 => Some(&STEPS_ROL_W_PI),
        4 => Some(&STEPS_ROL_W_PD),
        5 => Some(&STEPS_ROL_W_D16),
        6 => Some(&STEPS_ROL_W_IDX),
        7 => match reg {
            0 => Some(&STEPS_ROL_W_ABSW),
            1 => Some(&STEPS_ROL_W_ABSL),
            _ => None,
        },
        _ => None,
    }
}


