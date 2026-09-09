//! RESET Instruction Handler
//!
//! Asserts the external RESET line for 124 clock cycles:
//! - Privileged instruction (triggers Privilege Violation if in user mode).
//! - CPU registers and execution state are unaffected.
//!
//! Timing: 132 CPU clocks (66 CCKs).

use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// RESET Handler
// ============================================================================

pub fn alu_reset(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    if !state.is_supervisor() {
        state.micro.current_steps = &common::STEPS_PRIVILEGE_VIOLATION;
        state.micro.micro_step = 0;
        state.micro.clocks_remaining = 0;
    }
}

pub static ALU_RESET: MicroStep = MicroStep {
    step_fn: None,
    alu_fn: Some(alu_reset),
    base_clocks: 0,
};

pub static STEPS_RESET: [MicroStep; 4] = [
    ALU_RESET,
    common::ALU_IDLE_128CLK,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];
