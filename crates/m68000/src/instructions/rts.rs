//! RTS (Return from Subroutine) instruction handler
//!
//! Pops a 32-bit program counter from the stack and refills the instruction prefetch queue.
//! Execution time: 16 CPU clocks (8 CCKs).

use crate::micro::common;
use crate::micro::types::MicroStep;

/// Cycle-exact micro-step sequence for `RTS` (16 CPU clocks / 8 CCKs)
pub static STEPS_RTS: [MicroStep; 4] = [
    common::POP_STACK_HIGH,
    common::POP_STACK_LOW,
    common::READ_TARGET_OPCODE,
    common::PREFETCH_TARGET_RETIRE,
];
