//! RTE (Return from Exception) Instruction Handler
//!
//! Privileged return instruction that restores status register (SR) and program counter (PC):
//! 1. Pops 16-bit status word from stack into SR.
//! 2. Pops 32-bit return address from stack into PC.
//!
//! Timing: 20 CPU clocks (10 CCKs) or 34 clocks on privilege violation.

use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// RTE Handler (Return from Exception)
// ============================================================================

pub fn alu_rte_init(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    if !state.is_supervisor() {
        state.micro.current_steps = &common::STEPS_PRIVILEGE_VIOLATION;
        state.micro.micro_step = 0;
        state.micro.clocks_remaining = 0;
    }
}

pub static ALU_RTE_INIT: MicroStep = MicroStep {
    step_fn: None,
    alu_fn: Some(alu_rte_init),
    base_clocks: 0,
};

pub static STEPS_RTE: [MicroStep; 11] = [
    ALU_RTE_INIT,
    common::POP_STACK_SR_READ,
    common::POP_STACK_SR_FINISH,
    common::POP_STACK_HIGH_READ,
    common::POP_STACK_HIGH_FINISH,
    common::POP_STACK_LOW_READ,
    common::POP_STACK_RTE_FINISH,
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];
