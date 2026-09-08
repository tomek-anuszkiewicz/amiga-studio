//! Bcc (Branch Conditionally) instruction handler
//!
//! Evaluates the 14 conditional branches using 8-bit short or 16-bit word displacements.
//! Execution time: 10 CPU clocks if taken; 8/12 CPU clocks if not taken.

use crate::micro::common;
use crate::micro::types::{MicroAction, MicroStep};
use crate::state::CpuState;

/// Taken branch execution steps (10 CPU clocks / 5 CCKs)
pub static STEPS_BRANCH_TAKEN: [MicroStep; 3] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: None,
        base_clocks: 2,
    },
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];

/// Untaken short branch execution steps (8 CPU clocks / 4 CCKs)
pub static STEPS_BRANCH_NOT_TAKEN_SHORT: [MicroStep; 2] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: None,
        base_clocks: 4,
    },
    common::RETIRE_STANDARD,
];

/// Untaken word branch execution steps (12 CPU clocks / 6 CCKs)
pub static STEPS_BRANCH_NOT_TAKEN_WORD: [MicroStep; 3] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: None,
        base_clocks: 4,
    },
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];

#[inline(always)]
pub fn alu_bcc_short(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let cond = ((state.ir >> 8) & 0x0F) as u8;
    if state.eval_condition(cond) {
        let d8 = (state.ir & 0xFF) as i8;
        let base_pc = state.pc.wrapping_sub(2);
        state.micro.ea_addr = base_pc.wrapping_add(d8 as i32 as u32);
        state.micro.current_steps = &STEPS_BRANCH_TAKEN;
    } else {
        state.micro.current_steps = &STEPS_BRANCH_NOT_TAKEN_SHORT;
    }
    state.micro.micro_step = 0;
}

#[inline(always)]
pub fn alu_bcc_word(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let cond = ((state.ir >> 8) & 0x0F) as u8;
    if state.eval_condition(cond) {
        let disp = state.prefetch[0] as i16 as i32;
        let base_pc = state.pc.wrapping_sub(2);
        state.micro.ea_addr = base_pc.wrapping_add(disp as u32);
        state.micro.current_steps = &STEPS_BRANCH_TAKEN;
    } else {
        state.micro.ea_addr = state.pc;
        state.micro.current_steps = &STEPS_BRANCH_NOT_TAKEN_WORD;
    }
    state.micro.micro_step = 0;
}

/// Bcc.S condition evaluation step
pub static STEPS_BCC_SHORT: [MicroStep; 1] = [MicroStep {
    action: MicroAction::BranchEval,
    alu_fn: Some(alu_bcc_short),
    base_clocks: 0,
}];

/// Bcc.W condition evaluation step
pub static STEPS_BCC_WORD: [MicroStep; 1] = [MicroStep {
    action: MicroAction::BranchEval,
    alu_fn: Some(alu_bcc_word),
    base_clocks: 0,
}];

/// Compile-time opcode decoder for Bcc ($6200..=$6FFF)
pub const fn decode_bcc_steps(d8: u8) -> &'static [MicroStep] {
    if d8 != 0 {
        &STEPS_BCC_SHORT
    } else {
        &STEPS_BCC_WORD
    }
}

/// Evaluates Bcc branch condition and returns the target PC if taken (for external queries)
#[inline]
pub fn evaluate_bcc(state: &CpuState, cond: u8, base_pc: u32, displacement: i32) -> Option<u32> {
    if state.eval_condition(cond) {
        let target = base_pc.wrapping_add(displacement as u32) & 0x00FF_FFFF;
        Some(target)
    } else {
        None
    }
}


