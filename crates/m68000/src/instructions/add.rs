//! M68000 ADD Instruction (`ADD <ea>, Dn` and `ADD Dn, <ea>`)
//!
//! Provides cycle-exact Color Clock micro-step execution slices and
//! branchless ALU arithmetic functions (`add_b`, `add_w`, `add_l`, `execute_add`).

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::{flags, MicroAction, MicroStep};
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
    let s = (state.micro.last_read & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = add_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_add_w_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.last_read;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = add_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_add_l_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.scratch[1];
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
    let d = (state.micro.last_read & 0xFF) as u8;
    let res = add_b(state, s, d);
    state.micro.write_buffer = res as u32;
}

pub fn alu_add_w_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = state.micro.last_read;
    let res = add_w(state, s, d);
    state.micro.write_buffer = res as u32;
}

pub fn alu_add_l_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.micro.scratch[1];
    let res = add_l(state, s, d);
    state.micro.write_buffer = res;
}

// ============================================================================
// Static Micro-Step Slices: ADD <ea>, Dn
// ============================================================================

// Byte: <ea>, Dn
pub static STEPS_ADD_B_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_b_dn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_B_AI_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_B_PI_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_B_PD_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_B_D16_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_B_IDX_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_B_ABSW_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_B_ABSL_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_B_PCD16_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_B_PCIDX_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_B_IMM_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_add_b_imm_dn), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// Word: <ea>, Dn
pub static STEPS_ADD_W_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_w_dn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_W_AN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_w_an_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_W_AI_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_W_PI_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_pi_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_W_PD_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_pd_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_W_D16_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_W_IDX_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_W_ABSW_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_W_ABSL_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_W_PCD16_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_W_PCIDX_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_W_IMM_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_add_w_imm_dn), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// Long: <ea>, Dn
pub static STEPS_ADD_L_DN_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_l_dn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_L_AN_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_l_an_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_L_AI_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_L_PI_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_pi_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_L_PD_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_L_D16_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_L_IDX_DN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_L_ABSW_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_L_ABSL_DN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_L_PCD16_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_L_PCIDX_DN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_ADD_L_IMM_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_add_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// ============================================================================
// Static Micro-Step Slices: ADD Dn, <ea> (Class 0 Read-Modify-Write)
// ============================================================================

// Byte: Dn, <ea>
pub static STEPS_ADD_B_DN_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_ADD_B_DN_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_pi_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_ADD_B_DN_PD: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_ADD_B_DN_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_ADD_B_DN_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_ADD_B_DN_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_ADD_B_DN_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];

// Word: Dn, <ea>
pub static STEPS_ADD_W_DN_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ADD_W_DN_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_pi_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ADD_W_DN_PD: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_pd_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ADD_W_DN_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ADD_W_DN_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ADD_W_DN_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_ADD_W_DN_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];

// Long: Dn, <ea>
pub static STEPS_ADD_L_DN_AI: [MicroStep; 5] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_ADD_L_DN_PI: [MicroStep; 5] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_dst_pi_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_ADD_L_DN_PD: [MicroStep; 5] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_dst_pd_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_ADD_L_DN_D16: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_ADD_L_DN_IDX: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_ADD_L_DN_ABSW: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_ADD_L_DN_ABSL: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_add_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
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

// ============================================================================
// Legacy Forwarders (Bypassed at runtime by OPCODE_DESCRIPTOR_TABLE)
// ============================================================================

use crate::core::{Cpu, StepResult};
use memory_bus::MemoryBus;

pub fn op_add_b_absl_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_absw_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_ai_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_disp_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_dn_absl(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_dn_absw(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_dn_ai(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_dn_disp(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_dn_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_dn_idx(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_dn_pd(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_dn_pi(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_idx_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_imm_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_pcdisp_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_pcidx_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_pd_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_b_pi_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_absl_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_absw_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_ai_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_an_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_disp_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_dn_absl(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_dn_absw(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_dn_ai(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_dn_disp(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_dn_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_dn_idx(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_dn_pd(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_dn_pi(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_idx_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_imm_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_pcdisp_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_pcidx_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_pd_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_l_pi_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_absl_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_absw_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_ai_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_an_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_disp_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_dn_absl(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_dn_absw(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_dn_ai(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_dn_disp(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_dn_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_dn_idx(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_dn_pd(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_dn_pi(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_idx_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_imm_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_pcdisp_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_pcidx_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_pd_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_add_w_pi_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
