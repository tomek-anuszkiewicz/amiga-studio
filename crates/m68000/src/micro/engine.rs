//! Motorola 68000 Micro-Step State Machine Execution Engine
//!
//! Models cycle-exact 2-phase Color Clock execution (CCK1 and CCK2) per 4-clock
//! CPU bus cycle, driving atomic MicroSteps directly from pre-compiled slices.

use super::types::{
    default_empty_steps, MicroRetireMode, MicroStep, OpcodeDescriptor, RecordedTransaction,
    EMPTY_STEPS,
};
use crate::core::StepResult;
use memory_bus::{BusCycle, CckPhase, MemoryBus, MemoryBusResult};
use serde::{Deserialize, Serialize};

/// Sub-cycle execution micro-state of the M68000 CPU
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuMicroState {
    /// Current Color Clock phase (CCK1 or CCK2)
    pub phase: CckPhase,
    /// In-flight structured bus cycle
    pub active_bus_cycle: Option<BusCycle>,
    /// Last 16-bit word received from completed bus read cycle
    #[serde(default)]
    pub last_read: u16,
    /// Intermediate latched prefetch word (e.g. for Class 0 RMW where prefetch precedes write)
    #[serde(default)]
    pub scratch_prefetch: u16,
    /// Internal execution CPU clocks remaining (non-bus micro-operations)
    pub internal_clocks: u16,
    /// Step index within the current instruction's micro-operation sequence
    pub micro_step: u16,
    /// Intermediate temporary registers for multi-step micro-operations
    pub scratch: [u32; 4],
    /// Pipeline retirement mode upon concluding the current in-flight cycle
    #[serde(default)]
    pub retire_mode: MicroRetireMode,
    /// Optional transaction log for cycle-exact verification (disabled by default)
    #[serde(skip)]
    pub transaction_log: Option<Vec<RecordedTransaction>>,
    /// Wait cycles accumulated during the currently active bus cycle
    #[serde(default)]
    pub current_cycle_wait_cycles: u32,

    // --- Micro-Step State Machine Fields ---
    /// Hardware Data Output Buffer (DOB) holding ALU result for memory writes
    #[serde(default)]
    pub write_buffer: u32,
    /// Resolved effective memory address for operands or branch/jump targets
    #[serde(default)]
    pub ea_addr: u32,
    /// Pre-decoded source register index (0..7 for Dn/An)
    #[serde(default)]
    pub reg_src: u8,
    /// Pre-decoded destination register index (0..7 for Dn/An)
    #[serde(default)]
    pub reg_dst: u8,
    /// Cached pointer to the active opcode's slice of MicroSteps
    #[serde(skip, default = "default_empty_steps")]
    pub current_steps: &'static [MicroStep],
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
            last_read: 0,
            scratch_prefetch: 0,
            internal_clocks: 0,
            micro_step: 0,
            scratch: [0; 4],
            retire_mode: MicroRetireMode::None,
            transaction_log: None,
            current_cycle_wait_cycles: 0,
            write_buffer: 0,
            ea_addr: 0,
            reg_src: 0,
            reg_dst: 0,
            current_steps: &EMPTY_STEPS,
        }
    }

    /// Resets the micro-state machine to initial power-on / reset state
    pub fn reset(&mut self) {
        self.phase = CckPhase::Cck1;
        self.active_bus_cycle = None;
        self.last_read = 0;
        self.scratch_prefetch = 0;
        self.internal_clocks = 0;
        self.micro_step = 0;
        self.scratch = [0; 4];
        self.retire_mode = MicroRetireMode::None;
        self.current_cycle_wait_cycles = 0;
        self.write_buffer = 0;
        self.ea_addr = 0;
        self.reg_src = 0;
        self.reg_dst = 0;
        self.current_steps = &EMPTY_STEPS;
    }

    /// Initializes active micro-steps for a new instruction
    #[inline(always)]
    pub fn initiate_instruction(&mut self, desc: &OpcodeDescriptor) {
        self.current_steps = desc.steps;
        self.reg_src = desc.reg_src;
        self.reg_dst = desc.reg_dst;
        self.micro_step = 0;
        self.phase = CckPhase::Cck1;
        self.retire_mode = MicroRetireMode::None;
    }

    /// Enables or disables transaction recording
    #[inline]
    pub fn enable_transaction_recording(&mut self, enabled: bool) {
        if enabled {
            self.transaction_log = Some(Vec::new());
        } else {
            self.transaction_log = None;
        }
    }

    /// Records an internal CPU operation (no bus transaction)
    #[inline]
    pub fn record_internal_clocks(&mut self, clocks: u16) {
        self.internal_clocks = clocks;
        if let Some(ref mut log) = self.transaction_log {
            log.push(RecordedTransaction::Internal {
                duration: clocks as u32,
            });
        }
    }

    /// Returns whether an external bus transaction is currently in flight
    #[inline(always)]
    pub fn is_bus_busy(&self) -> bool {
        self.active_bus_cycle.is_some()
    }

    /// Marks instruction to retire via standard prefetch upon completing the in-flight cycle
    #[inline(always)]
    pub fn mark_standard_prefetch_retire(&mut self) {
        self.retire_mode = MicroRetireMode::StandardPrefetch;
    }

    /// Marks instruction to retire via scratch prefetch (Class 0 RMW / stack)
    #[inline(always)]
    pub fn mark_scratch_prefetch_retire(&mut self) {
        self.retire_mode = MicroRetireMode::ScratchPrefetch;
    }

    /// Marks instruction to retire via target branch refill
    #[inline(always)]
    pub fn mark_target_refill_retire(&mut self, target: u32, new_ir: u16) {
        self.retire_mode = MicroRetireMode::TargetRefill { target, new_ir };
    }

    /// Initiates a structured bus cycle, scheduling it on the micro-state machine
    pub fn initiate_bus_cycle(&mut self, cycle: BusCycle) {
        self.active_bus_cycle = Some(cycle);
        self.phase = CckPhase::Cck1;
        self.current_cycle_wait_cycles = 0;
    }

    /// Helper to record a completed bus cycle into the transaction log
    #[inline]
    pub fn record_completed_bus_cycle(&mut self, cycle: &BusCycle) {
        if let Some(ref mut log) = self.transaction_log {
            let duration = 4u32.wrapping_add(self.current_cycle_wait_cycles.wrapping_mul(2));
            let bus_data = if cycle.is_read {
                self.last_read
            } else {
                cycle.data
            };
            log.push(RecordedTransaction::Bus {
                is_read: cycle.is_read,
                is_tas: false,
                duration,
                fc: cycle.fc,
                addr: cycle.addr & 0x00FF_FFFF,
                size: cycle.size,
                data: bus_data,
                uds: cycle.uds,
                lds: cycle.lds,
            });
        }
    }

    /// Finalizes a completed bus cycle, latching read data and advancing step index
    #[inline(always)]
    fn finish_bus_cycle(&mut self, cycle: &BusCycle, data: u16) {
        if cycle.is_read {
            self.last_read = data;
            if let Some(step) = self.current_steps.get(self.micro_step as usize) {
                if step.action == super::types::MicroAction::BusReadLongLow {
                    self.scratch[1] = self.scratch[0] | (data as u32);
                }
            }
        }
        self.record_completed_bus_cycle(cycle);
        self.active_bus_cycle = None;
        self.phase = CckPhase::Cck1;
        let is_movem = self
            .current_steps
            .get(self.micro_step as usize)
            .is_some_and(|s| s.action == super::types::MicroAction::MovemTransfer);
        if !is_movem {
            self.micro_step = self.micro_step.wrapping_add(1);
        }
    }

    /// Advances the in-flight bus cycle or internal clocks by exactly 1 Color Clock (CCK)
    pub fn step_cck(&mut self, bus: &mut MemoryBus, wait_cycles: &mut u32) -> StepResult {
        if let Some(mut cycle) = self.active_bus_cycle {
            match self.phase {
                CckPhase::Cck1 => match bus.begin_cycle(&mut cycle) {
                    MemoryBusResult::Blocked => {
                        self.current_cycle_wait_cycles =
                            self.current_cycle_wait_cycles.wrapping_add(1);
                        *wait_cycles = wait_cycles.wrapping_add(1);
                        StepResult::WaitState
                    }
                    MemoryBusResult::Phase1Ready => {
                        self.active_bus_cycle = Some(cycle);
                        self.phase = CckPhase::Cck2;
                        StepResult::StepCompleted
                    }
                    MemoryBusResult::Ready(data) => {
                        self.finish_bus_cycle(&cycle, data);
                        StepResult::StepCompleted
                    }
                },
                CckPhase::Cck2 => match bus.end_cycle(&mut cycle) {
                    MemoryBusResult::Blocked => {
                        self.current_cycle_wait_cycles =
                            self.current_cycle_wait_cycles.wrapping_add(1);
                        *wait_cycles = wait_cycles.wrapping_add(1);
                        StepResult::WaitState
                    }
                    MemoryBusResult::Ready(data) => {
                        self.finish_bus_cycle(&cycle, data);
                        StepResult::StepCompleted
                    }
                    MemoryBusResult::Phase1Ready => {
                        self.finish_bus_cycle(&cycle, 0);
                        StepResult::StepCompleted
                    }
                },
            }
        } else if self.internal_clocks > 0 {
            // 2 CPU clocks = 1 CCK cycle
            self.internal_clocks = self.internal_clocks.saturating_sub(2);
            self.phase = self.phase.next();
            if self.internal_clocks == 0 {
                self.micro_step = self.micro_step.wrapping_add(1);
            }
            StepResult::StepCompleted
        } else {
            StepResult::StepCompleted
        }
    }
}
