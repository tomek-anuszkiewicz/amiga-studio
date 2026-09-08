//! NOP (No Operation) instruction handler
//!
//! Advances program counter and initiates standard instruction prefetch.
//! Execution time: 4 CPU clocks (2 CCKs).

use crate::core::{Cpu, StepResult};
use crate::micro::common;
use crate::micro::types::MicroStep;
use memory_bus::MemoryBus;

/// Micro-step execution sequence for `NOP` (4 clocks / 2 CCKs)
pub static STEPS_NOP: [MicroStep; 1] = [common::RETIRE_STANDARD];

/// Execution handler for `NOP` (legacy fallback)
pub fn op_nop(cpu: &mut Cpu, _bus: &mut MemoryBus) -> StepResult {
    cpu.initiate_prefetch();
    cpu.state.micro.mark_standard_prefetch_retire();
    StepResult::StepCompleted
}
