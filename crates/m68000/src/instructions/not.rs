//! M68000 NOT Instruction (`NOT <ea>`)
//!
//! Performs bitwise NOT (one's complement) on a destination data register or memory location.

use crate::micro::common;
use crate::micro::ea;
use crate::core::Cpu;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Leaf ALU NOT Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn not_b(state: &mut CpuState, d: u8) -> u8 {
    let res = !d;
    state.set_ccr_nz_clear_vc((res & 0x80) != 0, res == 0);
    res
}

#[inline(always)]
pub fn not_w(state: &mut CpuState, d: u16) -> u16 {
    let res = !d;
    state.set_ccr_nz_clear_vc((res & 0x8000) != 0, res == 0);
    res
}

#[inline(always)]
pub fn not_l(state: &mut CpuState, d: u32) -> u32 {
    let res = !d;
    state.set_ccr_nz_clear_vc((res & 0x8000_0000) != 0, res == 0);
    res
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_not_b_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = not_b(state, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_not_w_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = not_w(state, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_not_l_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let d = state.d_long(reg_dst as usize);
    let res = not_l(state, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_not_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = (state.micro.last_read & 0xFF) as u8;
    let res = not_b(state, d);
    state.micro.write_buffer = res as u32;
}

pub fn alu_not_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = state.micro.last_read;
    let res = not_w(state, d);
    state.micro.write_buffer = res as u32;
}

pub fn alu_not_l_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = state.micro.scratch[1];
    let res = not_l(state, d);
    state.micro.write_buffer = res;
}

// ============================================================================
// Static Micro-Step Slices: NOT <ea>
// ============================================================================

// Byte
pub static STEPS_NOT_B_DN: [MicroStep; 1] = [
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_not_b_dn), base_clocks: 4 },
];
pub static STEPS_NOT_B_AI: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_b_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_NOT_B_PI: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: Some(ea::ea_calc_dst_pi_b), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_b_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_NOT_B_PD: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_b_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_NOT_B_D16: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_b_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_NOT_B_IDX: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_b_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_NOT_B_ABSW: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_b_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_NOT_B_ABSL: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_b_mem), base_clocks: 4 },
    common::RMW_WRITE_BYTE_RETIRE,
];

// Word
pub static STEPS_NOT_W_DN: [MicroStep; 1] = [
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_not_w_dn), base_clocks: 4 },
];
pub static STEPS_NOT_W_AI: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_NOT_W_PI: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: Some(ea::ea_calc_dst_pi_w), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_NOT_W_PD: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_pd_w), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_NOT_W_D16: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_NOT_W_IDX: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_NOT_W_ABSW: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_NOT_W_ABSL: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_w_mem), base_clocks: 4 },
    common::RMW_WRITE_WORD_RETIRE,
];

// Long
pub static STEPS_NOT_L_DN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: None, base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_not_l_dn), base_clocks: 4 },
];
pub static STEPS_NOT_L_AI: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_NOT_L_PI: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: Some(ea::ea_calc_dst_pi_l), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_NOT_L_PD: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_pd_l), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_NOT_L_D16: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_NOT_L_IDX: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_NOT_L_ABSW: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_NOT_L_ABSL: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_prefetch_to_scratch, alu_fn: Some(alu_not_l_mem), base_clocks: 4 },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

// ============================================================================
// Static Decoder Function
// ============================================================================

/// Decodes the micro-step sequence for NOT based on size, mode, and reg
pub const fn decode_not_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_NOT_B_DN),
            2 => Some(&STEPS_NOT_B_AI),
            3 => Some(&STEPS_NOT_B_PI),
            4 => Some(&STEPS_NOT_B_PD),
            5 => Some(&STEPS_NOT_B_D16),
            6 => Some(&STEPS_NOT_B_IDX),
            7 => match reg {
                0 => Some(&STEPS_NOT_B_ABSW),
                1 => Some(&STEPS_NOT_B_ABSL),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_NOT_W_DN),
            2 => Some(&STEPS_NOT_W_AI),
            3 => Some(&STEPS_NOT_W_PI),
            4 => Some(&STEPS_NOT_W_PD),
            5 => Some(&STEPS_NOT_W_D16),
            6 => Some(&STEPS_NOT_W_IDX),
            7 => match reg {
                0 => Some(&STEPS_NOT_W_ABSW),
                1 => Some(&STEPS_NOT_W_ABSL),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_NOT_L_DN),
            2 => Some(&STEPS_NOT_L_AI),
            3 => Some(&STEPS_NOT_L_PI),
            4 => Some(&STEPS_NOT_L_PD),
            5 => Some(&STEPS_NOT_L_D16),
            6 => Some(&STEPS_NOT_L_IDX),
            7 => match reg {
                0 => Some(&STEPS_NOT_L_ABSW),
                1 => Some(&STEPS_NOT_L_ABSL),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

