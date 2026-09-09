//! BSR (Branch to Subroutine) instruction handler
//!
//! Subroutine branch pushing the return PC onto the stack.
//! Supports 8-bit short and 16-bit word displacements.
//! Execution time: 18 CPU clocks (2 internal + 2 stack writes + 2 prefetch).

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

#[inline(always)]
pub fn alu_bsr_short(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let d8 = (state.ir & 0xFF) as i8;
    let base_pc = state.pc.wrapping_sub(2);
    state.micro.ea_addr = base_pc.wrapping_add(d8 as i32 as u32);
    state.micro.destination = base_pc; // return_pc = opcode_pc + 2
}

#[inline(always)]
pub fn alu_bsr_word(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let disp = state.prefetch[0] as i16 as i32;
    let base_pc = state.pc.wrapping_sub(2);
    state.micro.ea_addr = base_pc.wrapping_add(disp as u32);
    state.micro.destination = state.pc; // return_pc = opcode_pc + 4
}

/// BSR.S (8-bit short displacement, 18 CPU clocks / 9 CCKs)
pub static STEPS_BSR_SHORT: [MicroStep; 9] = [
    MicroStep {
        step_fn: Cpu::step_alu,
        alu_fn: Some(alu_bsr_short),
        base_clocks: 2,
    },
    common::PUSH_STACK_HIGH_IDLE,
    common::PUSH_STACK_HIGH_WRITE,
    common::BUS_WRITE_IDLE,
    common::PUSH_STACK_LOW_WRITE,
    common::READ_TARGET_OPCODE_READ,
    common::READ_TARGET_OPCODE_FINISH,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_RETIRE_2CLK,
];

/// BSR.W (16-bit word displacement, 18 CPU clocks / 9 CCKs)
pub static STEPS_BSR_WORD: [MicroStep; 9] = [
    MicroStep {
        step_fn: Cpu::step_alu,
        alu_fn: Some(alu_bsr_word),
        base_clocks: 2,
    },
    common::PUSH_STACK_HIGH_IDLE,
    common::PUSH_STACK_HIGH_WRITE,
    common::BUS_WRITE_IDLE,
    common::PUSH_STACK_LOW_WRITE,
    common::READ_TARGET_OPCODE_READ,
    common::READ_TARGET_OPCODE_FINISH,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_RETIRE_2CLK,
];

/// Compile-time opcode decoder for BSR ($6100..=$61FF)
pub const fn decode_bsr_steps(d8: u8) -> &'static [MicroStep] {
    if d8 != 0 {
        &STEPS_BSR_SHORT
    } else {
        &STEPS_BSR_WORD
    }
}
