//! NOP (No Operation) instruction handler
//!
//! Advances program counter and initiates standard instruction prefetch.
//! Execution time: 4 CPU clocks (2 CCKs).

use crate::core::{Cpu, StepResult};
use memory_bus::MemoryBus;

/// Execution handler for `NOP` (4 CPU clocks / 2 CCKs)
#[inline]
pub fn op_nop(cpu: &mut Cpu, _bus: &mut MemoryBus) -> StepResult {
    cpu.initiate_prefetch();
    cpu.state.micro.mark_standard_prefetch_retire();
    StepResult::StepCompleted
}
