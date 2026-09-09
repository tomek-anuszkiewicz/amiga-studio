//! MOVEA (Move Address) instruction handlers
//!
//! Loads an effective address operand into an Address Register (An)
//! with word sign-extension, leaving CCR completely untouched.

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Pure ALU Callbacks: MOVEA Word
// ============================================================================

#[inline(always)]
pub fn alu_movea_w_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = (state.d_word(reg_src as usize) as i16 as i32) as u32;
    state.write_a(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_movea_w_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = (state.a_word(reg_src as usize) as i16 as i32) as u32;
    state.write_a(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_movea_w_mem(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = (state.micro.source as i16 as i32) as u32;
    state.write_a(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_movea_w_imm(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = (state.prefetch[0] as i16 as i32) as u32;
    state.write_a(reg_dst as usize, val);
}

// ============================================================================
// Pure ALU Callbacks: MOVEA Long
// ============================================================================

#[inline(always)]
pub fn alu_movea_l_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.d_long(reg_src as usize);
    state.write_a(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_movea_l_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let val = state.read_a(reg_src as usize);
    state.write_a(reg_dst as usize, val);
}

#[inline(always)]
pub fn alu_movea_l_mem(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.micro.source;
    state.write_a(reg_dst as usize, val);
}

// ============================================================================
// Static Step Slices: MOVEA Word
// ============================================================================

pub static STEPS_MOVEA_W_DN: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_W_AN: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_an),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_W_AI: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_W_PI: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_W_PD: [MicroStep; 5] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_W_D16_AN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_W_IDX_AN: [MicroStep; 7] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_W_ABSW: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_W_ABSL: [MicroStep; 8] = [
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
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_W_D16_PC: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_W_IDX_PC: [MicroStep; 7] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_w_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_W_IMM: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_movea_w_imm),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];

// ============================================================================
// Static Step Slices: MOVEA Long
// ============================================================================

pub static STEPS_MOVEA_L_DN: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_L_AN: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_an),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_L_AI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_L_PI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_L_PD: [MicroStep; 7] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_L_D16_AN: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_L_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_L_ABSW: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_L_ABSL: [MicroStep; 10] = [
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
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_L_D16_PC: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_L_IDX_PC: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_MOVEA_L_IMM: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_movea_l_mem),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

// ============================================================================
// Opcode Descriptor Decoder Helper for MOVEA
// ============================================================================

/// Compile-time opcode decoder for MOVEA
pub const fn decode_movea_steps(is_long: bool, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    if is_long {
        match mode {
            0 => Some(&STEPS_MOVEA_L_DN),
            1 => Some(&STEPS_MOVEA_L_AN),
            2 => Some(&STEPS_MOVEA_L_AI),
            3 => Some(&STEPS_MOVEA_L_PI),
            4 => Some(&STEPS_MOVEA_L_PD),
            5 => Some(&STEPS_MOVEA_L_D16_AN),
            6 => Some(&STEPS_MOVEA_L_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_MOVEA_L_ABSW),
                1 => Some(&STEPS_MOVEA_L_ABSL),
                2 => Some(&STEPS_MOVEA_L_D16_PC),
                3 => Some(&STEPS_MOVEA_L_IDX_PC),
                4 => Some(&STEPS_MOVEA_L_IMM),
                _ => None,
            },
            _ => None,
        }
    } else {
        match mode {
            0 => Some(&STEPS_MOVEA_W_DN),
            1 => Some(&STEPS_MOVEA_W_AN),
            2 => Some(&STEPS_MOVEA_W_AI),
            3 => Some(&STEPS_MOVEA_W_PI),
            4 => Some(&STEPS_MOVEA_W_PD),
            5 => Some(&STEPS_MOVEA_W_D16_AN),
            6 => Some(&STEPS_MOVEA_W_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_MOVEA_W_ABSW),
                1 => Some(&STEPS_MOVEA_W_ABSL),
                2 => Some(&STEPS_MOVEA_W_D16_PC),
                3 => Some(&STEPS_MOVEA_W_IDX_PC),
                4 => Some(&STEPS_MOVEA_W_IMM),
                _ => None,
            },
            _ => None,
        }
    }
}
