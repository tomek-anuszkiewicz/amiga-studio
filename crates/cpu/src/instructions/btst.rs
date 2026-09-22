//! BTST (Bit Test) instruction handlers and operation logic
//!
//! Tests a single bit in a Data Register (modulo 32) or memory operand (modulo 8).
//! Updates Z flag (set if bit was 0, cleared if bit was 1). Other condition codes and destination contents are unaffected.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Leaf ALU BTST Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn btst_l(state: &mut CpuState, bit_num: u32, val: u32) {
    let bit_idx = bit_num & 31;
    let mask = 1 << bit_idx;
    state.set_ccr_z_only((val & mask) == 0);
}

#[inline(always)]
pub fn btst_b(state: &mut CpuState, bit_num: u32, val: u8) {
    let bit_idx = bit_num & 7;
    let mask = 1 << bit_idx;
    state.set_ccr_z_only((val & mask) == 0);
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_btst_l_dyn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let bit_num = state.d_long(reg_src as usize);
    let val = state.d_long(reg_dst as usize);
    btst_l(state, bit_num, val);
}

pub fn alu_btst_b_dyn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let bit_num = state.d_long(reg_src as usize);
    let val = (state.micro.destination & 0xFF) as u8;
    btst_b(state, bit_num, val);
}

pub fn alu_btst_b_dyn_imm(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let bit_num = state.d_long(reg_src as usize);
    let val = (state.prefetch & 0xFF) as u8;
    btst_b(state, bit_num, val);
}

pub fn alu_btst_l_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let bit_num = state.micro.source;
    let val = state.d_long(reg_dst as usize);
    btst_l(state, bit_num, val);
}

pub fn alu_btst_b_imm_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let bit_num = state.micro.source;
    let val = (state.micro.destination & 0xFF) as u8;
    btst_b(state, bit_num, val);
}

// ============================================================================
// Immediate Bit Latching Callbacks
// ============================================================================

pub fn latch_bit_imm(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let imm = (state.prefetch & 0xFF) as u32;
    state.micro.source = imm;
}

pub fn latch_bit_imm_calc_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let imm = (state.prefetch & 0xFF) as u32;
    state.micro.source = imm;
    ea::ea_calc_dst_ai(state, 0, reg_dst);
}

pub fn latch_bit_imm_calc_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let imm = (state.prefetch & 0xFF) as u32;
    state.micro.source = imm;
    ea::ea_calc_dst_pi_b(state, 0, reg_dst);
}

// ============================================================================
// Static Micro-Step Slices: Dynamic BTST Dn, <ea>
// ============================================================================

pub static STEPS_BTST_DYN_DN: [MicroStep; 3] = [
    common::ALU_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_btst_l_dyn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_DYN_AI: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_btst_b_dyn_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_DYN_PI: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pi_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_btst_b_dyn_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_DYN_PD: [MicroStep; 5] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_btst_b_dyn_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_DYN_D16_AN: [MicroStep; 6] = [
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
        alu_fn: Some(alu_btst_b_dyn_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_DYN_IDX_AN: [MicroStep; 7] = [
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
        alu_fn: Some(alu_btst_b_dyn_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_DYN_ABSW: [MicroStep; 6] = [
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
        alu_fn: Some(alu_btst_b_dyn_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_DYN_ABSL: [MicroStep; 8] = [
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
        alu_fn: Some(alu_btst_b_dyn_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_DYN_D16_PC: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_btst_b_dyn_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_DYN_IDX_PC: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_btst_b_dyn_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_DYN_IMM: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_btst_b_dyn_imm),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// Static Micro-Step Slices: Static BTST #imm, <ea>
// ============================================================================

pub static STEPS_BTST_STAT_DN: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::ALU_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_btst_l_imm_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_STAT_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm_calc_ai),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_btst_b_imm_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_STAT_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm_calc_pi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_btst_b_imm_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_STAT_PD: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_btst_b_imm_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_STAT_D16_AN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
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
        alu_fn: Some(alu_btst_b_imm_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_STAT_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
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
        alu_fn: Some(alu_btst_b_imm_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_STAT_ABSW: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
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
        alu_fn: Some(alu_btst_b_imm_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_STAT_ABSL: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
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
        alu_fn: Some(alu_btst_b_imm_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_STAT_D16_PC: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_btst_b_imm_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_BTST_STAT_IDX_PC: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(latch_bit_imm),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_btst_b_imm_mem),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

// ============================================================================
// Instruction Decoder Callback
// ============================================================================

pub const fn decode_btst_dyn_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(&STEPS_BTST_DYN_DN),
        2 => Some(&STEPS_BTST_DYN_AI),
        3 => Some(&STEPS_BTST_DYN_PI),
        4 => Some(&STEPS_BTST_DYN_PD),
        5 => Some(&STEPS_BTST_DYN_D16_AN),
        6 => Some(&STEPS_BTST_DYN_IDX_AN),
        7 => match reg {
            0 => Some(&STEPS_BTST_DYN_ABSW),
            1 => Some(&STEPS_BTST_DYN_ABSL),
            2 => Some(&STEPS_BTST_DYN_D16_PC),
            3 => Some(&STEPS_BTST_DYN_IDX_PC),
            4 => Some(&STEPS_BTST_DYN_IMM),
            _ => None,
        },
        _ => None,
    }
}

pub const fn decode_btst_stat_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(&STEPS_BTST_STAT_DN),
        2 => Some(&STEPS_BTST_STAT_AI),
        3 => Some(&STEPS_BTST_STAT_PI),
        4 => Some(&STEPS_BTST_STAT_PD),
        5 => Some(&STEPS_BTST_STAT_D16_AN),
        6 => Some(&STEPS_BTST_STAT_IDX_AN),
        7 => match reg {
            0 => Some(&STEPS_BTST_STAT_ABSW),
            1 => Some(&STEPS_BTST_STAT_ABSL),
            2 => Some(&STEPS_BTST_STAT_D16_PC),
            3 => Some(&STEPS_BTST_STAT_IDX_PC),
            _ => None,
        },
        _ => None,
    }
}
