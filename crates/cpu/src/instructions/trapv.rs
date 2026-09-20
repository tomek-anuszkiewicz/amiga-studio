//! TRAPV (Trap on Overflow) Instruction Handler
//!
//! If the overflow (V) condition code flag is set, initiates exception processing
//! for Vector 7 (address $001C). Otherwise, normal 4-clock execution.
//!
//! Timing: 4 clocks without trap, 34 clocks with Vector 7 trap.

use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::{vector, CpuState, SR_T};

// ============================================================================
// Micro-Step ALU Callbacks: TRAPV
// ============================================================================

pub fn alu_trapv_init(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    if state.is_v() {
        state.micro.current_steps = &STEPS_TRAPV_EXCEPTION;
        state.micro.micro_step = 0;
        state.micro.clocks_remaining = 0;
    }
}

pub static ALU_TRAPV_INIT: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_trapv_init),
    base_clocks: 0,
};

pub static STEPS_TRAPV: [MicroStep; 3] = [
    ALU_TRAPV_INIT,
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
];

pub fn alu_trapv_exception_init(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let vector_addr = vector::addr(vector::TRAPV);
    let return_pc = state.pc.wrapping_sub(2);
    let old_sr = state.sr;
    state.set_supervisor(true);
    state.sr &= !SR_T;
    state.micro.source = return_pc;
    state.micro.destination = old_sr as u32;
    state.micro.ea_addr = vector_addr;
}

pub static ALU_TRAPV_EXCEPTION_INIT: MicroStep = MicroStep {
    bus_fn: None,
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
