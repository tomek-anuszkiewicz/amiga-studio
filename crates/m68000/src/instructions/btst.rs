//! BTST (Bit Test) instruction handlers and CCR updates
//!
//! Tests a single bit in a Data Register (modulo 32) or memory operand (modulo 8).
//! Updates Z flag (set if bit is 0, cleared if bit is 1). Other condition codes unaffected.

use crate::core::{Cpu, StepResult};
use crate::micro::ea;
use crate::micro::types::{flags, MicroAction, MicroStep};
use crate::state::CpuState;
use memory_bus::MemoryBus;

// ============================================================================
// Leaf ALU BTST Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn btst_l(state: &mut CpuState, bit_num: u32, val: u32) {
    let bit_idx = bit_num & 31;
    let bit_val = (val & (1 << bit_idx)) != 0;
    state.set_ccr_z_only(!bit_val);
}

#[inline(always)]
pub fn btst_b(state: &mut CpuState, bit_num: u32, val: u8) {
    let bit_idx = bit_num & 7;
    let bit_val = (val & (1 << bit_idx)) != 0;
    state.set_ccr_z_only(!bit_val);
}

#[inline]
pub fn execute_btst(state: &mut CpuState, bit_num: u32, val: u32, is_register: bool) {
    if is_register {
        btst_l(state, bit_num, val);
    } else {
        btst_b(state, bit_num, (val & 0xFF) as u8);
    }
}

// ============================================================================
// Immediate Bit Latching Callbacks
// ============================================================================

#[inline(always)]
pub fn latch_bit_imm(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.scratch[3] = (state.prefetch[0] & 0xFF) as u32;
}

#[inline(always)]
pub fn latch_bit_imm_calc_ai(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[3] = (state.prefetch[0] & 0xFF) as u32;
    ea::ea_calc_dst_ai(state, 0, reg_dst);
}

#[inline(always)]
pub fn latch_bit_imm_calc_pi(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    state.micro.scratch[3] = (state.prefetch[0] & 0xFF) as u32;
    ea::ea_calc_dst_pi_b(state, 0, reg_dst);
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
    let val = (state.micro.last_read & 0xFF) as u8;
    btst_b(state, bit_num, val);
}

pub fn alu_btst_b_dyn_imm(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let bit_num = state.d_long(reg_src as usize);
    let val = (state.prefetch[0] & 0xFF) as u8;
    btst_b(state, bit_num, val);
}

pub fn alu_btst_l_imm_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let bit_num = state.micro.scratch[3];
    let val = state.d_long(reg_dst as usize);
    btst_l(state, bit_num, val);
}

pub fn alu_btst_b_imm_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let bit_num = state.micro.scratch[3];
    let val = (state.micro.last_read & 0xFF) as u8;
    btst_b(state, bit_num, val);
}

// ============================================================================
// Static Micro-Step Slices: Dynamic BTST Dn, <ea>
// ============================================================================

pub static STEPS_BTST_DYN_DN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_l_dyn_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_DYN_AI: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_dyn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_DYN_PI: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_dst_pi_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_dyn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_DYN_PD: [MicroStep; 3] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_dyn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_DYN_D16: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_dyn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_DYN_IDX: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_dyn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_DYN_ABSW: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_dyn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_DYN_ABSL: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_dyn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_DYN_PCD16: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_dyn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_DYN_PCIDX: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_dyn_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_DYN_IMM: [MicroStep; 3] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_btst_b_dyn_imm), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// ============================================================================
// Static Micro-Step Slices: Static BTST #imm, <ea>
// ============================================================================

pub static STEPS_BTST_STAT_DN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_bit_imm), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_l_imm_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_STAT_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_bit_imm_calc_ai), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_STAT_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_bit_imm_calc_pi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_STAT_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_bit_imm), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_pd_b), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_STAT_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_bit_imm), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_dst_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_STAT_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_bit_imm), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_dst_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_STAT_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_bit_imm), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_STAT_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_bit_imm), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_STAT_PCD16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_bit_imm), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_BTST_STAT_PCIDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(latch_bit_imm), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_btst_b_imm_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// ============================================================================
// Static Decoder Functions
// ============================================================================

/// Decodes the micro-step sequence for dynamic BTST Dn, <ea>
pub const fn decode_btst_dyn_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(&STEPS_BTST_DYN_DN),
        2 => Some(&STEPS_BTST_DYN_AI),
        3 => Some(&STEPS_BTST_DYN_PI),
        4 => Some(&STEPS_BTST_DYN_PD),
        5 => Some(&STEPS_BTST_DYN_D16),
        6 => Some(&STEPS_BTST_DYN_IDX),
        7 => match reg {
            0 => Some(&STEPS_BTST_DYN_ABSW),
            1 => Some(&STEPS_BTST_DYN_ABSL),
            2 => Some(&STEPS_BTST_DYN_PCD16),
            3 => Some(&STEPS_BTST_DYN_PCIDX),
            4 => Some(&STEPS_BTST_DYN_IMM),
            _ => None,
        },
        _ => None,
    }
}

/// Decodes the micro-step sequence for static BTST #imm, <ea>
pub const fn decode_btst_stat_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(&STEPS_BTST_STAT_DN),
        2 => Some(&STEPS_BTST_STAT_AI),
        3 => Some(&STEPS_BTST_STAT_PI),
        4 => Some(&STEPS_BTST_STAT_PD),
        5 => Some(&STEPS_BTST_STAT_D16),
        6 => Some(&STEPS_BTST_STAT_IDX),
        7 => match reg {
            0 => Some(&STEPS_BTST_STAT_ABSW),
            1 => Some(&STEPS_BTST_STAT_ABSL),
            2 => Some(&STEPS_BTST_STAT_PCD16),
            3 => Some(&STEPS_BTST_STAT_PCIDX),
            _ => None,
        },
        _ => None,
    }
}

// ============================================================================
// Legacy Stubs (to be removed in Phase 7)
// ============================================================================

pub fn op_btst_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_btst_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_btst_b_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_dn(cpu, bus) }
pub fn op_btst_b_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_dn(cpu, bus) }
pub fn op_btst_b_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_dn(cpu, bus) }
pub fn op_btst_b_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_dn(cpu, bus) }
pub fn op_btst_b_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_dn(cpu, bus) }
pub fn op_btst_b_dn_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_dn(cpu, bus) }
pub fn op_btst_b_dn_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_dn(cpu, bus) }
pub fn op_btst_b_dn_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_dn(cpu, bus) }
pub fn op_btst_b_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_dn(cpu, bus) }
pub fn op_btst_b_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_dn(cpu, bus) }

pub fn op_btst_b_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_imm(cpu, bus) }
pub fn op_btst_b_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_imm(cpu, bus) }
pub fn op_btst_b_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_imm(cpu, bus) }
pub fn op_btst_b_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_imm(cpu, bus) }
pub fn op_btst_b_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_imm(cpu, bus) }
pub fn op_btst_b_imm_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_imm(cpu, bus) }
pub fn op_btst_b_imm_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_imm(cpu, bus) }
pub fn op_btst_b_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_imm(cpu, bus) }
pub fn op_btst_b_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_imm(cpu, bus) }

pub fn op_btst_l_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_dn(cpu, bus) }
pub fn op_btst_l_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_btst_imm(cpu, bus) }
