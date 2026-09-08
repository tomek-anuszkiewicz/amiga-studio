//! MOVEM (Move Multiple Registers) Micro-Step State Machine Implementation
//!
//! Cycle-exact micro-step slices and decode functions.

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::{flags, MicroAction, MicroStep};
use crate::state::CpuState;

// ============================================================================
// Pure ALU Callbacks: MOVEM
// ============================================================================

/// Latches the 16-bit register mask extension word into scratch[2] and resets transfer state
#[inline(always)]
pub fn alu_movem_fetch_mask(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.scratch[2] = state.prefetch[0] as u32;
    state.micro.scratch[3] = 0;
}

// ============================================================================
// Atomic Micro-Step Constants
// ============================================================================

const FETCH_MASK: MicroStep = MicroStep {
    action: MicroAction::FetchExtension,
    alu_fn: Some(alu_movem_fetch_mask),
    base_clocks: 4,
    flags: flags::READ | flags::PROGRAM_SPACE,
};
const FETCH_EXT: MicroStep = MicroStep {
    action: MicroAction::FetchExtension,
    alu_fn: None,
    base_clocks: 4,
    flags: flags::READ | flags::PROGRAM_SPACE,
};
const MOVEM_TRANSFER: MicroStep = MicroStep {
    action: MicroAction::MovemTransfer,
    alu_fn: None,
    base_clocks: 0,
    flags: flags::NONE,
};
const PREFETCH_RETIRE: MicroStep = common::RETIRE_STANDARD;

// ============================================================================
// Static Step Slices: MOVEM
// ============================================================================

pub static STEPS_MOVEM_AI: [MicroStep; 4] = [
    FETCH_MASK,
    MicroStep::alu(ea::ea_calc_src_ai),
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_PI: [MicroStep; 4] = [
    FETCH_MASK,
    MicroStep::alu(ea::ea_calc_src_ai),
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_PD: [MicroStep; 4] = [
    FETCH_MASK,
    MicroStep::alu(ea::ea_calc_src_ai),
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_D16: [MicroStep; 4] = [
    FETCH_MASK,
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_IDX: [MicroStep; 5] = [
    FETCH_MASK,
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
        flags: flags::NONE,
    },
    FETCH_EXT,
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_ABSW: [MicroStep; 4] = [
    FETCH_MASK,
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_ABSL: [MicroStep; 5] = [
    FETCH_MASK,
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_PCD16: [MicroStep; 4] = [
    FETCH_MASK,
    MicroStep {
        action: MicroAction::FetchExtension,
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 4,
        flags: flags::READ | flags::PROGRAM_SPACE,
    },
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

pub static STEPS_MOVEM_PCIDX: [MicroStep; 5] = [
    FETCH_MASK,
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
        flags: flags::NONE,
    },
    FETCH_EXT,
    MOVEM_TRANSFER,
    PREFETCH_RETIRE,
];

// ============================================================================
// Opcode Decoder: MOVEM
// ============================================================================

/// Compile-time opcode decoder for MOVEM
pub const fn decode_movem_steps(is_reg_to_mem: bool, mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    if is_reg_to_mem {
        match mode {
            2 => Some(&STEPS_MOVEM_AI),
            4 => Some(&STEPS_MOVEM_PD),
            5 => Some(&STEPS_MOVEM_D16),
            6 => Some(&STEPS_MOVEM_IDX),
            7 => match reg {
                0 => Some(&STEPS_MOVEM_ABSW),
                1 => Some(&STEPS_MOVEM_ABSL),
                _ => None,
            },
            _ => None,
        }
    } else {
        match mode {
            2 => Some(&STEPS_MOVEM_AI),
            3 => Some(&STEPS_MOVEM_PI),
            5 => Some(&STEPS_MOVEM_D16),
            6 => Some(&STEPS_MOVEM_IDX),
            7 => match reg {
                0 => Some(&STEPS_MOVEM_ABSW),
                1 => Some(&STEPS_MOVEM_ABSL),
                2 => Some(&STEPS_MOVEM_PCD16),
                3 => Some(&STEPS_MOVEM_PCIDX),
                _ => None,
            },
            _ => None,
        }
    }
}
