//! M68000 OR Instruction (`OR <ea>, Dn` and `OR Dn, <ea>`)
//!
//! Performs bitwise OR between a source operand and a destination data register or memory location.
//! CCR flags: N = MSB(res), Z = (res == 0), V = 0, C = 0, X unaffected.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// Core Leaf ALU OR Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn or_b(state: &mut CpuState, s: u8, d: u8) -> u8 {
    let res = d | s;
    state.set_ccr_nz_clear_vc((res & 0x80) != 0, res == 0);
    res
}

#[inline(always)]
pub fn or_w(state: &mut CpuState, s: u16, d: u16) -> u16 {
    let res = d | s;
    state.set_ccr_nz_clear_vc((res & 0x8000) != 0, res == 0);
    res
}

#[inline(always)]
pub fn or_l(state: &mut CpuState, s: u32, d: u32) -> u32 {
    let res = d | s;
    state.set_ccr_nz_clear_vc((res & 0x8000_0000) != 0, res == 0);
    res
}

// ============================================================================
// Micro-Step ALU Callbacks (AluFn)
// ============================================================================

pub fn alu_or_b_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = or_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_or_w_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = or_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_or_l_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = or_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_or_b_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.source & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = or_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_or_w_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.source & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = or_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_or_l_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.source;
    let d = state.d_long(reg_dst as usize);
    let res = or_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_or_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = or_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_or_w_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.prefetch[0];
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = or_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_or_b_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.micro.destination & 0xFF) as u8;
    let res = or_b(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFF) | (res as u32);
}

pub fn alu_or_w_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.micro.destination & 0xFFFF) as u16;
    let res = or_w(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFFFF) | (res as u32);
}

pub fn alu_or_l_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.micro.destination;
    let res = or_l(state, s, d);
    state.micro.destination = res;
}

// ============================================================================
// Static Micro-Step Slices: OR <ea>, Dn (Byte)
// ============================================================================

pub static STEPS_OR_B_DN_DN: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_AI_DN: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_PI_DN: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_pi_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_PD_DN: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_pd_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_D16_AN_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_IDX_AN_DN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_ABSW_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_ABSL_DN: [MicroStep; 8] = [
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
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_D16_PC_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_IDX_PC_DN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_B_IMM_DN: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
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
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_AI_DN: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_PI_DN: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_PD_DN: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_D16_AN_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_IDX_AN_DN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_ABSW_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_ABSL_DN: [MicroStep; 8] = [
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
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_D16_PC_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_IDX_PC_DN: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_W_IMM_DN: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
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
    common::ALU_IDLE_4CLK,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_dn_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_AI_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_PI_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_PD_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_D16_AN_DN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_IDX_AN_DN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
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
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_ABSW_DN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_ABSL_DN: [MicroStep; 10] = [
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
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_D16_PC_DN: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_IDX_PC_DN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
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
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_mem_dn),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_OR_L_IMM_DN: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_l_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
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
// Static Micro-Step Slices: OR Dn, <ea> (Byte RMW)
// ============================================================================

pub static STEPS_OR_B_DN_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_OR_B_DN_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pi_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_OR_B_DN_PD: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_OR_B_DN_D16_AN: [MicroStep; 8] = [
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
        alu_fn: Some(alu_or_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_OR_B_DN_IDX_AN: [MicroStep; 9] = [
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
        alu_fn: Some(alu_or_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_OR_B_DN_ABSW: [MicroStep; 8] = [
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
        alu_fn: Some(alu_or_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_OR_B_DN_ABSL: [MicroStep; 10] = [
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
        alu_fn: Some(alu_or_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

// ============================================================================
// Static Micro-Step Slices: OR Dn, <ea> (Word RMW)
// ============================================================================

pub static STEPS_OR_W_DN_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_OR_W_DN_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_OR_W_DN_PD: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_OR_W_DN_D16_AN: [MicroStep; 8] = [
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
        alu_fn: Some(alu_or_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_OR_W_DN_IDX_AN: [MicroStep; 9] = [
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
        alu_fn: Some(alu_or_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_OR_W_DN_ABSW: [MicroStep; 8] = [
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
        alu_fn: Some(alu_or_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_OR_W_DN_ABSL: [MicroStep; 10] = [
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
        alu_fn: Some(alu_or_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

// ============================================================================
// Static Micro-Step Slices: OR Dn, <ea> (Long RMW)
// ============================================================================

pub static STEPS_OR_L_DN_AI: [MicroStep; 10] = [
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
        alu_fn: Some(alu_or_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_OR_L_DN_PI: [MicroStep; 10] = [
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
        alu_fn: Some(alu_or_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_OR_L_DN_PD: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: Some(ea::ea_calc_dst_pd_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_OR_L_DN_D16_AN: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_OR_L_DN_IDX_AN: [MicroStep; 13] = [
    MicroStep {
        bus_fn: None,
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
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_OR_L_DN_ABSW: [MicroStep; 12] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

pub static STEPS_OR_L_DN_ABSL: [MicroStep; 14] = [
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
    common::READ_DST_LONG_HIGH,
    common::BUS_READ_IDLE,
    common::READ_DST_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_or_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
];

// ============================================================================
// Decoder: OR Dn, <ea>
// ============================================================================

pub const fn decode_or_dn_ea_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            2 => Some(&STEPS_OR_B_DN_AI),
            3 => Some(&STEPS_OR_B_DN_PI),
            4 => Some(&STEPS_OR_B_DN_PD),
            5 => Some(&STEPS_OR_B_DN_D16_AN),
            6 => Some(&STEPS_OR_B_DN_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_OR_B_DN_ABSW),
                1 => Some(&STEPS_OR_B_DN_ABSL),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            2 => Some(&STEPS_OR_W_DN_AI),
            3 => Some(&STEPS_OR_W_DN_PI),
            4 => Some(&STEPS_OR_W_DN_PD),
            5 => Some(&STEPS_OR_W_DN_D16_AN),
            6 => Some(&STEPS_OR_W_DN_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_OR_W_DN_ABSW),
                1 => Some(&STEPS_OR_W_DN_ABSL),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            2 => Some(&STEPS_OR_L_DN_AI),
            3 => Some(&STEPS_OR_L_DN_PI),
            4 => Some(&STEPS_OR_L_DN_PD),
            5 => Some(&STEPS_OR_L_DN_D16_AN),
            6 => Some(&STEPS_OR_L_DN_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_OR_L_DN_ABSW),
                1 => Some(&STEPS_OR_L_DN_ABSL),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
// ============================================================================
// Opcode Decoder Function
// ============================================================================

pub const fn decode_or_steps(dir: u8, size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    if dir == 0 {
        decode_or_ea_dn_steps(size, mode, reg)
    } else {
        decode_or_dn_ea_steps(size, mode, reg)
    }
}
