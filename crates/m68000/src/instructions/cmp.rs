//! M68000 CMP Instruction (`CMP <ea>, Dn`)
//!
//! Evaluates (dst - src) and updates N, Z, V, and C condition code flags.
//! Extend (X) flag is unaffected. Destination register Dn is not modified.

use crate::core::{Cpu, StepResult};
use crate::micro::ea;
use crate::micro::types::{flags, MicroAction, MicroStep, Size};
use crate::state::CpuState;
use memory_bus::MemoryBus;

// ============================================================================
// Core Leaf ALU Compare Functions (Branchless & Cycle-Exact CCR)
// ============================================================================

#[inline(always)]
pub fn cmp_b(state: &mut CpuState, s: u8, d: u8) {
    let (res, c) = d.overflowing_sub(s);
    let v = (((s ^ d) & (d ^ res)) & 0x80) != 0;
    let n = (res & 0x80) != 0;
    let z = res == 0;
    state.set_ccr_nzvc(n, z, v, c);
}

#[inline(always)]
pub fn cmp_w(state: &mut CpuState, s: u16, d: u16) {
    let (res, c) = d.overflowing_sub(s);
    let v = (((s ^ d) & (d ^ res)) & 0x8000) != 0;
    let n = (res & 0x8000) != 0;
    let z = res == 0;
    state.set_ccr_nzvc(n, z, v, c);
}

#[inline(always)]
pub fn cmp_l(state: &mut CpuState, s: u32, d: u32) {
    let (res, c) = d.overflowing_sub(s);
    let v = (((s ^ d) & (d ^ res)) & 0x8000_0000) != 0;
    let n = (res & 0x8000_0000) != 0;
    let z = res == 0;
    state.set_ccr_nzvc(n, z, v, c);
}

/// Unified core compare function
pub fn execute_cmp(state: &mut CpuState, src: u32, dst: u32, size: Size) {
    match size {
        Size::Byte => cmp_b(state, (src & 0xFF) as u8, (dst & 0xFF) as u8),
        Size::Word => cmp_w(state, (src & 0xFFFF) as u16, (dst & 0xFFFF) as u16),
        Size::Long => cmp_l(state, src, dst),
    }
}

// ============================================================================
// Micro-Step ALU Callbacks (AluFn)
// ============================================================================

pub fn alu_cmp_b_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    cmp_b(state, s, d);
}

pub fn alu_cmp_w_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    cmp_w(state, s, d);
}

pub fn alu_cmp_w_an_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.read_a(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    cmp_w(state, s, d);
}

pub fn alu_cmp_l_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmp_l_an_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.read_a(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmp_b_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.last_read & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    cmp_b(state, s, d);
}

pub fn alu_cmp_w_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.last_read;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    cmp_w(state, s, d);
}

pub fn alu_cmp_l_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.scratch[1];
    let d = state.d_long(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmp_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    cmp_b(state, s, d);
}

pub fn alu_cmp_w_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.prefetch[0];
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    cmp_w(state, s, d);
}

// ============================================================================
// Static Micro-Step Slices: CMP <ea>, Dn
// ============================================================================

// Byte: <ea>, Dn
pub static STEPS_CMP_B_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_b_dn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_B_AI_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_B_PI_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_B_PD_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_B_D16_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_B_IDX_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_B_ABSW_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_B_ABSL_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_B_D16PC_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_B_IDXPC_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_B_IMM_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_cmp_b_imm_dn), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// Word: <ea>, Dn
pub static STEPS_CMP_W_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_w_dn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_W_AN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_w_an_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_W_AI_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_W_PI_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_pi_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_W_PD_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_pd_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_W_D16_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_W_IDX_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_W_ABSW_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_W_ABSL_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_W_D16PC_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_W_IDXPC_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_W_IMM_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_cmp_w_imm_dn), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// Long: <ea>, Dn
pub static STEPS_CMP_L_DN_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_l_dn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_L_AN_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_l_an_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_L_AI_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_L_PI_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_pi_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_L_PD_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_L_D16_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_L_IDX_DN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_L_ABSW_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_L_ABSL_DN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_L_D16PC_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_L_IDXPC_DN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMP_L_IMM_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmp_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// ============================================================================
// Opcode Descriptor Decoder Helper for CMP
// ============================================================================

/// Maps a CMP opcode's bit fields to its static micro-step sequence
pub const fn decode_cmp_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_CMP_B_DN_DN),
            2 => Some(&STEPS_CMP_B_AI_DN),
            3 => Some(&STEPS_CMP_B_PI_DN),
            4 => Some(&STEPS_CMP_B_PD_DN),
            5 => Some(&STEPS_CMP_B_D16_DN),
            6 => Some(&STEPS_CMP_B_IDX_DN),
            7 => match reg {
                0 => Some(&STEPS_CMP_B_ABSW_DN),
                1 => Some(&STEPS_CMP_B_ABSL_DN),
                2 => Some(&STEPS_CMP_B_D16PC_DN),
                3 => Some(&STEPS_CMP_B_IDXPC_DN),
                4 => Some(&STEPS_CMP_B_IMM_DN),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_CMP_W_DN_DN),
            1 => Some(&STEPS_CMP_W_AN_DN),
            2 => Some(&STEPS_CMP_W_AI_DN),
            3 => Some(&STEPS_CMP_W_PI_DN),
            4 => Some(&STEPS_CMP_W_PD_DN),
            5 => Some(&STEPS_CMP_W_D16_DN),
            6 => Some(&STEPS_CMP_W_IDX_DN),
            7 => match reg {
                0 => Some(&STEPS_CMP_W_ABSW_DN),
                1 => Some(&STEPS_CMP_W_ABSL_DN),
                2 => Some(&STEPS_CMP_W_D16PC_DN),
                3 => Some(&STEPS_CMP_W_IDXPC_DN),
                4 => Some(&STEPS_CMP_W_IMM_DN),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_CMP_L_DN_DN),
            1 => Some(&STEPS_CMP_L_AN_DN),
            2 => Some(&STEPS_CMP_L_AI_DN),
            3 => Some(&STEPS_CMP_L_PI_DN),
            4 => Some(&STEPS_CMP_L_PD_DN),
            5 => Some(&STEPS_CMP_L_D16_DN),
            6 => Some(&STEPS_CMP_L_IDX_DN),
            7 => match reg {
                0 => Some(&STEPS_CMP_L_ABSW_DN),
                1 => Some(&STEPS_CMP_L_ABSL_DN),
                2 => Some(&STEPS_CMP_L_D16PC_DN),
                3 => Some(&STEPS_CMP_L_IDXPC_DN),
                4 => Some(&STEPS_CMP_L_IMM_DN),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

// ============================================================================
// Legacy compatibility forwarders
// ============================================================================
pub fn op_cmp_b_absl_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_b_absw_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_b_ai_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_b_disp_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_b_dn_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_b_idx_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_b_imm_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_b_pcdisp_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_b_pcidx_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_b_pd_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_b_pi_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_l_absl_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_l_absw_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_l_ai_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_l_an_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_l_disp_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_l_dn_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_l_idx_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_l_imm_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_l_pcdisp_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_l_pcidx_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_l_pd_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_l_pi_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_w_absl_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_w_absw_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_w_ai_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_w_an_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_w_disp_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_w_dn_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_w_idx_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_w_imm_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_w_pcdisp_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_w_pcidx_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_w_pd_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmp_w_pi_dn(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
