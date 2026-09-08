//! M68000 AND Instruction (`AND <ea>, Dn` and `AND Dn, <ea>`)
//!
//! Evaluates bitwise AND between source and destination, updating CCR flags
//! (N, Z set according to result, V and C cleared, X unaffected).

use crate::core::{Cpu, StepResult};
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::{flags, MicroAction, MicroStep, Size};
use crate::state::CpuState;
use memory_bus::MemoryBus;

// ============================================================================
// Leaf ALU AND Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn and_b(state: &mut CpuState, s: u8, d: u8) -> u8 {
    let res = d & s;
    state.set_ccr_nz_clear_vc((res & 0x80) != 0, res == 0);
    res
}

#[inline(always)]
pub fn and_w(state: &mut CpuState, s: u16, d: u16) -> u16 {
    let res = d & s;
    state.set_ccr_nz_clear_vc((res & 0x8000) != 0, res == 0);
    res
}

#[inline(always)]
pub fn and_l(state: &mut CpuState, s: u32, d: u32) -> u32 {
    let res = d & s;
    state.set_ccr_nz_clear_vc((res & 0x8000_0000) != 0, res == 0);
    res
}

pub fn execute_and(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    match size {
        Size::Byte => {
            let res = and_b(state, (src & 0xFF) as u8, (dst & 0xFF) as u8);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let res = and_w(state, (src & 0xFFFF) as u16, (dst & 0xFFFF) as u16);
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => and_l(state, src, dst),
    }
}

// ============================================================================
// Micro-Step ALU Callbacks: <ea>, Dn
// ============================================================================

pub fn alu_and_b_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = and_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_and_w_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = and_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_and_l_dn_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = and_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_and_w_an_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.read_a(reg_src as usize) & 0xFFFF) as u16;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = and_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_and_l_an_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.read_a(reg_src as usize);
    let d = state.d_long(reg_dst as usize);
    let res = and_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_and_b_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.last_read & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = and_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_and_w_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.last_read;
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = and_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_and_l_mem_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.scratch[1];
    let d = state.d_long(reg_dst as usize);
    let res = and_l(state, s, d);
    state.set_d_long(reg_dst as usize, res);
}

pub fn alu_and_b_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] & 0xFF) as u8;
    let d = (state.d_long(reg_dst as usize) & 0xFF) as u8;
    let res = and_b(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFF) | (res as u32));
}

pub fn alu_and_w_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.prefetch[0];
    let d = (state.d_long(reg_dst as usize) & 0xFFFF) as u16;
    let res = and_w(state, s, d);
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (res as u32));
}

pub fn alu_and_b_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFF) as u8;
    let d = (state.micro.last_read & 0xFF) as u8;
    let res = and_b(state, s, d);
    state.micro.write_buffer = res as u32;
}

pub fn alu_and_w_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    let d = state.micro.last_read;
    let res = and_w(state, s, d);
    state.micro.write_buffer = res as u32;
}

pub fn alu_and_l_dn_mem(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.micro.scratch[1];
    let res = and_l(state, s, d);
    state.micro.write_buffer = res;
}

// ============================================================================
// Static Micro-Step Slices: AND <ea>, Dn
// ============================================================================

// Byte: <ea>, Dn
pub static STEPS_AND_B_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_b_dn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_B_AI_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_B_PI_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_B_PD_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_B_D16_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_B_IDX_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_B_ABSW_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_B_ABSL_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_B_PCD16_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_B_PCIDX_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_b_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_B_IMM_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_and_b_imm_dn), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// Word: <ea>, Dn
pub static STEPS_AND_W_DN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_w_dn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_W_AN_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_w_an_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_W_AI_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_W_PI_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_pi_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_W_PD_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_w), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_W_D16_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_W_IDX_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_W_ABSW_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_W_ABSL_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_W_PCD16_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_W_PCIDX_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_w_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_W_IMM_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_and_w_imm_dn), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// Long: <ea>, Dn
pub static STEPS_AND_L_DN_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_l_dn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_L_AN_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_l_an_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_L_AI_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_L_PI_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_pi_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_L_PD_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_L_D16_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_L_IDX_DN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_L_ABSW_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_L_ABSL_DN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_L_PCD16_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_L_PCIDX_DN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_AND_L_IMM_DN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_and_l_mem_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// ============================================================================
// Static Micro-Step Slices: AND Dn, <ea> (RMW)
// ============================================================================

// Byte RMW
pub static STEPS_AND_B_DN_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_AND_B_DN_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_pi_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_AND_B_DN_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_AND_B_DN_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_AND_B_DN_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_AND_B_DN_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];
pub static STEPS_AND_B_DN_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_b_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_BYTE_RETIRE,
];

// Word RMW
pub static STEPS_AND_W_DN_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_AND_W_DN_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_dst_pi_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_AND_W_DN_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_w), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_AND_W_DN_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_AND_W_DN_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_AND_W_DN_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];
pub static STEPS_AND_W_DN_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_w_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_WORD_RETIRE,
];

// Long RMW
pub static STEPS_AND_L_DN_AI: [MicroStep; 5] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_AND_L_DN_PI: [MicroStep; 5] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_dst_pi_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_AND_L_DN_PD: [MicroStep; 6] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_l), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_AND_L_DN_D16: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_AND_L_DN_IDX: [MicroStep; 7] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_AND_L_DN_ABSW: [MicroStep; 6] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];
pub static STEPS_AND_L_DN_ABSL: [MicroStep; 7] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusPrefetchToScratch, alu_fn: Some(alu_and_l_dn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
    common::RMW_WRITE_LONG_HIGH,
    common::RMW_WRITE_LONG_LOW_RETIRE,
];

// ============================================================================
// Static Decoder Function
// ============================================================================

/// Decodes the micro-step sequence for AND based on direction, size, mode, and reg
pub const fn decode_and_steps(dir: u8, size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    if dir == 0 {
        // <ea>, Dn
        match size {
            0 => match mode {
                0 => Some(&STEPS_AND_B_DN_DN),
                2 => Some(&STEPS_AND_B_AI_DN),
                3 => Some(&STEPS_AND_B_PI_DN),
                4 => Some(&STEPS_AND_B_PD_DN),
                5 => Some(&STEPS_AND_B_D16_DN),
                6 => Some(&STEPS_AND_B_IDX_DN),
                7 => match reg {
                    0 => Some(&STEPS_AND_B_ABSW_DN),
                    1 => Some(&STEPS_AND_B_ABSL_DN),
                    2 => Some(&STEPS_AND_B_PCD16_DN),
                    3 => Some(&STEPS_AND_B_PCIDX_DN),
                    4 => Some(&STEPS_AND_B_IMM_DN),
                    _ => None,
                },
                _ => None,
            },
            1 => match mode {
                0 => Some(&STEPS_AND_W_DN_DN),
                1 => Some(&STEPS_AND_W_AN_DN),
                2 => Some(&STEPS_AND_W_AI_DN),
                3 => Some(&STEPS_AND_W_PI_DN),
                4 => Some(&STEPS_AND_W_PD_DN),
                5 => Some(&STEPS_AND_W_D16_DN),
                6 => Some(&STEPS_AND_W_IDX_DN),
                7 => match reg {
                    0 => Some(&STEPS_AND_W_ABSW_DN),
                    1 => Some(&STEPS_AND_W_ABSL_DN),
                    2 => Some(&STEPS_AND_W_PCD16_DN),
                    3 => Some(&STEPS_AND_W_PCIDX_DN),
                    4 => Some(&STEPS_AND_W_IMM_DN),
                    _ => None,
                },
                _ => None,
            },
            2 => match mode {
                0 => Some(&STEPS_AND_L_DN_DN),
                1 => Some(&STEPS_AND_L_AN_DN),
                2 => Some(&STEPS_AND_L_AI_DN),
                3 => Some(&STEPS_AND_L_PI_DN),
                4 => Some(&STEPS_AND_L_PD_DN),
                5 => Some(&STEPS_AND_L_D16_DN),
                6 => Some(&STEPS_AND_L_IDX_DN),
                7 => match reg {
                    0 => Some(&STEPS_AND_L_ABSW_DN),
                    1 => Some(&STEPS_AND_L_ABSL_DN),
                    2 => Some(&STEPS_AND_L_PCD16_DN),
                    3 => Some(&STEPS_AND_L_PCIDX_DN),
                    4 => Some(&STEPS_AND_L_IMM_DN),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    } else {
        // Dn, <ea> (RMW memory)
        match size {
            0 => match mode {
                2 => Some(&STEPS_AND_B_DN_AI),
                3 => Some(&STEPS_AND_B_DN_PI),
                4 => Some(&STEPS_AND_B_DN_PD),
                5 => Some(&STEPS_AND_B_DN_D16),
                6 => Some(&STEPS_AND_B_DN_IDX),
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
                5 => Some(&STEPS_AND_W_DN_D16),
                6 => Some(&STEPS_AND_W_DN_IDX),
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
                5 => Some(&STEPS_AND_L_DN_D16),
                6 => Some(&STEPS_AND_L_DN_IDX),
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
}

// ============================================================================
// Legacy Stubs (to be removed in Phase 7)
// ============================================================================

pub fn op_and_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_and_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_and_b_absl_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_b_absw_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_b_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_b_disp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_b_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_b_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_b_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_b_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_b_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_b_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_b_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_b_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_b_idx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_b_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_b_pcdisp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_b_pcidx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_b_pd_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_b_pi_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }

pub fn op_and_l_absl_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_l_absw_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_l_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_l_an_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_l_disp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_l_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_l_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_l_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_l_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_l_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_l_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_l_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_l_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_l_idx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_l_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_l_pcdisp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_l_pcidx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_l_pd_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_l_pi_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }

pub fn op_and_w_absl_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_w_absw_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_w_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_w_an_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_w_disp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_w_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_w_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_w_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_w_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_w_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_w_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_w_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_w_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_dn_to_ea(cpu, bus) }
pub fn op_and_w_idx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_w_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_w_pcdisp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_w_pcidx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_w_pd_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
pub fn op_and_w_pi_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_and_ea_to_dn(cpu, bus) }
