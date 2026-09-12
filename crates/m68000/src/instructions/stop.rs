//! STOP Instruction Handler
//!
//! Loads immediate operand into status register (SR) and stops processor execution:
//! - Privileged instruction (triggers Privilege Violation if in user mode).
//! - Sets `state.stopped = true`.
//! - Execution resumes on trace, interrupt, or reset.
//!
//! Timing: 4 CPU clocks (2 CCKs).

use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// STOP #<imm> Handler
// ============================================================================

pub fn alu_stop(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    if !state.is_supervisor() {
        state.micro.current_steps = &common::STEPS_PRIVILEGE_VIOLATION;
        state.micro.micro_step = 0;
        state.micro.clocks_remaining = 0;
        return;
    }
    let imm = state.prefetch[0];
    state.set_sr(imm);
    state.stopped = true;
    state.micro.prefetch_retired = true;
}

pub static ALU_STOP: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_stop),
    base_clocks: 0,
};

pub static STEPS_STOP: [MicroStep; 2] = [ALU_STOP, common::ALU_IDLE_4CLK];
