//! RTS (Return from Subroutine) instruction handler
//!
//! Pops a 32-bit program counter from the stack and refills the instruction prefetch queue.
//! Execution time: 16 CPU clocks (8 CCKs).

use crate::micro::common;
use crate::micro::types::MicroStep;

/// Cycle-exact micro-step sequence for `RTS` (16 CPU clocks / 8 CCKs)
pub static STEPS_RTS: [MicroStep; 8] = [
    common::POP_STACK_HIGH_READ,
    common::POP_STACK_HIGH_FINISH,
    common::POP_STACK_LOW_READ,
    common::POP_STACK_LOW_FINISH,
    common::READ_TARGET_OPCODE_READ,
    common::READ_TARGET_OPCODE_FINISH,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_RETIRE_2CLK,
];
