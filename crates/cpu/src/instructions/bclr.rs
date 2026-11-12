//! BCLR (Bit Clear) instruction handlers and operation logic
//!
//! Tests and clears a single bit in a Data Register (modulo 32) or memory operand (modulo 8).
//! Updates Z flag (set if bit was 0 prior to clearing, cleared if bit was 1). Other condition codes unaffected.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::encoding::{extended_mode, mode};
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

const CLEAR_LOW_WORD_CLOCKS: u16 = 4;
const CLEAR_HIGH_WORD_CLOCKS: u16 = 6;

// ============================================================================
// Leaf ALU BCLR Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn bclr_l(state: &mut CpuState, bit_num: u32, val: u32) -> u32 {
    let bit_idx = bit_num & (u32::BITS - 1);
    let mask = 1 << bit_idx;
    state.set_ccr_z_only((val & mask) == 0);
    val & !mask
}

#[inline(always)]
pub fn bclr_b(state: &mut CpuState, bit_num: u32, val: u8) -> u8 {
    let bit_idx = bit_num & (u8::BITS - 1);
    let mask = 1 << bit_idx;
    state.set_ccr_z_only((val & mask) == 0);
    val & !mask
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_bclr_l_dyn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let bit_num = state.d_long(reg_src as usize);
    let val = state.d_long(reg_dst as usize);
    let res = bclr_l(state, bit_num, val);
    state.set_d_long(reg_dst as usize, res);
    // Modifying the upper register word takes one additional internal CCK.
    state.micro_mut().clocks_remaining = if bit_num & super::BIT_REGISTER_UPPER_WORD_SELECT == 0 {
        CLEAR_LOW_WORD_CLOCKS
    } else {
        CLEAR_HIGH_WORD_CLOCKS
    };
}

pub fn alu_bclr_b_dyn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let bit_num = state.d_long(reg_src as usize);
    let val = (state.micro().destination & 0xFF) as u8;
    let res = bclr_b(state, bit_num, val);
    state.micro_mut().destination = (state.micro().destination & !0xFF) | (res as u32);
}

pub fn alu_bclr_l_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let bit_num = state.micro().source;
    let val = state.d_long(reg_dst as usize);
    let res = bclr_l(state, bit_num, val);
    state.set_d_long(reg_dst as usize, res);
    // Modifying the upper register word takes one additional internal CCK.
    state.micro_mut().clocks_remaining = if bit_num & super::BIT_REGISTER_UPPER_WORD_SELECT == 0 {
        CLEAR_LOW_WORD_CLOCKS
    } else {
        CLEAR_HIGH_WORD_CLOCKS
    };
}

pub fn alu_bclr_b_imm_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let bit_num = state.micro().source;
    let val = (state.micro().destination & 0xFF) as u8;
    let res = bclr_b(state, bit_num, val);
    state.micro_mut().destination = (state.micro().destination & !0xFF) | (res as u32);
}

// ============================================================================
// Immediate Bit Latching Callbacks
// ============================================================================

pub fn latch_bit_imm(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let imm = (state.prefetch() & 0xFF) as u32;
    state.micro_mut().source = imm;
}

pub fn latch_bit_imm_calc_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let imm = (state.prefetch() & 0xFF) as u32;
    state.micro_mut().source = imm;
    ea::ea_calc_dst_ai(state, 0, reg_dst);
}

pub fn latch_bit_imm_calc_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let imm = (state.prefetch() & 0xFF) as u32;
    state.micro_mut().source = imm;
    ea::ea_calc_dst_pi_b(state, 0, reg_dst);
}

// ============================================================================
// Static Micro-Step Slices: Dynamic BCLR Dn, <ea>
// ============================================================================

pub static STEPS_BCLR_DYN_DN: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_bclr_l_dyn_dn),
];

pub static STEPS_BCLR_DYN_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_bclr_b_dyn_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_BCLR_DYN_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pi_b),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_bclr_b_dyn_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_BCLR_DYN_PD: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_bclr_b_dyn_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_BCLR_DYN_D16_AN: [MicroStep; 8] = [
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
        alu_fn: Some(alu_bclr_b_dyn_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_BCLR_DYN_IDX_AN: [MicroStep; 9] = [
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
        alu_fn: Some(alu_bclr_b_dyn_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_BCLR_DYN_ABSW: [MicroStep; 8] = [
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
        alu_fn: Some(alu_bclr_b_dyn_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_BCLR_DYN_ABSL: [MicroStep; 10] = [
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
        alu_fn: Some(alu_bclr_b_dyn_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

// ============================================================================
// Static Micro-Step Slices: Static BCLR #imm, <ea>
// ============================================================================

pub static STEPS_BCLR_STAT_DN: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    MicroStep::alu(alu_bclr_l_imm_dn),
];

pub static STEPS_BCLR_STAT_AI: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm_calc_ai),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_bclr_b_imm_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_BCLR_STAT_PI: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm_calc_pi),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_bclr_b_imm_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_BCLR_STAT_PD: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_bclr_b_imm_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_BCLR_STAT_D16_AN: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
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
        alu_fn: Some(alu_bclr_b_imm_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_BCLR_STAT_IDX_AN: [MicroStep; 11] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
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
        alu_fn: Some(alu_bclr_b_imm_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_BCLR_STAT_ABSW: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
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
        alu_fn: Some(alu_bclr_b_imm_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_BCLR_STAT_ABSL: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::FETCH_EXT_FINISH,
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
        alu_fn: Some(alu_bclr_b_imm_mem),
        base_clocks: MicroStep::CCK_CLOCKS,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

// ============================================================================
// Instruction Decoder Callback
// ============================================================================

pub const fn decode_bclr_dyn_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        mode::DATA_REGISTER => Some(&STEPS_BCLR_DYN_DN),
        mode::INDIRECT => Some(&STEPS_BCLR_DYN_AI),
        mode::POSTINCREMENT => Some(&STEPS_BCLR_DYN_PI),
        mode::PREDECREMENT => Some(&STEPS_BCLR_DYN_PD),
        mode::DISPLACEMENT => Some(&STEPS_BCLR_DYN_D16_AN),
        mode::INDEXED => Some(&STEPS_BCLR_DYN_IDX_AN),
        mode::EXTENDED => match reg {
            extended_mode::ABSOLUTE_WORD => Some(&STEPS_BCLR_DYN_ABSW),
            extended_mode::ABSOLUTE_LONG => Some(&STEPS_BCLR_DYN_ABSL),
            _ => None,
        },
        _ => None,
    }
}

pub const fn decode_bclr_stat_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        mode::DATA_REGISTER => Some(&STEPS_BCLR_STAT_DN),
        mode::INDIRECT => Some(&STEPS_BCLR_STAT_AI),
        mode::POSTINCREMENT => Some(&STEPS_BCLR_STAT_PI),
        mode::PREDECREMENT => Some(&STEPS_BCLR_STAT_PD),
        mode::DISPLACEMENT => Some(&STEPS_BCLR_STAT_D16_AN),
        mode::INDEXED => Some(&STEPS_BCLR_STAT_IDX_AN),
        mode::EXTENDED => match reg {
            extended_mode::ABSOLUTE_WORD => Some(&STEPS_BCLR_STAT_ABSW),
            extended_mode::ABSOLUTE_LONG => Some(&STEPS_BCLR_STAT_ABSL),
            _ => None,
        },
        _ => None,
    }
}
