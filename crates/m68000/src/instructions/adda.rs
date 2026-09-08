//! M68000 ADDA Instruction (`ADDA <ea>, An`)
//!
//! Adds an effective address operand to an address register without modifying CCR flags.
//! Sign-extends word operands to 32 bits prior to addition.

use crate::micro::ea;
use crate::core::Cpu;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Micro-Step ALU Callbacks (AluFn)
// ============================================================================

pub fn alu_adda_w_dn_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) as i16 as i32) as u32;
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_add(s));
}

pub fn alu_adda_w_an_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.read_a(reg_src as usize) as i16 as i32) as u32;
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_add(s));
}

pub fn alu_adda_l_dn_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_add(s));
}

pub fn alu_adda_l_an_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.read_a(reg_src as usize);
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_add(s));
}

pub fn alu_adda_w_mem_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.last_read as i16 as i32) as u32;
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_add(s));
}

pub fn alu_adda_l_mem_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.scratch[1];
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_add(s));
}

pub fn alu_adda_w_imm_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] as i16 as i32) as u32;
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_add(s));
}

// ============================================================================
// Static Micro-Step Slices: ADDA <ea>, An
// ============================================================================

// Word: <ea>, An
pub static STEPS_ADDA_W_DN_AN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_w_dn_an), base_clocks: 4 },
];
pub static STEPS_ADDA_W_AN_AN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_w_an_an), base_clocks: 4 },
];
pub static STEPS_ADDA_W_AI_AN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_w_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_W_PI_AN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: Some(ea::ea_calc_src_pi_w), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_w_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_W_PD_AN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: Some(ea::ea_calc_src_pd_w), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_w_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_W_D16_AN: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_w_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_W_IDX_AN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_w_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_W_ABSW_AN: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_w_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_W_ABSL_AN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_w_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_W_PCD16_AN: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_w_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_W_PCIDX_AN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_word, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_w_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_W_IMM_AN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(alu_adda_w_imm_an), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: None, base_clocks: 4 },
];

// Long: <ea>, An
pub static STEPS_ADDA_L_DN_AN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_l_dn_an), base_clocks: 4 },
];
pub static STEPS_ADDA_L_AN_AN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_l_an_an), base_clocks: 4 },
];
pub static STEPS_ADDA_L_AI_AN: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_l_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_L_PI_AN: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: Some(ea::ea_calc_src_pi_l), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_l_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_L_PD_AN: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_l_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_L_D16_AN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_l_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_L_IDX_AN: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_l_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_L_ABSW_AN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_l_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_L_ABSL_AN: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_l_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_L_PCD16_AN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_l_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_L_PCIDX_AN: [MicroStep; 5] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_high, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_bus_read_long_low, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_l_mem_an), base_clocks: 4 },
];
pub static STEPS_ADDA_L_IMM_AN: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_fetch_extension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_opcode_and_retire, alu_fn: Some(alu_adda_l_mem_an), base_clocks: 4 },
];

// ============================================================================
// Opcode Descriptor Decoder Helper for ADDA
// ============================================================================

/// Maps an ADDA opcode's bit fields to its static micro-step sequence
pub const fn decode_adda_steps(is_long: bool, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    if !is_long {
        // Word: ADDA.W <ea>, An
        match mode {
            0 => Some(&STEPS_ADDA_W_DN_AN),
            1 => Some(&STEPS_ADDA_W_AN_AN),
            2 => Some(&STEPS_ADDA_W_AI_AN),
            3 => Some(&STEPS_ADDA_W_PI_AN),
            4 => Some(&STEPS_ADDA_W_PD_AN),
            5 => Some(&STEPS_ADDA_W_D16_AN),
            6 => Some(&STEPS_ADDA_W_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_ADDA_W_ABSW_AN),
                1 => Some(&STEPS_ADDA_W_ABSL_AN),
                2 => Some(&STEPS_ADDA_W_PCD16_AN),
                3 => Some(&STEPS_ADDA_W_PCIDX_AN),
                4 => Some(&STEPS_ADDA_W_IMM_AN),
                _ => None,
            },
            _ => None,
        }
    } else {
        // Long: ADDA.L <ea>, An
        match mode {
            0 => Some(&STEPS_ADDA_L_DN_AN),
            1 => Some(&STEPS_ADDA_L_AN_AN),
            2 => Some(&STEPS_ADDA_L_AI_AN),
            3 => Some(&STEPS_ADDA_L_PI_AN),
            4 => Some(&STEPS_ADDA_L_PD_AN),
            5 => Some(&STEPS_ADDA_L_D16_AN),
            6 => Some(&STEPS_ADDA_L_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_ADDA_L_ABSW_AN),
                1 => Some(&STEPS_ADDA_L_ABSL_AN),
                2 => Some(&STEPS_ADDA_L_PCD16_AN),
                3 => Some(&STEPS_ADDA_L_PCIDX_AN),
                4 => Some(&STEPS_ADDA_L_IMM_AN),
                _ => None,
            },
            _ => None,
        }
    }
}

