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
use crate::Cpu;

// ============================================================================
// RESET Handler
// ============================================================================

pub fn alu_reset(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    if !state.is_supervisor() {
        state.micro_mut().current_steps = &common::STEPS_PRIVILEGE_VIOLATION;
        state.micro_mut().micro_step = 0;
        state.micro_mut().clocks_remaining = 0;
    }
}

pub static ALU_RESET: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_reset),
    base_clocks: 0,
};

pub fn alu_assert_reset(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.set_reset_line_asserted(true);
}

pub fn alu_release_reset(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.set_reset_line_asserted(false);
}

pub static RESET_ASSERT_IDLE: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_assert_reset),
    base_clocks: 124,
};

pub static STEPS_RESET: [MicroStep; 5] = [
    ALU_RESET,
    common::ALU_IDLE_4CLK,
    RESET_ASSERT_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_release_reset),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];
