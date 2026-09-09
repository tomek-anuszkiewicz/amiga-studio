//! M68000 OR <ea>, Dn Micro-Step Execution Slices
//!
//! Provides static execution slices for OR with data register destination.

use super::*;
use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;

// ============================================================================
// Static Micro-Step Slices: OR <ea>, Dn (Byte)
// ============================================================================

pub static STEPS_OR_B_DN_DN: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_AI_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_PI_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_pi_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_PD_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_pd_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_D16_AN_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_IDX_AN_DN: [MicroStep; 7] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_ABSW_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_ABSL_DN: [MicroStep; 8] = [
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
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_D16_PC_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_IDX_PC_DN: [MicroStep; 7] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_IMM_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_or_b_imm_dn),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// Static Micro-Step Slices: OR <ea>, Dn (Word)
// ============================================================================

pub static STEPS_OR_W_DN_DN: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_AI_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_PI_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_PD_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_D16_AN_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_IDX_AN_DN: [MicroStep; 7] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_ABSW_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_ABSL_DN: [MicroStep; 8] = [
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
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_D16_PC_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_IDX_PC_DN: [MicroStep; 7] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_IMM_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_or_w_imm_dn),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// Static Micro-Step Slices: OR <ea>, Dn (Long)
// ============================================================================

pub static STEPS_OR_L_DN_DN: [MicroStep; 3] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 4,
    },
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_AI_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_PI_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_PD_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_D16_AN_DN: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_IDX_AN_DN: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_ABSW_DN: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_ABSL_DN: [MicroStep; 10] = [
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
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_D16_PC_DN: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_IDX_PC_DN: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_IMM_DN: [MicroStep; 6] = [
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
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

// ============================================================================
// Decoder: OR <ea>, Dn
// ============================================================================

pub const fn decode_or_ea_dn_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_OR_B_DN_DN),
            2 => Some(&STEPS_OR_B_AI_DN),
            3 => Some(&STEPS_OR_B_PI_DN),
            4 => Some(&STEPS_OR_B_PD_DN),
            5 => Some(&STEPS_OR_B_D16_AN_DN),
            6 => Some(&STEPS_OR_B_IDX_AN_DN),
            7 => match reg {
                0 => Some(&STEPS_OR_B_ABSW_DN),
                1 => Some(&STEPS_OR_B_ABSL_DN),
                2 => Some(&STEPS_OR_B_D16_PC_DN),
                3 => Some(&STEPS_OR_B_IDX_PC_DN),
                4 => Some(&STEPS_OR_B_IMM_DN),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_OR_W_DN_DN),
            2 => Some(&STEPS_OR_W_AI_DN),
            3 => Some(&STEPS_OR_W_PI_DN),
            4 => Some(&STEPS_OR_W_PD_DN),
            5 => Some(&STEPS_OR_W_D16_AN_DN),
            6 => Some(&STEPS_OR_W_IDX_AN_DN),
            7 => match reg {
                0 => Some(&STEPS_OR_W_ABSW_DN),
                1 => Some(&STEPS_OR_W_ABSL_DN),
                2 => Some(&STEPS_OR_W_D16_PC_DN),
                3 => Some(&STEPS_OR_W_IDX_PC_DN),
                4 => Some(&STEPS_OR_W_IMM_DN),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_OR_L_DN_DN),
            2 => Some(&STEPS_OR_L_AI_DN),
            3 => Some(&STEPS_OR_L_PI_DN),
            4 => Some(&STEPS_OR_L_PD_DN),
            5 => Some(&STEPS_OR_L_D16_AN_DN),
            6 => Some(&STEPS_OR_L_IDX_AN_DN),
            7 => match reg {
                0 => Some(&STEPS_OR_L_ABSW_DN),
                1 => Some(&STEPS_OR_L_ABSL_DN),
                2 => Some(&STEPS_OR_L_D16_PC_DN),
                3 => Some(&STEPS_OR_L_IDX_PC_DN),
                4 => Some(&STEPS_OR_L_IMM_DN),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
