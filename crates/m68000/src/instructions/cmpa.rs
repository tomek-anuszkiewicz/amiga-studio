//! M68000 CMPA Instruction (`CMPA <ea>, An`)
//!
//! Compares an effective address operand with an Address Register An.
//! Word operands are sign-extended to 32 bits and compared as 32-bit (Long).
//! Updates N, Z, V, C flags; Extend (X) flag is unaffected. Destination An is not modified.

use crate::core::{Cpu, StepResult};
use crate::instructions::cmp::cmp_l;
use crate::micro::ea;
use crate::micro::types::{flags, MicroAction, MicroStep};
use crate::state::CpuState;
use memory_bus::MemoryBus;

// ============================================================================
// Micro-Step ALU Callbacks (AluFn)
// ============================================================================

pub fn alu_cmpa_w_dn_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) as i16 as i32) as u32;
    let d = state.read_a(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmpa_w_an_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.read_a(reg_src as usize) as i16 as i32) as u32;
    let d = state.read_a(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmpa_l_dn_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.read_a(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmpa_l_an_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.read_a(reg_src as usize);
    let d = state.read_a(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmpa_w_mem_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.last_read as i16 as i32) as u32;
    let d = state.read_a(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmpa_l_mem_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.scratch[1];
    let d = state.read_a(reg_dst as usize);
    cmp_l(state, s, d);
}

pub fn alu_cmpa_w_imm_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] as i16 as i32) as u32;
    let d = state.read_a(reg_dst as usize);
    cmp_l(state, s, d);
}

// ============================================================================
// Static Micro-Step Slices: CMPA <ea>, An
// ============================================================================

// Word: <ea>, An (takes 6 clocks for register comparison)
pub static STEPS_CMPA_W_DN_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_w_dn_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_W_AN_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_w_an_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_W_AI_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_W_PI_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_pi_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_W_PD_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_pd_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_W_D16_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_W_IDX_AN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_W_ABSW_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_W_ABSL_AN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_W_D16PC_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_W_IDXPC_AN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_W_IMM_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_cmpa_w_imm_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// Long: <ea>, An
pub static STEPS_CMPA_L_DN_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_l_dn_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_L_AN_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_l_an_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_L_AI_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_L_PI_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_pi_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_L_PD_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_L_D16_AN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_L_IDX_AN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_L_ABSW_AN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_L_ABSL_AN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_L_D16PC_AN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_L_IDXPC_AN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_CMPA_L_IMM_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_cmpa_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// ============================================================================
// Opcode Descriptor Decoder Helper for CMPA
// ============================================================================

/// Maps a CMPA opcode's bit fields to its static micro-step sequence
pub const fn decode_cmpa_steps(is_long: bool, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    if !is_long {
        // Word: CMPA.W <ea>, An
        match mode {
            0 => Some(&STEPS_CMPA_W_DN_AN),
            1 => Some(&STEPS_CMPA_W_AN_AN),
            2 => Some(&STEPS_CMPA_W_AI_AN),
            3 => Some(&STEPS_CMPA_W_PI_AN),
            4 => Some(&STEPS_CMPA_W_PD_AN),
            5 => Some(&STEPS_CMPA_W_D16_AN),
            6 => Some(&STEPS_CMPA_W_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_CMPA_W_ABSW_AN),
                1 => Some(&STEPS_CMPA_W_ABSL_AN),
                2 => Some(&STEPS_CMPA_W_D16PC_AN),
                3 => Some(&STEPS_CMPA_W_IDXPC_AN),
                4 => Some(&STEPS_CMPA_W_IMM_AN),
                _ => None,
            },
            _ => None,
        }
    } else {
        // Long: CMPA.L <ea>, An
        match mode {
            0 => Some(&STEPS_CMPA_L_DN_AN),
            1 => Some(&STEPS_CMPA_L_AN_AN),
            2 => Some(&STEPS_CMPA_L_AI_AN),
            3 => Some(&STEPS_CMPA_L_PI_AN),
            4 => Some(&STEPS_CMPA_L_PD_AN),
            5 => Some(&STEPS_CMPA_L_D16_AN),
            6 => Some(&STEPS_CMPA_L_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_CMPA_L_ABSW_AN),
                1 => Some(&STEPS_CMPA_L_ABSL_AN),
                2 => Some(&STEPS_CMPA_L_D16PC_AN),
                3 => Some(&STEPS_CMPA_L_IDXPC_AN),
                4 => Some(&STEPS_CMPA_L_IMM_AN),
                _ => None,
            },
            _ => None,
        }
    }
}

// ============================================================================
// Legacy compatibility forwarders
// ============================================================================
pub fn op_cmpa(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_l_absl_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_l_absw_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_l_ai_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_l_an_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_l_disp_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_l_dn_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_l_idx_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_l_imm_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_l_pcdisp_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_l_pcidx_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_l_pd_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_l_pi_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_w_absl_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_w_absw_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_w_ai_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_w_an_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_w_disp_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_w_dn_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_w_idx_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_w_imm_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_w_pcdisp_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_w_pcidx_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_w_pd_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_cmpa_w_pi_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
