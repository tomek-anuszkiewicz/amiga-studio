//! NOP (No Operation) instruction handler
//!
//! Advances program counter and initiates standard instruction prefetch.
//! Execution time: 4 CPU clocks (2 CCKs).

use crate::micro::common;
use crate::micro::types::MicroStep;

/// Micro-step execution sequence for `NOP` (4 clocks / 2 CCKs)
pub static STEPS_NOP: [MicroStep; 2] = [common::PREFETCH_NEXT_READ, common::BUS_READ_IDLE];
