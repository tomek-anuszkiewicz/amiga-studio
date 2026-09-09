//! M68000 AND Dn, <ea> Micro-Step Execution Slices (RMW Class 0)
//!
//! Provides static execution slices for AND with memory destination.

use super::*;
use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;

// ============================================================================
// Static Micro-Step Slices: AND Dn, <ea> (Byte RMW)
// ============================================================================

pub static STEPS_AND_B_DN_AI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_AND_B_DN_PI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pi_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_AND_B_DN_PD: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_AND_B_DN_D16_AN: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_AND_B_DN_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_AND_B_DN_ABSW: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_AND_B_DN_ABSL: [MicroStep; 10] = [
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
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

// ============================================================================
// Static Micro-Step Slices: AND Dn, <ea> (Word RMW)
// ============================================================================

pub static STEPS_AND_W_DN_AI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_AND_W_DN_PI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_AND_W_DN_PD: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_AND_W_DN_D16_AN: [MicroStep; 8] = [
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
        alu_fn: Some(alu_and_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_AND_W_DN_IDX_AN: [MicroStep; 9] = [
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
        alu_fn: Some(alu_and_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_AND_W_DN_ABSW: [MicroStep; 8] = [
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
        alu_fn: Some(alu_and_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_AND_W_DN_ABSL: [MicroStep; 10] = [
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
        alu_fn: Some(alu_and_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

// ============================================================================
// Static Micro-Step Slices: AND Dn, <ea> (Long RMW)
// ============================================================================

pub static STEPS_AND_L_DN_AI: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_AND_L_DN_PI: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: Some(ea::ea_calc_dst_pi_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_AND_L_DN_PD: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: Some(ea::ea_calc_dst_pd_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_AND_L_DN_D16_AN: [MicroStep; 12] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_AND_L_DN_IDX_AN: [MicroStep; 13] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_AND_L_DN_ABSW: [MicroStep; 12] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_AND_L_DN_ABSL: [MicroStep; 14] = [
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
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_and_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

// ============================================================================
// Decoder: AND Dn, <ea>
// ============================================================================

pub const fn decode_and_dn_ea_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            2 => Some(&STEPS_AND_B_DN_AI),
            3 => Some(&STEPS_AND_B_DN_PI),
            4 => Some(&STEPS_AND_B_DN_PD),
            5 => Some(&STEPS_AND_B_DN_D16_AN),
            6 => Some(&STEPS_AND_B_DN_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_AND_B_DN_ABSW),
                1 => Some(&STEPS_AND_B_DN_ABSL),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            2 => Some(&STEPS_AND_W_DN_AI),
            3 => Some(&STEPS_AND_W_DN_PI),
            4 => Some(&STEPS_AND_W_DN_PD),
            5 => Some(&STEPS_AND_W_DN_D16_AN),
            6 => Some(&STEPS_AND_W_DN_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_AND_W_DN_ABSW),
                1 => Some(&STEPS_AND_W_DN_ABSL),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            2 => Some(&STEPS_AND_L_DN_AI),
            3 => Some(&STEPS_AND_L_DN_PI),
            4 => Some(&STEPS_AND_L_DN_PD),
            5 => Some(&STEPS_AND_L_DN_D16_AN),
            6 => Some(&STEPS_AND_L_DN_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_AND_L_DN_ABSW),
                1 => Some(&STEPS_AND_L_DN_ABSL),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
