//! Motorola 68000 CPU micro-state machine and Color Clock (CCK) sub-cycle engine
//!
//! Models 2-phase Color Clock execution (CCK1 and CCK2) per 4-clock CPU bus cycle,
//! tracking active bus cycles, internal ALU cycles, and bus contention wait states.

use crate::core::StepResult;
use memory_bus::{BusCycle, CckPhase, MemoryBus, MemoryBusResult};
use serde::{Deserialize, Serialize};

/// Sub-cycle execution micro-state of the M68000 CPU
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuMicroState {
    /// Current Color Clock phase (CCK1 or CCK2)
    pub phase: CckPhase,
    /// In-flight structured bus cycle (if awaiting memory response or holding wait states)
    pub active_bus_cycle: Option<BusCycle>,
    /// Internal execution CPU clocks remaining (non-bus micro-operations)
    pub internal_clocks: u16,
    /// Step index within the current instruction's micro-operation sequence
    pub micro_step: u16,
    /// Intermediate temporary registers for multi-step micro-operations
    pub scratch: [u32; 2],
}

impl Default for CpuMicroState {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuMicroState {
    /// Creates a new CPU micro-state initialized to CCK1 with no active transactions
    pub fn new() -> Self {
        Self {
            phase: CckPhase::Cck1,
            active_bus_cycle: None,
            internal_clocks: 0,
            micro_step: 0,
            scratch: [0; 2],
        }
    }

    /// Resets the micro-state machine to initial power-on / reset state
    pub fn reset(&mut self) {
        self.phase = CckPhase::Cck1;
        self.active_bus_cycle = None;
        self.internal_clocks = 0;
        self.micro_step = 0;
        self.scratch = [0; 2];
    }

    /// Returns whether an external bus transaction is currently in flight
    #[inline]
    pub fn is_bus_busy(&self) -> bool {
        self.active_bus_cycle.is_some()
    }

    /// Initiates a structured bus cycle, scheduling it on the micro-state machine
    pub fn initiate_bus_cycle(&mut self, cycle: BusCycle) {
        self.active_bus_cycle = Some(cycle);
        self.phase = CckPhase::Cck1;
    }

    /// Advances the micro-state machine by exactly 1 Color Clock (CCK)
    pub fn step_cck(&mut self, bus: &mut MemoryBus, wait_cycles: &mut u32) -> StepResult {
        if let Some(mut cycle) = self.active_bus_cycle {
            match self.phase {
                CckPhase::Cck1 => {
                    match bus.begin_cycle(&mut cycle) {
                        MemoryBusResult::Blocked => {
                            // Target bus occupied by Agnus DMA: Gary withholds _DTACK, CPU stalls
                            *wait_cycles = wait_cycles.wrapping_add(1);
                            StepResult::WaitState
                        }
                        MemoryBusResult::Phase1Ready => {
                            self.active_bus_cycle = Some(cycle);
                            self.phase = CckPhase::Cck2;
                            StepResult::StepCompleted
                        }
                        MemoryBusResult::Ready(_) => {
                            // Immediate transaction completion
                            self.active_bus_cycle = None;
                            self.phase = CckPhase::Cck1;
                            self.micro_step = self.micro_step.wrapping_add(1);
                            StepResult::StepCompleted
                        }
                    }
                }
                CckPhase::Cck2 => {
                    match bus.end_cycle(&mut cycle) {
                        MemoryBusResult::Blocked => {
                            // Write blocked at CCK2 by Agnus DMA
                            *wait_cycles = wait_cycles.wrapping_add(1);
                            StepResult::WaitState
                        }
                        MemoryBusResult::Ready(_) => {
                            self.active_bus_cycle = None;
                            self.phase = CckPhase::Cck1;
                            self.micro_step = self.micro_step.wrapping_add(1);
                            StepResult::StepCompleted
                        }
                        MemoryBusResult::Phase1Ready => {
                            self.active_bus_cycle = None;
                            self.phase = CckPhase::Cck1;
                            self.micro_step = self.micro_step.wrapping_add(1);
                            StepResult::StepCompleted
                        }
                    }
                }
            }
        } else if self.internal_clocks > 0 {
            // 2 CPU clocks = 1 CCK cycle
            self.internal_clocks = self.internal_clocks.saturating_sub(2);
            self.phase = self.phase.next();
            StepResult::StepCompleted
        } else {
            StepResult::StepCompleted
        }
    }
}
