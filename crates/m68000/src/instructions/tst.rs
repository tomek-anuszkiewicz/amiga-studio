//! M68000 TST Instruction (`TST <ea>`)
//!
//! Compares the operand with zero and sets the condition codes (N, Z) accordingly.
//! Clears V and C. Extend (X) flag is unaffected.
//! Only data alterable addressing modes are allowed on the 68000.

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Core Leaf Flag Evaluation (Branchless & Cycle-Exact CCR)
// ============================================================================

#[inline(always)]
pub fn tst_b(state: &mut CpuState, val: u8) {
    state.set_ccr_nz_clear_vc((val & 0x80) != 0, val == 0);
}

#[inline(always)]
pub fn tst_w(state: &mut CpuState, val: u16) {
    state.set_ccr_nz_clear_vc((val & 0x8000) != 0, val == 0);
}

#[inline(always)]
pub fn tst_l(state: &mut CpuState, val: u32) {
    state.set_ccr_nz_clear_vc((val & 0x8000_0000) != 0, val == 0);
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_tst_b_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    tst_b(state, d);
}

pub fn alu_tst_w_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    tst_w(state, d);
}

pub fn alu_tst_l_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let d = state.d_long(reg_dst as usize);
    tst_l(state, d);
}

pub fn alu_tst_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = (state.micro.destination & 0xFF) as u8;
    tst_b(state, d);
}

pub fn alu_tst_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = (state.micro.destination & 0xFFFF) as u16;
    tst_w(state, d);
}

pub fn alu_tst_l_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = state.micro.destination;
    tst_l(state, d);
}

// ============================================================================
// Static Micro-Step Slices: TST Byte
// ============================================================================

pub static STEPS_TST_B_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_b_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_B_AI: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_B_PI: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pi_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_B_PD: [MicroStep; 5] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_B_D16_AN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_B_IDX_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_B_ABSW: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_B_ABSL: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_b_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

// ============================================================================
// Static Micro-Step Slices: TST Word
// ============================================================================

pub static STEPS_TST_W_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_w_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_W_AI: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_W_PI: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_W_PD: [MicroStep; 5] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: 2,
    },
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_W_D16_AN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_W_IDX_AN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_W_ABSW: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_W_ABSL: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_w_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

// ============================================================================
// Static Micro-Step Slices: TST Long
// ============================================================================

pub static STEPS_TST_L_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_l_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_L_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_L_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: Some(ea::ea_calc_dst_pi_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_L_PD: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_l),
        base_clocks: 2,
    },
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_L_D16_AN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_L_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_L_ABSW: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_TST_L_ABSL: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_tst_l_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

// ============================================================================
// Instruction Decoder Callback
// ============================================================================

pub const fn decode_tst_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_TST_B_DN),
            2 => Some(&STEPS_TST_B_AI),
            3 => Some(&STEPS_TST_B_PI),
            4 => Some(&STEPS_TST_B_PD),
            5 => Some(&STEPS_TST_B_D16_AN),
            6 => Some(&STEPS_TST_B_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_TST_B_ABSW),
                1 => Some(&STEPS_TST_B_ABSL),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_TST_W_DN),
            2 => Some(&STEPS_TST_W_AI),
            3 => Some(&STEPS_TST_W_PI),
            4 => Some(&STEPS_TST_W_PD),
            5 => Some(&STEPS_TST_W_D16_AN),
            6 => Some(&STEPS_TST_W_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_TST_W_ABSW),
                1 => Some(&STEPS_TST_W_ABSL),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_TST_L_DN),
            2 => Some(&STEPS_TST_L_AI),
            3 => Some(&STEPS_TST_L_PI),
            4 => Some(&STEPS_TST_L_PD),
            5 => Some(&STEPS_TST_L_D16_AN),
            6 => Some(&STEPS_TST_L_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_TST_L_ABSW),
                1 => Some(&STEPS_TST_L_ABSL),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
