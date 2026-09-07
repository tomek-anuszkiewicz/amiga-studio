//! Motorola 68000 CPU micro-state machine and Color Clock (CCK) sub-cycle engine
//!
//! Models 2-phase Color Clock execution (CCK1 and CCK2) per 4-clock CPU bus cycle,
//! tracking active bus cycles, internal ALU cycles, and bus contention wait states.

use crate::core::StepResult;
use memory_bus::{BusAccessSize, BusCycle, CckPhase, MemoryBus, MemoryBusResult};
use serde::{Deserialize, Serialize};

/// A recorded bus or internal transaction captured for cycle-exact verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordedTransaction {
    Bus {
        /// Read (true) vs Write (false)
        is_read: bool,
        /// Indivisible TAS read-modify-write cycle
        is_tas: bool,
        /// Duration of the bus cycle in CPU clocks (4, plus 2 per wait cycle)
        duration: u32,
        /// Function Code bits (1=User Data, 2=User Program, 5=Supervisor Data, 6=Supervisor Program)
        fc: u8,
        /// 24-bit physical memory address
        addr: u32,
        /// Transfer size (Byte or Word)
        size: BusAccessSize,
        /// Data transferred on the bus (0-255 for byte, 0-65535 for word)
        data: u16,
        /// Upper Data Strobe (_UDS)
        uds: bool,
        /// Lower Data Strobe (_LDS)
        lds: bool,
    },
    Internal {
        /// Duration of the internal operation in CPU clocks
        duration: u32,
    },
}

/// Instruction retirement and pipeline refill mode when finishing micro-operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MicroRetireMode {
    /// Instruction is actively progressing through micro-steps
    #[default]
    None,
    /// Standard sequential prefetch: ir = prefetch[0], prefetch[0] = last_read, pc += 2
    StandardPrefetch,
    /// RMW / Stack push: ir = prefetch[0], prefetch[0] = scratch_prefetch, pc += 2
    ScratchPrefetch,
    /// Taken branch / jump target refill: ir = new_ir, prefetch[0] = last_read, pc = target + 4
    TargetRefill { target: u32, new_ir: u16 },
}

/// Sub-cycle execution micro-state of the M68000 CPU
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuMicroState {
    /// Current Color Clock phase (CCK1 or CCK2)
    pub phase: CckPhase,
    /// In-flight structured bus cycle (if awaiting memory response or holding wait states)
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
    pub scratch: [u32; 2],
    /// Pipeline retirement mode upon concluding the current in-flight cycle
    #[serde(default)]
    pub retire_mode: MicroRetireMode,
    /// Optional transaction log for cycle-exact verification (disabled by default)
    #[serde(skip)]
    pub transaction_log: Option<Vec<RecordedTransaction>>,
    /// Wait cycles accumulated during the currently active bus cycle
    #[serde(default)]
    pub current_cycle_wait_cycles: u32,
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
            scratch: [0; 2],
            retire_mode: MicroRetireMode::None,
            transaction_log: None,
            current_cycle_wait_cycles: 0,
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
        self.scratch = [0; 2];
        self.retire_mode = MicroRetireMode::None;
        self.current_cycle_wait_cycles = 0;
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
    #[inline]
    pub fn is_bus_busy(&self) -> bool {
        self.active_bus_cycle.is_some()
    }

    /// Returns whether the current instruction is marked for retirement
    #[inline]
    pub fn is_instruction_done(&self) -> bool {
        self.retire_mode != MicroRetireMode::None
    }

    /// Marks instruction to retire via standard prefetch upon completing the in-flight cycle
    #[inline]
    pub fn mark_standard_prefetch_retire(&mut self) {
        self.retire_mode = MicroRetireMode::StandardPrefetch;
    }

    /// Marks instruction to retire via scratch prefetch (Class 0 RMW / stack)
    #[inline]
    pub fn mark_scratch_prefetch_retire(&mut self) {
        self.retire_mode = MicroRetireMode::ScratchPrefetch;
    }

    /// Marks instruction to retire via target branch refill
    #[inline]
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
    fn record_completed_bus_cycle(&mut self, cycle: &BusCycle) {
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

    /// Advances the micro-state machine by exactly 1 Color Clock (CCK)
    pub fn step_cck(&mut self, bus: &mut MemoryBus, wait_cycles: &mut u32) -> StepResult {
        if let Some(mut cycle) = self.active_bus_cycle {
            match self.phase {
                CckPhase::Cck1 => {
                    match bus.begin_cycle(&mut cycle) {
                        MemoryBusResult::Blocked => {
                            // Target bus occupied by Agnus DMA: Gary withholds _DTACK, CPU stalls
                            self.current_cycle_wait_cycles = self.current_cycle_wait_cycles.wrapping_add(1);
                            *wait_cycles = wait_cycles.wrapping_add(1);
                            StepResult::WaitState
                        }
                        MemoryBusResult::Phase1Ready => {
                            self.active_bus_cycle = Some(cycle);
                            self.phase = CckPhase::Cck2;
                            StepResult::StepCompleted
                        }
                        MemoryBusResult::Ready(data) => {
                            // Immediate transaction completion
                            if cycle.is_read {
                                self.last_read = data;
                            }
                            self.record_completed_bus_cycle(&cycle);
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
                            self.current_cycle_wait_cycles = self.current_cycle_wait_cycles.wrapping_add(1);
                            *wait_cycles = wait_cycles.wrapping_add(1);
                            StepResult::WaitState
                        }
                        MemoryBusResult::Ready(data) => {
                            if cycle.is_read {
                                self.last_read = data;
                            }
                            self.record_completed_bus_cycle(&cycle);
                            self.active_bus_cycle = None;
                            self.phase = CckPhase::Cck1;
                            self.micro_step = self.micro_step.wrapping_add(1);
                            StepResult::StepCompleted
                        }
                        MemoryBusResult::Phase1Ready => {
                            self.record_completed_bus_cycle(&cycle);
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
            if self.internal_clocks == 0 {
                self.micro_step = self.micro_step.wrapping_add(1);
            }
            StepResult::StepCompleted
        } else {
            StepResult::StepCompleted
        }
    }
}

