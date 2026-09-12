//! TRAP instruction handler
//!
//! Initiates exception processing for TRAP #0..15 (vectors 32..47).
//! Pushes return PC and SR to the supervisor stack and jumps through vector.
//! Execution time: 34 CPU clocks (17 CCKs).

use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

/// Initial setup for TRAP exception: saves old SR, switches to supervisor
pub fn alu_trap_init(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let opcode = state.ir;
    let vec = (opcode & 0x000F) as u32;
    let vector_addr = 0x0000_0080 + vec * 4;
    let base_pc = state.pc.wrapping_sub(2);
    let return_pc = base_pc;
    let old_sr = state.sr;
    // Switch to supervisor mode (S=1, T=0)
    state.set_supervisor(true);
    state.sr &= !0x8000;
    state.micro.source = return_pc;
    state.micro.destination = old_sr as u32;
    state.micro.ea_addr = vector_addr;
}

pub static ALU_TRAP_INIT: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_trap_init),
    base_clocks: 2,
};

/// Microcode pipeline for `TRAP #<vector>` (34 CPU clocks / 17 CCKs)
pub static STEPS_TRAP: [MicroStep; 17] = [
    ALU_TRAP_INIT,
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
