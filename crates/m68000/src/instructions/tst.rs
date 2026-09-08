//! M68000 TST Instruction (`TST <ea>`)
//!
//! Evaluates an effective address operand against zero.
//! Updates N and Z flags according to the operand value; clears V and C flags.
//! Extend (X) flag is unaffected. Operand is not modified.

use crate::micro::ea;
use crate::core::Cpu;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Leaf ALU TST Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn tst_b(state: &mut CpuState, d: u8) {
    state.set_ccr_nz_clear_vc((d & 0x80) != 0, d == 0);
}

#[inline(always)]
pub fn tst_w(state: &mut CpuState, d: u16) {
    state.set_ccr_nz_clear_vc((d & 0x8000) != 0, d == 0);
}

#[inline(always)]
pub fn tst_l(state: &mut CpuState, d: u32) {
    state.set_ccr_nz_clear_vc((d & 0x8000_0000) != 0, d == 0);
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_tst_b_dn(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let d = (state.d_long(reg_src as usize) & 0xFF) as u8;
    tst_b(state, d);
}

pub fn alu_tst_w_dn(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let d = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    tst_w(state, d);
}

pub fn alu_tst_l_dn(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let d = state.d_long(reg_src as usize);
    tst_l(state, d);
}

pub fn alu_tst_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = (state.micro.last_read & 0xFF) as u8;
    tst_b(state, d);
}

pub fn alu_tst_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = state.micro.last_read;
    tst_w(state, d);
}

pub fn alu_tst_l_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = state.micro.scratch[1];
    tst_l(state, d);
}

// ============================================================================
// Static Micro-Step Slices: TST <ea>
// ============================================================================

// Byte
pub static STEPS_TST_B_DN: [MicroStep; 1] = [
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_b_dn), base_clocks: 4 },
];
pub static STEPS_TST_B_AI: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4 },
];
pub static STEPS_TST_B_PI: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4 },
];
pub static STEPS_TST_B_PD: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4 },
];
pub static STEPS_TST_B_D16: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4 },
];
pub static STEPS_TST_B_IDX: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4 },
];
pub static STEPS_TST_B_ABSW: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4 },
];
pub static STEPS_TST_B_ABSL: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4 },
];
pub static STEPS_TST_B_PCD16: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4 },
];
pub static STEPS_TST_B_PCIDX: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_byte, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4 },
];

// Word
pub static STEPS_TST_W_DN: [MicroStep; 1] = [
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_w_dn), base_clocks: 4 },
];
pub static STEPS_TST_W_AI: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4 },
];
pub static STEPS_TST_W_PI: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: Some(ea::ea_calc_src_pi_w), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4 },
];
pub static STEPS_TST_W_PD: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: Some(ea::ea_calc_src_pd_w), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4 },
];
pub static STEPS_TST_W_D16: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4 },
];
pub static STEPS_TST_W_IDX: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4 },
];
pub static STEPS_TST_W_ABSW: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4 },
];
pub static STEPS_TST_W_ABSL: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4 },
];
pub static STEPS_TST_W_PCD16: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4 },
];
pub static STEPS_TST_W_PCIDX: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4 },
];

// Long
pub static STEPS_TST_L_DN: [MicroStep; 1] = [
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_l_dn), base_clocks: 4 },
];
pub static STEPS_TST_L_AI: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4 },
];
pub static STEPS_TST_L_PI: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: Some(ea::ea_calc_src_pi_l), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4 },
];
pub static STEPS_TST_L_PD: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4 },
];
pub static STEPS_TST_L_D16: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4 },
];
pub static STEPS_TST_L_IDX: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4 },
];
pub static STEPS_TST_L_ABSW: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4 },
];
pub static STEPS_TST_L_ABSL: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4 },
];
pub static STEPS_TST_L_PCD16: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4 },
];
pub static STEPS_TST_L_PCIDX: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4 },
];

// ============================================================================
// Static Decoder Function
// ============================================================================

/// Decodes the micro-step sequence for TST based on size, mode, and reg
pub const fn decode_tst_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_TST_B_DN),
            2 => Some(&STEPS_TST_B_AI),
            3 => Some(&STEPS_TST_B_PI),
            4 => Some(&STEPS_TST_B_PD),
            5 => Some(&STEPS_TST_B_D16),
            6 => Some(&STEPS_TST_B_IDX),
            7 => match reg {
                0 => Some(&STEPS_TST_B_ABSW),
                1 => Some(&STEPS_TST_B_ABSL),
                2 => Some(&STEPS_TST_B_PCD16),
                3 => Some(&STEPS_TST_B_PCIDX),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_TST_W_DN),
            2 => Some(&STEPS_TST_W_AI),
            3 => Some(&STEPS_TST_W_PI),
            4 => Some(&STEPS_TST_W_PD),
            5 => Some(&STEPS_TST_W_D16),
            6 => Some(&STEPS_TST_W_IDX),
            7 => match reg {
                0 => Some(&STEPS_TST_W_ABSW),
                1 => Some(&STEPS_TST_W_ABSL),
                2 => Some(&STEPS_TST_W_PCD16),
                3 => Some(&STEPS_TST_W_PCIDX),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_TST_L_DN),
            2 => Some(&STEPS_TST_L_AI),
            3 => Some(&STEPS_TST_L_PI),
            4 => Some(&STEPS_TST_L_PD),
            5 => Some(&STEPS_TST_L_D16),
            6 => Some(&STEPS_TST_L_IDX),
            7 => match reg {
                0 => Some(&STEPS_TST_L_ABSW),
                1 => Some(&STEPS_TST_L_ABSL),
                2 => Some(&STEPS_TST_L_PCD16),
                3 => Some(&STEPS_TST_L_PCIDX),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

