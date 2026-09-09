//! RTR (Return and Restore Condition Codes) Instruction Handler
//!
//! Unprivileged return instruction that restores condition codes (CCR) and program counter (PC):
//! 1. Pops condition code byte from stack into CCR.
//! 2. Pops 32-bit return address from stack into PC.
//!
//! Timing: 20 CPU clocks (10 CCKs).

use crate::micro::common;
use crate::micro::types::MicroStep;

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
