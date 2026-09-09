//! Motorola 68000 Micro-Step State Machine Execution Engine
//!
//! Models cycle-exact 2-phase Color Clock execution (CCK1 and CCK2) per 4-clock
//! CPU bus cycle, driving atomic MicroSteps directly from pre-compiled slices.

use super::types::{
    default_empty_steps, MicroStep, OpcodeDescriptor, RecordedTransaction, EMPTY_STEPS,
};
use memory_bus::{BusAccessSize, CckPhase};
use serde::{Deserialize, Serialize};

/// Sub-cycle execution micro-state of the M68000 CPU
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuMicroState {
    /// Current Color Clock phase (CCK1 or CCK2)
    pub phase: CckPhase,
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
    /// Optional transaction log for cycle-exact verification (disabled by default)
    #[serde(skip)]
    pub transaction_log: Option<Vec<RecordedTransaction>>,
    /// Wait cycles accumulated during the currently active bus cycle
    #[serde(default)]
    pub current_cycle_wait_cycles: u32,

    // --- Micro-Step State Machine Fields ---
    /// Decoded source operand buffer for ALU operations
    #[serde(default)]
    pub source: u32,
    /// Decoded destination operand buffer and ALU output result buffer
    #[serde(default)]
    pub destination: u32,
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
            last_read: 0,
            scratch_prefetch: 0,
            internal_clocks: 0,
            micro_step: 0,
            scratch: [0; 4],
            transaction_log: None,
            current_cycle_wait_cycles: 0,
            source: 0,
            destination: 0,
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
        self.last_read = 0;
        self.scratch_prefetch = 0;
        self.internal_clocks = 0;
        self.micro_step = 0;
        self.scratch = [0; 4];
        self.current_cycle_wait_cycles = 0;
        self.source = 0;
        self.destination = 0;
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

    /// Helper to record a completed bus cycle into the transaction log
    #[inline]
    pub fn record_bus_transaction(
        &mut self,
        is_read: bool,
        is_tas: bool,
        fc: u8,
        addr: u32,
        size: BusAccessSize,
        data: u16,
        uds: bool,
        lds: bool,
    ) {
        if let Some(ref mut log) = self.transaction_log {
            let duration = 4u32.wrapping_add(self.current_cycle_wait_cycles.wrapping_mul(2));
            log.push(RecordedTransaction::Bus {
                is_read,
                is_tas,
                duration,
                fc,
                addr: addr & 0x00FF_FFFF,
                size,
                data,
                uds,
                lds,
            });
        }
    }
}
