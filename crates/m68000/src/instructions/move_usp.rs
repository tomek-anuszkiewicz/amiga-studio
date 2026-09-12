//! MOVE USP Instruction Handlers (`MOVE An, USP` and `MOVE USP, An`)
//!
//! Transfers data between general address register and user stack pointer:
//! - Privileged instruction (triggers Privilege Violation if in user mode).
//!
//! Timing: 4 CPU clocks (2 CCKs).

use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// MOVE USP Handlers (MOVE An, USP / MOVE USP, An)
// ============================================================================

pub fn alu_move_to_usp(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    if !state.is_supervisor() {
        state.micro.current_steps = &common::STEPS_PRIVILEGE_VIOLATION;
        state.micro.micro_step = 0;
        state.micro.clocks_remaining = 0;
        return;
    }
    state.usp = state.read_a(reg_src as usize);
}

pub static ALU_MOVE_TO_USP: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_move_to_usp),
    base_clocks: 0,
};

pub static STEPS_MOVE_TO_USP: [MicroStep; 3] = [
    ALU_MOVE_TO_USP,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub fn alu_move_from_usp(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    if !state.is_supervisor() {
        state.micro.current_steps = &common::STEPS_PRIVILEGE_VIOLATION;
        state.micro.micro_step = 0;
        state.micro.clocks_remaining = 0;
        return;
    }
    state.set_a_long(reg_dst as usize, state.usp);
}

pub static ALU_MOVE_FROM_USP: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_move_from_usp),
    base_clocks: 0,
};

pub static STEPS_MOVE_FROM_USP: [MicroStep; 3] = [
    ALU_MOVE_FROM_USP,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];
