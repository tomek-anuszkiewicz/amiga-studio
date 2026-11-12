//! M68000 NOT Instruction (`NOT <ea>`)
//!
//! Performs bitwise NOT (one's complement) on a destination data register or memory location.

use super::{BIT_BYTE_SIGN, BIT_LONG_SIGN, BIT_WORD_SIGN};
use crate::micro::common;
use crate::micro::ea;
use crate::micro::encoding::{extended_mode, mode, operand_size};
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Leaf ALU NOT Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn not_b(state: &mut CpuState, d: u8) -> u8 {
    let res = !d;
    state.set_ccr_nz_clear_vc((res & BIT_BYTE_SIGN) != 0, res == 0);
    res
}

#[inline(always)]
pub fn not_w(state: &mut CpuState, d: u16) -> u16 {
    let res = !d;
    state.set_ccr_nz_clear_vc((res & BIT_WORD_SIGN) != 0, res == 0);
    res
}

#[inline(always)]
pub fn not_l(state: &mut CpuState, d: u32) -> u32 {
    let res = !d;
    state.set_ccr_nz_clear_vc((res & BIT_LONG_SIGN) != 0, res == 0);
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
    let d = (state.micro().destination & 0xFF) as u8;
    let res = not_b(state, d);
    state.micro_mut().destination = (state.micro().destination & !0xFF) | (res as u32);
}

pub fn alu_not_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = (state.micro().destination & 0xFFFF) as u16;
    let res = not_w(state, d);
    state.micro_mut().destination = (state.micro().destination & !0xFFFF) | (res as u32);
}

pub fn alu_not_l_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = state.micro().destination;
    let res = not_l(state, d);
    state.micro_mut().destination = res;
}

// ============================================================================
// Static Micro-Step Slices: NOT <ea>
// ============================================================================

// Byte
pub static STEPS_NOT_B_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_b_dn),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];
pub static STEPS_NOT_B_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_b_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];
pub static STEPS_NOT_B_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pi_b),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_b_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];
pub static STEPS_NOT_B_PD: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_b_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];
pub static STEPS_NOT_B_D16: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_b_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];
pub static STEPS_NOT_B_IDX: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_b_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];
pub static STEPS_NOT_B_ABSW: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_b_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];
pub static STEPS_NOT_B_ABSL: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_b_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

// Word
pub static STEPS_NOT_W_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_w_dn),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
];
pub static STEPS_NOT_W_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_NOT_W_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pi_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_NOT_W_PD: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_NOT_W_D16: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_NOT_W_IDX: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_NOT_W_ABSW: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];
pub static STEPS_NOT_W_ABSL: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_w_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

// Long
pub static STEPS_NOT_L_DN: [MicroStep; 3] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_l_dn),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];
pub static STEPS_NOT_L_AI: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
];
pub static STEPS_NOT_L_PI: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: Some(ea::ea_calc_dst_pi_l),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
];
pub static STEPS_NOT_L_PD: [MicroStep; 11] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_l),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
];
pub static STEPS_NOT_L_D16: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
];
pub static STEPS_NOT_L_IDX: [MicroStep; 13] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
];
pub static STEPS_NOT_L_ABSW: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
];
pub static STEPS_NOT_L_ABSL: [MicroStep; 14] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_not_l_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
];

// ============================================================================
// Static Decoder Function
// ============================================================================

/// Decodes the micro-step sequence for NOT based on size, mode, and reg
pub const fn decode_not_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        operand_size::BYTE => match mode {
            mode::DATA_REGISTER => Some(&STEPS_NOT_B_DN),
            mode::INDIRECT => Some(&STEPS_NOT_B_AI),
            mode::POSTINCREMENT => Some(&STEPS_NOT_B_PI),
            mode::PREDECREMENT => Some(&STEPS_NOT_B_PD),
            mode::DISPLACEMENT => Some(&STEPS_NOT_B_D16),
            mode::INDEXED => Some(&STEPS_NOT_B_IDX),
            mode::EXTENDED => match reg {
                extended_mode::ABSOLUTE_WORD => Some(&STEPS_NOT_B_ABSW),
                extended_mode::ABSOLUTE_LONG => Some(&STEPS_NOT_B_ABSL),
                _ => None,
            },
            _ => None,
        },
        operand_size::WORD => match mode {
            mode::DATA_REGISTER => Some(&STEPS_NOT_W_DN),
            mode::INDIRECT => Some(&STEPS_NOT_W_AI),
            mode::POSTINCREMENT => Some(&STEPS_NOT_W_PI),
            mode::PREDECREMENT => Some(&STEPS_NOT_W_PD),
            mode::DISPLACEMENT => Some(&STEPS_NOT_W_D16),
            mode::INDEXED => Some(&STEPS_NOT_W_IDX),
            mode::EXTENDED => match reg {
                extended_mode::ABSOLUTE_WORD => Some(&STEPS_NOT_W_ABSW),
                extended_mode::ABSOLUTE_LONG => Some(&STEPS_NOT_W_ABSL),
                _ => None,
            },
            _ => None,
        },
        operand_size::LONG => match mode {
            mode::DATA_REGISTER => Some(&STEPS_NOT_L_DN),
            mode::INDIRECT => Some(&STEPS_NOT_L_AI),
            mode::POSTINCREMENT => Some(&STEPS_NOT_L_PI),
            mode::PREDECREMENT => Some(&STEPS_NOT_L_PD),
            mode::DISPLACEMENT => Some(&STEPS_NOT_L_D16),
            mode::INDEXED => Some(&STEPS_NOT_L_IDX),
            mode::EXTENDED => match reg {
                extended_mode::ABSOLUTE_WORD => Some(&STEPS_NOT_L_ABSW),
                extended_mode::ABSOLUTE_LONG => Some(&STEPS_NOT_L_ABSL),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
