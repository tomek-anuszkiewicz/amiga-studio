//! M68000 SUBA Instruction (`SUBA <ea>, An`)
//!
//! Subtracts an effective address operand from an address register without modifying CCR flags.
//! Sign-extends word operands to 32 bits prior to subtraction.

use crate::core::{Cpu, StepResult};
use crate::micro::ea;
use crate::micro::types::{flags, MicroAction, MicroStep};
use crate::state::CpuState;
use memory_bus::MemoryBus;

// ============================================================================
// Micro-Step ALU Callbacks (AluFn)
// ============================================================================

pub fn alu_suba_w_dn_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.d_long(reg_src as usize) as i16 as i32) as u32;
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_sub(s));
}

pub fn alu_suba_w_an_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = (state.read_a(reg_src as usize) as i16 as i32) as u32;
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_sub(s));
}

pub fn alu_suba_l_dn_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.d_long(reg_src as usize);
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_sub(s));
}

pub fn alu_suba_l_an_an(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let s = state.read_a(reg_src as usize);
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_sub(s));
}

pub fn alu_suba_w_mem_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.micro.last_read as i16 as i32) as u32;
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_sub(s));
}

pub fn alu_suba_l_mem_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = state.micro.scratch[1];
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_sub(s));
}

pub fn alu_suba_w_imm_an(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let s = (state.prefetch[0] as i16 as i32) as u32;
    let d = state.read_a(reg_dst as usize);
    state.write_a(reg_dst as usize, d.wrapping_sub(s));
}

// ============================================================================
// Static Micro-Step Slices: SUBA <ea>, An
// ============================================================================

// Word: <ea>, An
pub static STEPS_SUBA_W_DN_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_w_dn_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_W_AN_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_w_an_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_W_AI_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_W_PI_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_pi_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_W_PD_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::BusReadWord, alu_fn: Some(ea::ea_calc_src_pd_w), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_W_D16_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_W_IDX_AN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_W_ABSW_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_W_ABSL_AN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_W_D16PC_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_W_IDXPC_AN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadWord, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_w_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_W_IMM_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(alu_suba_w_imm_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// Long: <ea>, An
pub static STEPS_SUBA_L_DN_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_l_dn_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_L_AN_AN: [MicroStep; 2] = [
    MicroStep { action: MicroAction::Alu, alu_fn: None, base_clocks: 4, flags: 0 },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_l_an_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_L_AI_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_ai), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_L_PI_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_pi_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_L_PD_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: Some(ea::ea_calc_src_pd_l), base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_L_D16_AN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_src_d16_an), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_L_IDX_AN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_src_idx_an), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_L_ABSW_AN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absw), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_L_ABSL_AN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_absl_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_L_D16PC_AN: [MicroStep; 4] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_d16_pc), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_L_IDXPC_AN: [MicroStep; 5] = [
    MicroStep { action: MicroAction::Alu, alu_fn: Some(ea::ea_calc_idx_pc), base_clocks: 2, flags: 0 },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::BusReadLongHigh, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::BusReadLongLow, alu_fn: None, base_clocks: 4, flags: flags::READ | flags::DATA_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];
pub static STEPS_SUBA_L_IMM_AN: [MicroStep; 3] = [
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_hi), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::FetchExtension, alu_fn: Some(ea::ea_calc_imm_l_lo), base_clocks: 4, flags: flags::READ | flags::PROGRAM_SPACE },
    MicroStep { action: MicroAction::PrefetchNextOpcodeAndRetire, alu_fn: Some(alu_suba_l_mem_an), base_clocks: 4, flags: flags::READ | flags::PREFETCH | flags::PROGRAM_SPACE },
];

// ============================================================================
// Opcode Descriptor Decoder Helper for SUBA
// ============================================================================

/// Maps a SUBA opcode's bit fields to its static micro-step sequence
pub const fn decode_suba_steps(is_long: bool, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    if !is_long {
        // Word: SUBA.W <ea>, An
        match mode {
            0 => Some(&STEPS_SUBA_W_DN_AN),
            1 => Some(&STEPS_SUBA_W_AN_AN),
            2 => Some(&STEPS_SUBA_W_AI_AN),
            3 => Some(&STEPS_SUBA_W_PI_AN),
            4 => Some(&STEPS_SUBA_W_PD_AN),
            5 => Some(&STEPS_SUBA_W_D16_AN),
            6 => Some(&STEPS_SUBA_W_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_SUBA_W_ABSW_AN),
                1 => Some(&STEPS_SUBA_W_ABSL_AN),
                2 => Some(&STEPS_SUBA_W_D16PC_AN),
                3 => Some(&STEPS_SUBA_W_IDXPC_AN),
                4 => Some(&STEPS_SUBA_W_IMM_AN),
                _ => None,
            },
            _ => None,
        }
    } else {
        // Long: SUBA.L <ea>, An
        match mode {
            0 => Some(&STEPS_SUBA_L_DN_AN),
            1 => Some(&STEPS_SUBA_L_AN_AN),
            2 => Some(&STEPS_SUBA_L_AI_AN),
            3 => Some(&STEPS_SUBA_L_PI_AN),
            4 => Some(&STEPS_SUBA_L_PD_AN),
            5 => Some(&STEPS_SUBA_L_D16_AN),
            6 => Some(&STEPS_SUBA_L_IDX_AN),
            7 => match reg {
                0 => Some(&STEPS_SUBA_L_ABSW_AN),
                1 => Some(&STEPS_SUBA_L_ABSL_AN),
                2 => Some(&STEPS_SUBA_L_D16PC_AN),
                3 => Some(&STEPS_SUBA_L_IDXPC_AN),
                4 => Some(&STEPS_SUBA_L_IMM_AN),
                _ => None,
            },
            _ => None,
        }
    }
}

// ============================================================================
// Legacy compatibility forwarders
// ============================================================================
pub fn op_suba(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_l_absl_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_l_absw_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_l_ai_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_l_an_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_l_disp_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_l_dn_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_l_idx_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_l_imm_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_l_pcdisp_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_l_pcidx_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_l_pd_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_l_pi_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_w_absl_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_w_absw_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_w_ai_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_w_an_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_w_disp_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_w_dn_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_w_idx_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_w_imm_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_w_pcdisp_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_w_pcidx_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_w_pd_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
pub fn op_suba_w_pi_an(_: &mut Cpu, _: &mut MemoryBus) -> StepResult { StepResult::InstructionCompleted }
