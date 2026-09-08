//! BRA (Branch Always) instruction handler
//!
//! Unconditional relative branch with 8-bit short or 16-bit word displacement.
//! Execution time: 10 CPU clocks (2 internal idle clocks + 2 bus prefetch cycles).

use crate::micro::common;
use crate::micro::types::{MicroAction, MicroStep};
use crate::state::CpuState;

#[inline(always)]
pub fn alu_bra_short(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d8 = (state.ir & 0xFF) as i8;
    let base_pc = state.pc.wrapping_sub(2);
    state.micro.ea_addr = base_pc.wrapping_add(d8 as i32 as u32);
}

#[inline(always)]
pub fn alu_bra_word(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let disp = state.prefetch[0] as i16 as i32;
    let base_pc = state.pc.wrapping_sub(2);
    state.micro.ea_addr = base_pc.wrapping_add(disp as u32);
}

/// BRA.S (8-bit short displacement, 10 CPU clocks / 5 CCKs)
pub static STEPS_BRA_SHORT: [MicroStep; 3] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(alu_bra_short),
        base_clocks: 2,
    },
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];

/// BRA.W (16-bit word displacement, 10 CPU clocks / 5 CCKs)
pub static STEPS_BRA_WORD: [MicroStep; 3] = [
    MicroStep {
        action: MicroAction::Alu,
        alu_fn: Some(alu_bra_word),
        base_clocks: 2,
    },
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];

/// Compile-time opcode decoder for BRA ($6000..=$60FF)
pub const fn decode_bra_steps(d8: u8) -> &'static [MicroStep] {
    if d8 != 0 {
        &STEPS_BRA_SHORT
    } else {
        &STEPS_BRA_WORD
    }
}


