//! M68000 SUB Instruction (`SUB <ea>, Dn` and `SUB Dn, <ea>`)
//!
//! Provides cycle-exact Color Clock micro-step execution slices and
//! branchless ALU arithmetic functions (`sub_b`, `sub_w`, `sub_l`).

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Core Leaf ALU Subtraction Functions (Branchless & Cycle-Exact CCR)
// ============================================================================

#[inline(always)]
pub fn sub_b(state: &mut CpuState, s: u8, d: u8) -> u8 {
    let (res, c) = d.overflowing_sub(s);
    let v = (((s ^ d) & (d ^ res)) & 0x80) != 0;
    let n = (res & 0x80) != 0;
    let z = res == 0;
    state.set_ccr_xnzvc(c, n, z, v, c);
    res
}

#[inline(always)]
pub fn sub_w(state: &mut CpuState, s: u16, d: u16) -> u16 {
    let (res, c) = d.overflowing_sub(s);
    let v = (((s ^ d) & (d ^ res)) & 0x8000) != 0;
    let n = (res & 0x8000) != 0;
    let z = res == 0;
    state.set_ccr_xnzvc(c, n, z, v, c);
    res
}

#[inline(always)]
pub fn sub_l(state: &mut CpuState, s: u32, d: u32) -> u32 {
    let (res, c) = d.overflowing_sub(s);
    let v = (((s ^ d) & (d ^ res)) & 0x8000_0000) != 0;
    let n = (res & 0x8000_0000) != 0;
    let z = res == 0;
    state.set_ccr_xnzvc(c, n, z, v, c);
    res
}

// ============================================================================
// Micro-Step ALU Callbacks (AluFn)
// ============================================================================

pub fn alu_sub_b_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = sub_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_sub_w_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = sub_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_sub_w_an_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.read_a(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = sub_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_sub_l_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = sub_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_sub_l_an_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.read_a(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = sub_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_sub_b_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.source & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = sub_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_sub_w_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.source & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = sub_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_sub_l_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.source;
    let d = state.d_long(reg_dst as usize);
    let res = sub_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_sub_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = sub_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_sub_w_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.prefetch[0];
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = sub_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_sub_b_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.micro.destination & 0xFF) as u8;
    let res = sub_b(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFF) | (res as u32);
}

pub fn alu_sub_w_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.micro.destination & 0xFFFF) as u16;
    let res = sub_w(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFFFF) | (res as u32);
}

pub fn alu_sub_l_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.micro.destination;
    let res = sub_l(state, s, d);
    state.micro.destination = res;
}

// ============================================================================
// Byte: <ea>, Dn
// ============================================================================

pub static STEPS_SUB_B_DN_DN: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_dn_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_B_AI_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_B_PI_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_pi_b),
        base_clocks: 2,
    },
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_B_PD_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_byte),
        alu_fn: Some(ea::ea_calc_src_pd_b),
        base_clocks: 2,
    },
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_B_D16_AN_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_B_IDX_AN_DN: [MicroStep; 7] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_B_ABSW_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_B_ABSL_DN: [MicroStep; 8] = [
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
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_B_D16_PC_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_B_IDX_PC_DN: [MicroStep; 7] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_B_IMM_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_sub_b_imm_dn),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];

// ============================================================================
// Word: <ea>, Dn
// ============================================================================

pub static STEPS_SUB_W_DN_DN: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_dn_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_W_AN_DN: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_an_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_W_AI_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_W_PI_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_W_PD_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_W_D16_AN_DN: [MicroStep; 6] = [
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
        alu_fn: Some(alu_sub_w_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_W_IDX_AN_DN: [MicroStep; 7] = [
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
        alu_fn: Some(alu_sub_w_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_W_ABSW_DN: [MicroStep; 6] = [
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
        alu_fn: Some(alu_sub_w_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_W_ABSL_DN: [MicroStep; 8] = [
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
        alu_fn: Some(alu_sub_w_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_W_D16_PC_DN: [MicroStep; 6] = [
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
        alu_fn: Some(alu_sub_w_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_W_IDX_PC_DN: [MicroStep; 7] = [
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
        alu_fn: Some(alu_sub_w_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_W_IMM_DN: [MicroStep; 4] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_sub_w_imm_dn),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];

// ============================================================================
// Long: <ea>, Dn
// ============================================================================

pub static STEPS_SUB_L_DN_DN: [MicroStep; 3] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 4,
    },
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_l_dn_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_L_AN_DN: [MicroStep; 3] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 4,
    },
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_l_an_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_L_AI_DN: [MicroStep; 6] = [
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
        alu_fn: Some(alu_sub_l_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_L_PI_DN: [MicroStep; 6] = [
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
        alu_fn: Some(alu_sub_l_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_L_PD_DN: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(ea::ea_calc_src_pd_l),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_l_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_L_D16_AN_DN: [MicroStep; 8] = [
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
        alu_fn: Some(alu_sub_l_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_L_IDX_AN_DN: [MicroStep; 9] = [
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
        alu_fn: Some(alu_sub_l_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_L_ABSW_DN: [MicroStep; 8] = [
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
        alu_fn: Some(alu_sub_l_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_L_ABSL_DN: [MicroStep; 10] = [
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
        alu_fn: Some(alu_sub_l_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_L_D16_PC_DN: [MicroStep; 8] = [
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
        alu_fn: Some(alu_sub_l_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_L_IDX_PC_DN: [MicroStep; 9] = [
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
        alu_fn: Some(alu_sub_l_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

pub static STEPS_SUB_L_IMM_DN: [MicroStep; 6] = [
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
        alu_fn: Some(alu_sub_l_mem_dn),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

// ============================================================================
// Byte RMW: Dn, <ea>
// ============================================================================

pub static STEPS_SUB_B_DN_AI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];

pub static STEPS_SUB_B_DN_PI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pi_b),
        base_clocks: 2,
    },
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];

pub static STEPS_SUB_B_DN_PD: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];

pub static STEPS_SUB_B_DN_D16_AN: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];

pub static STEPS_SUB_B_DN_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];

pub static STEPS_SUB_B_DN_ABSW: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];

pub static STEPS_SUB_B_DN_ABSL: [MicroStep; 10] = [
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
    common::READ_BYTE_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_b_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];

// ============================================================================
// Word RMW: Dn, <ea>
// ============================================================================

pub static STEPS_SUB_W_DN_AI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];

pub static STEPS_SUB_W_DN_PI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pi_w),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];

pub static STEPS_SUB_W_DN_PD: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];

pub static STEPS_SUB_W_DN_D16_AN: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];

pub static STEPS_SUB_W_DN_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];

pub static STEPS_SUB_W_DN_ABSW: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];

pub static STEPS_SUB_W_DN_ABSL: [MicroStep; 10] = [
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
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_w_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];

// ============================================================================
// Long RMW: Dn, <ea>
// ============================================================================

pub static STEPS_SUB_L_DN_AI: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];

pub static STEPS_SUB_L_DN_PI: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: Some(ea::ea_calc_dst_pi_l),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];

pub static STEPS_SUB_L_DN_PD: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_long_high),
        alu_fn: Some(ea::ea_calc_dst_pd_l),
        base_clocks: 2,
    },
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];

pub static STEPS_SUB_L_DN_D16_AN: [MicroStep; 12] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];

pub static STEPS_SUB_L_DN_IDX_AN: [MicroStep; 13] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];

pub static STEPS_SUB_L_DN_ABSW: [MicroStep; 12] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];

pub static STEPS_SUB_L_DN_ABSL: [MicroStep; 14] = [
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
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_sub_l_dn_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];

// ============================================================================
// Opcode Descriptor Decoder Helper for SUB
// ============================================================================

pub const fn decode_sub_steps(
    dir: u8,
    size: u8,
    mode: u8,
    reg: u8,
) -> Option<&'static [MicroStep]> {
    if dir == 0 {
        // <ea>, Dn
        match size {
            0 => match mode {
                0 => Some(&STEPS_SUB_B_DN_DN),
                2 => Some(&STEPS_SUB_B_AI_DN),
                3 => Some(&STEPS_SUB_B_PI_DN),
                4 => Some(&STEPS_SUB_B_PD_DN),
                5 => Some(&STEPS_SUB_B_D16_AN_DN),
                6 => Some(&STEPS_SUB_B_IDX_AN_DN),
                7 => match reg {
                    0 => Some(&STEPS_SUB_B_ABSW_DN),
                    1 => Some(&STEPS_SUB_B_ABSL_DN),
                    2 => Some(&STEPS_SUB_B_D16_PC_DN),
                    3 => Some(&STEPS_SUB_B_IDX_PC_DN),
                    4 => Some(&STEPS_SUB_B_IMM_DN),
                    _ => None,
                },
                _ => None,
            },
            1 => match mode {
                0 => Some(&STEPS_SUB_W_DN_DN),
                1 => Some(&STEPS_SUB_W_AN_DN),
                2 => Some(&STEPS_SUB_W_AI_DN),
                3 => Some(&STEPS_SUB_W_PI_DN),
                4 => Some(&STEPS_SUB_W_PD_DN),
                5 => Some(&STEPS_SUB_W_D16_AN_DN),
                6 => Some(&STEPS_SUB_W_IDX_AN_DN),
                7 => match reg {
                    0 => Some(&STEPS_SUB_W_ABSW_DN),
                    1 => Some(&STEPS_SUB_W_ABSL_DN),
                    2 => Some(&STEPS_SUB_W_D16_PC_DN),
                    3 => Some(&STEPS_SUB_W_IDX_PC_DN),
                    4 => Some(&STEPS_SUB_W_IMM_DN),
                    _ => None,
                },
                _ => None,
            },
            2 => match mode {
                0 => Some(&STEPS_SUB_L_DN_DN),
                1 => Some(&STEPS_SUB_L_AN_DN),
                2 => Some(&STEPS_SUB_L_AI_DN),
                3 => Some(&STEPS_SUB_L_PI_DN),
                4 => Some(&STEPS_SUB_L_PD_DN),
                5 => Some(&STEPS_SUB_L_D16_AN_DN),
                6 => Some(&STEPS_SUB_L_IDX_AN_DN),
                7 => match reg {
                    0 => Some(&STEPS_SUB_L_ABSW_DN),
                    1 => Some(&STEPS_SUB_L_ABSL_DN),
                    2 => Some(&STEPS_SUB_L_D16_PC_DN),
                    3 => Some(&STEPS_SUB_L_IDX_PC_DN),
                    4 => Some(&STEPS_SUB_L_IMM_DN),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    } else {
        // Dn, <ea>
        match size {
            0 => match mode {
                2 => Some(&STEPS_SUB_B_DN_AI),
                3 => Some(&STEPS_SUB_B_DN_PI),
                4 => Some(&STEPS_SUB_B_DN_PD),
                5 => Some(&STEPS_SUB_B_DN_D16_AN),
                6 => Some(&STEPS_SUB_B_DN_IDX_AN),
                7 => match reg {
                    0 => Some(&STEPS_SUB_B_DN_ABSW),
                    1 => Some(&STEPS_SUB_B_DN_ABSL),
                    _ => None,
                },
                _ => None,
            },
            1 => match mode {
                2 => Some(&STEPS_SUB_W_DN_AI),
                3 => Some(&STEPS_SUB_W_DN_PI),
                4 => Some(&STEPS_SUB_W_DN_PD),
                5 => Some(&STEPS_SUB_W_DN_D16_AN),
                6 => Some(&STEPS_SUB_W_DN_IDX_AN),
                7 => match reg {
                    0 => Some(&STEPS_SUB_W_DN_ABSW),
                    1 => Some(&STEPS_SUB_W_DN_ABSL),
                    _ => None,
                },
                _ => None,
            },
            2 => match mode {
                2 => Some(&STEPS_SUB_L_DN_AI),
                3 => Some(&STEPS_SUB_L_DN_PI),
                4 => Some(&STEPS_SUB_L_DN_PD),
                5 => Some(&STEPS_SUB_L_DN_D16_AN),
                6 => Some(&STEPS_SUB_L_DN_IDX_AN),
                7 => match reg {
                    0 => Some(&STEPS_SUB_L_DN_ABSW),
                    1 => Some(&STEPS_SUB_L_DN_ABSL),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }
}
