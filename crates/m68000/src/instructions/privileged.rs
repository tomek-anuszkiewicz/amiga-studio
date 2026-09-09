//! Privileged and System Control Instruction Handlers
//!
//! Implements cycle-exact handlers for M68000 privileged and status control operations:
//! - `MOVE An, USP` and `MOVE USP, An` (4 clocks / 2 CCKs)
//! - `STOP #<imm>` (4 clocks / 2 CCKs)
//! - `RESET` (132 clocks / 66 CCKs)
//! - `RTE` (20 clocks / 10 CCKs)
//! - `RTR` (20 clocks / 10 CCKs)
//! - `TRAPV` (4 clocks without trap, 34 clocks with Vector 7 trap)

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
    step_fn: None,
    alu_fn: Some(alu_move_to_usp),
    base_clocks: 0,
};

pub static STEPS_MOVE_TO_USP: [MicroStep; 3] = [
    ALU_MOVE_TO_USP,
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
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
    step_fn: None,
    alu_fn: Some(alu_move_from_usp),
    base_clocks: 0,
};

pub static STEPS_MOVE_FROM_USP: [MicroStep; 3] = [
    ALU_MOVE_FROM_USP,
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];

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
    step_fn: None,
    alu_fn: Some(alu_stop),
    base_clocks: 0,
};

pub static STEPS_STOP: [MicroStep; 2] = [ALU_STOP, common::ALU_IDLE_4CLK];

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
    common::PREFETCH_NEXT_RETIRE,
];

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

// ============================================================================
// RTR Handler (Return and Restore Condition Codes)
// ============================================================================

pub static STEPS_RTR: [MicroStep; 10] = [
    common::POP_STACK_SR_READ,
    common::POP_STACK_CCR_FINISH,
    common::POP_STACK_HIGH_READ,
    common::POP_STACK_HIGH_FINISH,
    common::POP_STACK_LOW_READ,
    common::POP_STACK_LOW_FINISH,
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

// ============================================================================
// TRAPV Handler (Trap on Overflow)
// ============================================================================

pub fn alu_trapv_init(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    if state.get_v() {
        state.micro.current_steps = &STEPS_TRAPV_EXCEPTION;
        state.micro.micro_step = 0;
        state.micro.clocks_remaining = 0;
    }
}

pub static ALU_TRAPV_INIT: MicroStep = MicroStep {
    step_fn: None,
    alu_fn: Some(alu_trapv_init),
    base_clocks: 0,
};

pub static STEPS_TRAPV: [MicroStep; 3] = [
    ALU_TRAPV_INIT,
    common::PREFETCH_NEXT_READ,
    common::PREFETCH_NEXT_RETIRE,
];

pub fn alu_trapv_exception_init(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let vector_addr = 0x0000_001C; // Vector 7 (28)
    let return_pc = state.pc.wrapping_sub(2);
    let old_sr = state.sr;
    state.set_supervisor(true);
    state.sr &= !0x8000;
    state.micro.source = return_pc;
    state.micro.destination = old_sr as u32;
    state.micro.ea_addr = vector_addr;
}

pub static ALU_TRAPV_EXCEPTION_INIT: MicroStep = MicroStep {
    step_fn: None,
    alu_fn: Some(alu_trapv_exception_init),
    base_clocks: 2,
};

pub static STEPS_TRAPV_EXCEPTION: [MicroStep; 17] = [
    ALU_TRAPV_EXCEPTION_INIT,
    common::ALU_IDLE,
    common::EXCEPTION_PUSH_PCLO_IDLE,
    common::EXCEPTION_PUSH_PCLO_WRITE,
    common::EXCEPTION_PUSH_SR_IDLE,
    common::EXCEPTION_PUSH_SR_WRITE,
    common::EXCEPTION_PUSH_PCHI_IDLE,
    common::EXCEPTION_PUSH_PCHI_WRITE,
    common::READ_VECTOR_HIGH_READ,
    common::BUS_READ_IDLE,
    common::READ_VECTOR_LOW_READ,
    common::READ_VECTOR_LOW_FINISH,
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];
