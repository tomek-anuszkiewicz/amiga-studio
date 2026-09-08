//! M68000 TST Instruction (`TST <ea>`)
//!
//! Evaluates an effective address operand against zero.
//! Updates N and Z flags according to the operand value; clears V and C flags.
//! Extend (X) flag is unaffected. Operand is not modified.

use crate::core::{Cpu, StepResult};
use crate::micro::ea;
use crate::micro::types::{flags, MicroAction, MicroStep, Size};
use crate::state::CpuState;
use memory_bus::MemoryBus;

// ============================================================================
// Leaf ALU TST Functions (Branchless & Endian-Neutral)
// ============================================================================

#[inline(always)]
pub fn tst_b(state: &mut CpuState, d: u8) {
    state.set_ccr_nz_clear_vc((d & 0x80) != 0, d == 0);
}

#[inline(always)]
pub fn tst_w(state: &mut CpuState, d: u16) {
    state.set_ccr_nz_clear_vc((d & 0x8000) != 0, d == 0);
}

#[inline(always)]
pub fn tst_l(state: &mut CpuState, d: u32) {
    state.set_ccr_nz_clear_vc((d & 0x8000_0000) != 0, d == 0);
}

pub fn execute_tst(state: &mut CpuState, val: u32, size: Size) {
    match size {
        Size::Byte => tst_b(state, (val & 0xFF) as u8),
        Size::Word => tst_w(state, (val & 0xFFFF) as u16),
        Size::Long => tst_l(state, val),
    }
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_tst_b_dn(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let d = (state.d_long(reg_src as usize) & 0xFF) as u8;
    tst_b(state, d);
}

pub fn alu_tst_w_dn(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let d = (state.d_long(reg_src as usize) & 0xFFFF) as u16;
    tst_w(state, d);
}

pub fn alu_tst_l_dn(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let d = state.d_long(reg_src as usize);
    tst_l(state, d);
}

pub fn alu_tst_b_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = (state.micro.last_read & 0xFF) as u8;
    tst_b(state, d);
}

pub fn alu_tst_w_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = state.micro.last_read;
    tst_w(state, d);
}

pub fn alu_tst_l_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d = state.micro.scratch[1];
    tst_l(state, d);
}

// ============================================================================
// Static Micro-Step Slices: TST <ea>
// ============================================================================

// Byte
pub static STEPS_TST_B_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_b_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_B_AI: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_B_PI: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_src_pi_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_B_PD: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadByte, alu_fn: Some(ea::ea_calc_src_pd_b), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_B_D16: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_B_IDX: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_B_ABSW: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_B_ABSL: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_B_PCD16: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_B_PCIDX: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadByte, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_b_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// Word
pub static STEPS_TST_W_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_w_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_W_AI: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_W_PI: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_pi_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_W_PD: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_pd_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_W_D16: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_W_IDX: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_W_ABSW: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_W_ABSL: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_W_PCD16: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_W_PCIDX: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_w_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// Long
pub static STEPS_TST_L_DN: [MicroStep; 1] = [
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_l_dn), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_L_AI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_L_PI: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_pi_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_L_PD: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_L_D16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_L_IDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_L_ABSW: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_L_ABSL: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_L_PCD16: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_TST_L_PCIDX: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_tst_l_mem), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// ============================================================================
// Static Decoder Function
// ============================================================================

/// Decodes the micro-step sequence for TST based on size, mode, and reg
pub const fn decode_tst_steps(size: u8, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => match mode {
            0 => Some(&STEPS_TST_B_DN),
            2 => Some(&STEPS_TST_B_AI),
            3 => Some(&STEPS_TST_B_PI),
            4 => Some(&STEPS_TST_B_PD),
            5 => Some(&STEPS_TST_B_D16),
            6 => Some(&STEPS_TST_B_IDX),
            7 => match reg {
                0 => Some(&STEPS_TST_B_ABSW),
                1 => Some(&STEPS_TST_B_ABSL),
                2 => Some(&STEPS_TST_B_PCD16),
                3 => Some(&STEPS_TST_B_PCIDX),
                _ => None,
            },
            _ => None,
        },
        1 => match mode {
            0 => Some(&STEPS_TST_W_DN),
            2 => Some(&STEPS_TST_W_AI),
            3 => Some(&STEPS_TST_W_PI),
            4 => Some(&STEPS_TST_W_PD),
            5 => Some(&STEPS_TST_W_D16),
            6 => Some(&STEPS_TST_W_IDX),
            7 => match reg {
                0 => Some(&STEPS_TST_W_ABSW),
                1 => Some(&STEPS_TST_W_ABSL),
                2 => Some(&STEPS_TST_W_PCD16),
                3 => Some(&STEPS_TST_W_PCIDX),
                _ => None,
            },
            _ => None,
        },
        2 => match mode {
            0 => Some(&STEPS_TST_L_DN),
            2 => Some(&STEPS_TST_L_AI),
            3 => Some(&STEPS_TST_L_PI),
            4 => Some(&STEPS_TST_L_PD),
            5 => Some(&STEPS_TST_L_D16),
            6 => Some(&STEPS_TST_L_IDX),
            7 => match reg {
                0 => Some(&STEPS_TST_L_ABSW),
                1 => Some(&STEPS_TST_L_ABSL),
                2 => Some(&STEPS_TST_L_PCD16),
                3 => Some(&STEPS_TST_L_PCIDX),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

// ============================================================================
// Legacy Stubs (to be removed in Phase 7)
// ============================================================================

pub fn op_tst(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    crate::micro::engine::execute_micro_step(cpu, bus)
}

pub fn op_tst_b_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_b_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_b_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_b_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_b_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_b_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_b_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_b_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_b_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_b_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }

pub fn op_tst_l_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_l_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_l_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_l_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_l_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_l_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_l_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_l_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_l_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_l_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }

pub fn op_tst_w_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_w_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_w_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_w_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_w_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_w_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_w_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_w_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_w_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
pub fn op_tst_w_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult { op_tst(cpu, bus) }
