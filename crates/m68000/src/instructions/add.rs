//! M68000 ADD Instruction (`ADD <ea>, Dn` and `ADD Dn, <ea>`)
//!
//! Provides cycle-exact Color Clock micro-step execution slices and
//! branchless ALU arithmetic functions (`add_b`, `add_w`, `add_l`, `execute_add`).

use crate::micro::common;
use crate::micro::ea;
use crate::core::Cpu;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Core Leaf ALU Addition Functions (Branchless & Cycle-Exact CCR)
// ============================================================================

#[inline(always)]
pub fn add_b(state: &mut CpuState, s: u8, d: u8) -> u8 {
    let (res, c) = d.overflowing_add(s);
    let v = ((!(s ^ d) & (d ^ res)) & 0x80) != 0;
    let n = (res & 0x80) != 0;
    let z = res == 0;
    state.set_ccr_xnzvc(c, n, z, v, c);
    res
}

#[inline(always)]
pub fn add_w(state: &mut CpuState, s: u16, d: u16) -> u16 {
    let (res, c) = d.overflowing_add(s);
    let v = ((!(s ^ d) & (d ^ res)) & 0x8000) != 0;
    let n = (res & 0x8000) != 0;
    let z = res == 0;
    state.set_ccr_xnzvc(c, n, z, v, c);
    res
}

#[inline(always)]
pub fn add_l(state: &mut CpuState, s: u32, d: u32) -> u32 {
    let (res, c) = d.overflowing_add(s);
    let v = ((!(s ^ d) & (d ^ res)) & 0x8000_0000) != 0;
    let n = (res & 0x8000_0000) != 0;
    let z = res == 0;
    state.set_ccr_xnzvc(c, n, z, v, c);
    res
}

// ============================================================================
// Micro-Step ALU Callbacks (AluFn)
// ============================================================================

pub fn alu_add_b_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = add_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_add_w_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = add_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_add_w_an_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.read_a(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = add_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_add_l_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = add_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_add_l_an_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.read_a(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = add_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_add_b_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.source & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = add_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_add_w_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.source & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = add_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_add_l_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.source;
    let d = state.d_long(reg_dst as usize);
    let res = add_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_add_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = add_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_add_w_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.prefetch[0];
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = add_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_add_b_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.micro.destination & 0xFF) as u8;
    let res = add_b(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFF) | (res as u32);
    state.micro.write_buffer = res as u32;
}

pub fn alu_add_w_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.micro.destination & 0xFFFF) as u16;
    let res = add_w(state, s, d);
    state.micro.destination = (state.micro.destination & !0xFFFF) | (res as u32);
    state.micro.write_buffer = res as u32;
}

pub fn alu_add_l_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.micro.destination;
    let res = add_l(state, s, d);
    state.micro.destination = res;
    state.micro.write_buffer = res;
}

// ============================================================================
// Static Micro-Step Slices: ADD <ea>, Dn
// ============================================================================

// Byte: <ea>, Dn
pub static STEPS_ADD_B_DN_DN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_b_dn_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_B_AI_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 2 },
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_B_PI_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 2 },
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_B_PD_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_byte, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2 },
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_B_D16_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_B_IDX_DN: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_B_ABSW_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_B_ABSL_DN: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_B_PCD16_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_B_PCIDX_DN: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_B_IMM_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(alu_add_b_imm_dn), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];

// Word: <ea>, Dn
pub static STEPS_ADD_W_DN_DN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_w_dn_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_W_AN_DN: [MicroStep; 2] = [
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_w_an_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_W_AI_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_word, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 2 },
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_W_PI_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_word, alu_fn: Some(ea::ea_calc_src_pi_w), base_clocks: 2 },
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_W_PD_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_word, alu_fn: Some(ea::ea_calc_src_pd_w), base_clocks: 2 },
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_W_D16_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_W_IDX_DN: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_W_ABSW_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_W_ABSL_DN: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_W_PCD16_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_W_PCIDX_DN: [MicroStep; 7] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_W_IMM_DN: [MicroStep; 4] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(alu_add_w_imm_dn), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];

// Long: <ea>, Dn
pub static STEPS_ADD_L_DN_DN: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_l_dn_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_L_AN_DN: [MicroStep; 3] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: None, base_clocks: 4 },
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_l_an_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_L_AI_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_long_high, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 2 },
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_L_PI_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_long_high, alu_fn: Some(ea::ea_calc_src_pi_l), base_clocks: 2 },
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_L_PD_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_bus_read_src_long_high, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2 },
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_L_D16_DN: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_L_IDX_DN: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2 },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_L_ABSW_DN: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_L_ABSL_DN: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_L_PCD16_DN: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_L_PCIDX_DN: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2 },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_SRC_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];
pub static STEPS_ADD_L_IMM_DN: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_next_read, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 2 },
    common::PREFETCH_NEXT_RETIRE,
];

// ============================================================================
// Static Micro-Step Slices: ADD Dn, <ea> (RMW Class 0)
// ============================================================================

// Byte RMW: Dn, <ea>
pub static STEPS_ADD_B_DN_AI: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_bus_read_dst_byte, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 2 },
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_ADD_B_DN_PI: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_bus_read_dst_byte, alu_fn: Some(ea::ea_calc_dst_pi_b), base_clocks: 2 },
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_ADD_B_DN_PD: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_bus_read_dst_byte, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 2 },
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_ADD_B_DN_D16: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_ADD_B_DN_IDX: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_ADD_B_DN_ABSW: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];
pub static STEPS_ADD_B_DN_ABSL: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::READ_BYTE_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE_RETIRE,
];

// Word RMW: Dn, <ea>
pub static STEPS_ADD_W_DN_AI: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_bus_read_dst_word, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 2 },
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];
pub static STEPS_ADD_W_DN_PI: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_bus_read_dst_word, alu_fn: Some(ea::ea_calc_dst_pi_w), base_clocks: 2 },
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];
pub static STEPS_ADD_W_DN_PD: [MicroStep; 6] = [
    MicroStep { step_fn: Cpu::step_bus_read_dst_word, alu_fn: Some(ea::ea_calc_dst_pd_w), base_clocks: 2 },
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];
pub static STEPS_ADD_W_DN_D16: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];
pub static STEPS_ADD_W_DN_IDX: [MicroStep; 9] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];
pub static STEPS_ADD_W_DN_ABSW: [MicroStep; 8] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];
pub static STEPS_ADD_W_DN_ABSL: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD_RETIRE,
];

// Long RMW: Dn, <ea>
pub static STEPS_ADD_L_DN_AI: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_bus_read_dst_long_high, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 2 },
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];
pub static STEPS_ADD_L_DN_PI: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_bus_read_dst_long_high, alu_fn: Some(ea::ea_calc_dst_pi_l), base_clocks: 2 },
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];
pub static STEPS_ADD_L_DN_PD: [MicroStep; 10] = [
    MicroStep { step_fn: Cpu::step_bus_read_dst_long_high, alu_fn: Some(ea::ea_calc_dst_pd_l), base_clocks: 2 },
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];
pub static STEPS_ADD_L_DN_D16: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];
pub static STEPS_ADD_L_DN_IDX: [MicroStep; 13] = [
    MicroStep { step_fn: Cpu::step_alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2 },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];
pub static STEPS_ADD_L_DN_ABSW: [MicroStep; 12] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absw), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];
pub static STEPS_ADD_L_DN_ABSL: [MicroStep; 14] = [
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    MicroStep { step_fn: Cpu::step_fetch_extension_read, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 2 },
    common::FETCH_EXT_FINISH,
    common::READ_DST_LONG_HIGH,
    common::READ_WORD_FINISH,
    common::READ_DST_LONG_LOW,
    common::READ_WORD_FINISH,
    MicroStep { step_fn: Cpu::step_prefetch_scratch_read, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 2 },
    common::PREFETCH_SCRATCH_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW_RETIRE,
];

// ============================================================================
// Opcode Descriptor Decoder Helper for ADD
// ============================================================================

/// Maps an ADD opcode's bit fields to its static micro-step sequence
pub const fn decode_add_steps(dir: u8, size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    if dir == 0 {
        // <ea>, Dn
        match size {
            0 => match mode {
                0 => Some(&STEPS_ADD_B_DN_DN),
                2 => Some(&STEPS_ADD_B_AI_DN),
                3 => Some(&STEPS_ADD_B_PI_DN),
                4 => Some(&STEPS_ADD_B_PD_DN),
                5 => Some(&STEPS_ADD_B_D16_DN),
                6 => Some(&STEPS_ADD_B_IDX_DN),
                7 => match reg {
                    0 => Some(&STEPS_ADD_B_ABSW_DN),
                    1 => Some(&STEPS_ADD_B_ABSL_DN),
                    2 => Some(&STEPS_ADD_B_PCD16_DN),
                    3 => Some(&STEPS_ADD_B_PCIDX_DN),
                    4 => Some(&STEPS_ADD_B_IMM_DN),
                    _ => None,
                },
                _ => None,
            },
            1 => match mode {
                0 => Some(&STEPS_ADD_W_DN_DN),
                1 => Some(&STEPS_ADD_W_AN_DN),
                2 => Some(&STEPS_ADD_W_AI_DN),
                3 => Some(&STEPS_ADD_W_PI_DN),
                4 => Some(&STEPS_ADD_W_PD_DN),
                5 => Some(&STEPS_ADD_W_D16_DN),
                6 => Some(&STEPS_ADD_W_IDX_DN),
                7 => match reg {
                    0 => Some(&STEPS_ADD_W_ABSW_DN),
                    1 => Some(&STEPS_ADD_W_ABSL_DN),
                    2 => Some(&STEPS_ADD_W_PCD16_DN),
                    3 => Some(&STEPS_ADD_W_PCIDX_DN),
                    4 => Some(&STEPS_ADD_W_IMM_DN),
                    _ => None,
                },
                _ => None,
            },
            2 => match mode {
                0 => Some(&STEPS_ADD_L_DN_DN),
                1 => Some(&STEPS_ADD_L_AN_DN),
                2 => Some(&STEPS_ADD_L_AI_DN),
                3 => Some(&STEPS_ADD_L_PI_DN),
                4 => Some(&STEPS_ADD_L_PD_DN),
                5 => Some(&STEPS_ADD_L_D16_DN),
                6 => Some(&STEPS_ADD_L_IDX_DN),
                7 => match reg {
                    0 => Some(&STEPS_ADD_L_ABSW_DN),
                    1 => Some(&STEPS_ADD_L_ABSL_DN),
                    2 => Some(&STEPS_ADD_L_PCD16_DN),
                    3 => Some(&STEPS_ADD_L_PCIDX_DN),
                    4 => Some(&STEPS_ADD_L_IMM_DN),
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
                2 => Some(&STEPS_ADD_B_DN_AI),
                3 => Some(&STEPS_ADD_B_DN_PI),
                4 => Some(&STEPS_ADD_B_DN_PD),
                5 => Some(&STEPS_ADD_B_DN_D16),
                6 => Some(&STEPS_ADD_B_DN_IDX),
                7 => match reg {
                    0 => Some(&STEPS_ADD_B_DN_ABSW),
                    1 => Some(&STEPS_ADD_B_DN_ABSL),
                    _ => None,
                },
                _ => None,
            },
            1 => match mode {
                2 => Some(&STEPS_ADD_W_DN_AI),
                3 => Some(&STEPS_ADD_W_DN_PI),
                4 => Some(&STEPS_ADD_W_DN_PD),
                5 => Some(&STEPS_ADD_W_DN_D16),
                6 => Some(&STEPS_ADD_W_DN_IDX),
                7 => match reg {
                    0 => Some(&STEPS_ADD_W_DN_ABSW),
                    1 => Some(&STEPS_ADD_W_DN_ABSL),
                    _ => None,
                },
                _ => None,
            },
            2 => match mode {
                2 => Some(&STEPS_ADD_L_DN_AI),
                3 => Some(&STEPS_ADD_L_DN_PI),
                4 => Some(&STEPS_ADD_L_DN_PD),
                5 => Some(&STEPS_ADD_L_DN_D16),
                6 => Some(&STEPS_ADD_L_DN_IDX),
                7 => match reg {
                    0 => Some(&STEPS_ADD_L_DN_ABSW),
                    1 => Some(&STEPS_ADD_L_DN_ABSL),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }
}

