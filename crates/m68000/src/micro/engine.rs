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

    /// Returns whether the current instruction is marked for retirement
    #[inline(always)]
    pub fn is_instruction_done(&self) -> bool {
        self.retire_mode != MicroRetireMode::None
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
                        if cycle.is_read {
                            self.last_read = data;
                            if let Some(step) = self.current_steps.get(self.micro_step as usize) {
                                if step.action == super::types::MicroAction::BusReadLongLow {
                                    self.scratch[1] = self.scratch[0] | (data as u32);
                                }
                            }
                        }
                        self.record_completed_bus_cycle(&cycle);
                        self.active_bus_cycle = None;
                        self.phase = CckPhase::Cck1;
                        let is_movem = self
                            .current_steps
                            .get(self.micro_step as usize)
                            .is_some_and(|s| s.action == super::types::MicroAction::MovemTransfer);
                        if !is_movem {
                            self.micro_step = self.micro_step.wrapping_add(1);
                        }
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
                        if cycle.is_read {
                            self.last_read = data;
                            if let Some(step) = self.current_steps.get(self.micro_step as usize) {
                                if step.action == super::types::MicroAction::BusReadLongLow {
                                    self.scratch[1] = self.scratch[0] | (data as u32);
                                }
                            }
                        }
                        self.record_completed_bus_cycle(&cycle);
                        self.active_bus_cycle = None;
                        self.phase = CckPhase::Cck1;
                        let is_movem = self
                            .current_steps
                            .get(self.micro_step as usize)
                            .is_some_and(|s| s.action == super::types::MicroAction::MovemTransfer);
                        if !is_movem {
                            self.micro_step = self.micro_step.wrapping_add(1);
                        }
                        StepResult::StepCompleted
                    }
                    MemoryBusResult::Phase1Ready => {
                        self.record_completed_bus_cycle(&cycle);
                        self.active_bus_cycle = None;
                        self.phase = CckPhase::Cck1;
                        let is_movem = self
                            .current_steps
                            .get(self.micro_step as usize)
                            .is_some_and(|s| s.action == super::types::MicroAction::MovemTransfer);
                        if !is_movem {
                            self.micro_step = self.micro_step.wrapping_add(1);
                        }
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

/// Helper to execute the CCK1 sub-phase for an in-flight bus transaction
#[inline]
pub fn step_active_bus_cck1(cpu: &mut crate::core::Cpu, bus: &mut MemoryBus) -> StepResult {
    if cpu.state.micro.is_bus_busy() && cpu.state.micro.phase == memory_bus::CckPhase::Cck1 {
        let bus_res = cpu.state.micro.step_cck(bus, &mut cpu.wait_cycles);
        cpu.total_clocks = cpu.total_clocks.wrapping_add(2);
        cpu.instruction_clocks = cpu.instruction_clocks.wrapping_add(2);
        if bus_res.is_wait() {
            return StepResult::WaitState;
        }
        return StepResult::StepCompleted;
    }
    StepResult::StepCompleted
}

/// Triggers a cycle-exact Group 0 Address Error on unaligned word/long access
#[inline(never)]
pub fn trigger_address_error_step(
    cpu: &mut crate::core::Cpu,
    addr: u32,
    is_read: bool,
    is_program_space: bool,
    bus: &mut MemoryBus,
) -> StepResult {
    let fc = if is_program_space {
        super::types::prog_fc(&cpu.state)
    } else {
        super::types::data_fc(&cpu.state)
    };
    cpu.instruction_clocks = cpu.instruction_clocks.wrapping_add(8);
    cpu.total_clocks = cpu.total_clocks.wrapping_add(8);
    cpu.handle_address_error_fc(addr, is_read, fc, bus);
    StepResult::InstructionCompleted
}

#[inline(always)]
fn movem_read_reg(state: &crate::state::CpuState, is_predec: bool, bit_idx: u8) -> u32 {
    if is_predec {
        if bit_idx < 8 {
            state.read_a((7 - bit_idx) as usize)
        } else {
            state.d_long((15 - bit_idx) as usize)
        }
    } else if bit_idx < 8 {
        state.d_long(bit_idx as usize)
    } else {
        state.read_a((bit_idx - 8) as usize)
    }
}

#[inline(always)]
fn movem_write_reg(state: &mut crate::state::CpuState, bit_idx: u8, val: u32) {
    if bit_idx < 8 {
        state.set_d_long(bit_idx as usize, val);
    } else {
        state.write_a((bit_idx - 8) as usize, val);
    }
}

/// Executes the active instruction's micro-step sequence directly
pub fn execute_micro_step(cpu: &mut crate::core::Cpu, bus: &mut MemoryBus) -> StepResult {
    while (cpu.state.micro.micro_step as usize) < cpu.state.micro.current_steps.len() {
        let step = cpu.state.micro.current_steps[cpu.state.micro.micro_step as usize];
        if let Some(alu) = step.alu_fn {
            let reg_src = cpu.state.micro.reg_src;
            let reg_dst = cpu.state.micro.reg_dst;
            alu(&mut cpu.state, reg_src, reg_dst);
        }
        match step.action {
            super::types::MicroAction::Alu => {
                let clocks = if cpu.state.micro.internal_clocks > 0 {
                    cpu.state.micro.internal_clocks
                } else {
                    step.base_clocks as u16
                };
                if clocks > 0 {
                    cpu.state.micro.internal_clocks = clocks.saturating_sub(2);
                    cpu.total_clocks = cpu.total_clocks.wrapping_add(2);
                    cpu.instruction_clocks = cpu.instruction_clocks.wrapping_add(2);
                    if cpu.state.micro.internal_clocks == 0 {
                        cpu.state.micro.micro_step = cpu.state.micro.micro_step.wrapping_add(1);
                    }
                    return StepResult::StepCompleted;
                } else {
                    cpu.state.micro.micro_step = cpu.state.micro.micro_step.wrapping_add(1);
                }
            }
            super::types::MicroAction::BusReadByte => {
                let addr = cpu.state.micro.ea_addr;
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(addr, memory_bus::BusAccessSize::Byte, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusReadWord => {
                let addr = cpu.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    return trigger_address_error_step(cpu, addr, true, false, bus);
                }
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(addr, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusReadLongHigh => {
                let addr = cpu.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    return trigger_address_error_step(cpu, addr, true, false, bus);
                }
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(addr, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusReadLongLow => {
                cpu.state.micro.scratch[0] = (cpu.state.micro.last_read as u32) << 16;
                let addr = cpu.state.micro.ea_addr.wrapping_add(2);
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(addr, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusWriteByte => {
                let addr = cpu.state.micro.ea_addr;
                let val = (cpu.state.micro.write_buffer & 0xFF) as u16;
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_write(addr, val, memory_bus::BusAccessSize::Byte, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusWriteWord => {
                let addr = cpu.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    return trigger_address_error_step(cpu, addr, false, false, bus);
                }
                let val = (cpu.state.micro.write_buffer & 0xFFFF) as u16;
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_write(addr, val, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusWriteLongHigh => {
                let addr = cpu.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    return trigger_address_error_step(cpu, addr, false, false, bus);
                }
                let val = ((cpu.state.micro.write_buffer >> 16) & 0xFFFF) as u16;
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_write(addr, val, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusWriteLongLow => {
                let addr = cpu.state.micro.ea_addr.wrapping_add(2);
                if (addr & 1) != 0 {
                    return trigger_address_error_step(cpu, addr, false, false, bus);
                }
                let val = (cpu.state.micro.write_buffer & 0xFFFF) as u16;
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_write(addr, val, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::FetchExtension => {
                let addr = cpu.state.pc;
                let fc = super::types::prog_fc(&cpu.state);
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(addr, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusPrefetchToScratch => {
                let addr = cpu.state.pc;
                let fc = super::types::prog_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(addr, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::PrefetchNextOpcodeAndRetire => {
                let addr = cpu.state.pc;
                let fc = super::types::prog_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(addr, memory_bus::BusAccessSize::Word, fc));
                cpu.state.micro.mark_standard_prefetch_retire();
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusWriteWordAndRetire => {
                let addr = cpu.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    cpu.state.ir = cpu.state.prefetch[0];
                    return trigger_address_error_step(cpu, addr, false, false, bus);
                }
                let val = (cpu.state.micro.write_buffer & 0xFFFF) as u16;
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_write(addr, val, memory_bus::BusAccessSize::Word, fc));
                cpu.state.micro.mark_scratch_prefetch_retire();
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusWriteByteAndRetire => {
                let addr = cpu.state.micro.ea_addr;
                let val = (cpu.state.micro.write_buffer & 0xFF) as u16;
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_write(addr, val, memory_bus::BusAccessSize::Byte, fc));
                cpu.state.micro.mark_scratch_prefetch_retire();
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusWriteLongLowAndRetire => {
                let addr = cpu.state.micro.ea_addr.wrapping_add(2);
                if (addr & 1) != 0 {
                    return trigger_address_error_step(cpu, addr, false, false, bus);
                }
                let val = (cpu.state.micro.write_buffer & 0xFFFF) as u16;
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_write(addr, val, memory_bus::BusAccessSize::Word, fc));
                cpu.state.micro.mark_scratch_prefetch_retire();
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusWriteLongHighAndRetire => {
                let addr = cpu.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    return trigger_address_error_step(cpu, addr, false, false, bus);
                }
                let val = ((cpu.state.micro.write_buffer >> 16) & 0xFFFF) as u16;
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_write(addr, val, memory_bus::BusAccessSize::Word, fc));
                cpu.state.micro.mark_scratch_prefetch_retire();
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusPopStack => {
                let sp = cpu.state.read_a(7);
                if (sp & 1) != 0 {
                    return trigger_address_error_step(cpu, sp, true, false, bus);
                }
                cpu.state.write_a(7, sp.wrapping_add(2));
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(sp, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusPopStackHigh => {
                let sp = cpu.state.read_a(7);
                if (sp & 1) != 0 {
                    return trigger_address_error_step(cpu, sp, true, false, bus);
                }
                cpu.state.write_a(7, sp.wrapping_add(2));
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(sp, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusPopStackLow => {
                let sp = cpu.state.read_a(7);
                if (sp & 1) != 0 {
                    return trigger_address_error_step(cpu, sp, true, false, bus);
                }
                cpu.state.write_a(7, sp.wrapping_add(2));
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(sp, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusPushStackHigh => {
                let sp = cpu.state.read_a(7).wrapping_sub(4);
                cpu.state.write_a(7, sp);
                if (sp & 1) != 0 {
                    return trigger_address_error_step(cpu, sp, false, false, bus);
                }
                let val = ((cpu.state.micro.write_buffer >> 16) & 0xFFFF) as u16;
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_write(sp, val, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusPushStackLow => {
                let sp_low = cpu.state.read_a(7).wrapping_add(2);
                let val = (cpu.state.micro.write_buffer & 0xFFFF) as u16;
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_write(sp_low, val, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusPushStackLowAndRetire => {
                let sp_low = cpu.state.read_a(7).wrapping_add(2);
                let val = (cpu.state.micro.write_buffer & 0xFFFF) as u16;
                let fc = super::types::data_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_write(sp_low, val, memory_bus::BusAccessSize::Word, fc));
                cpu.state.micro.mark_scratch_prefetch_retire();
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BusReadTargetOpcode => {
                let addr = cpu.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    return trigger_address_error_step(cpu, addr, true, true, bus);
                }
                let fc = super::types::prog_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(addr, memory_bus::BusAccessSize::Word, fc));
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::PrefetchTargetAndRetire => {
                let addr = cpu.state.micro.ea_addr.wrapping_add(2);
                let fc = super::types::prog_fc(&cpu.state);
                cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(addr, memory_bus::BusAccessSize::Word, fc));
                cpu.state.micro.mark_target_refill_retire(cpu.state.micro.ea_addr, cpu.state.micro.scratch_prefetch);
                return step_active_bus_cck1(cpu, bus);
            }
            super::types::MicroAction::BranchEval => {
                continue;
            }
            super::types::MicroAction::MovemTransfer => {
                let ir = cpu.state.ir;
                let is_reg_to_mem = (ir & 0x0400) == 0;
                let is_long = (ir & 0x0040) != 0;
                let mode = ((ir >> 3) & 7) as u8;
                let reg_ea = (ir & 7) as usize;
                let is_predec = is_reg_to_mem && mode == 4;
                let is_postinc = !is_reg_to_mem && mode == 3;
                let fc = super::types::data_fc(&cpu.state);

                let mask = cpu.state.micro.scratch[2] as u16;
                let state_raw = cpu.state.micro.scratch[3];
                let bus_in_flight = (state_raw & 1) != 0;
                let mut bit_idx = ((state_raw >> 1) & 0x1F) as u8;
                let mut sub_word = ((state_raw >> 6) & 1) as u8;
                let dummy_read_active = ((state_raw >> 7) & 1) != 0;

                // 1. Initial check: mask == 0 and address alignment
                if !bus_in_flight && bit_idx == 0 && sub_word == 0 && !dummy_read_active {
                    if mask == 0 {
                        cpu.state.micro.scratch[3] = 0;
                        cpu.state.micro.micro_step = cpu.state.micro.micro_step.wrapping_add(1);
                        continue;
                    }
                    let ea = cpu.state.micro.ea_addr;
                    if (ea & 1) != 0 {
                        if is_predec {
                            return trigger_address_error_step(cpu, ea.wrapping_sub(2), false, false, bus);
                        } else if is_postinc {
                            cpu.state.write_a(reg_ea, ea.wrapping_add(2));
                            return trigger_address_error_step(cpu, ea, true, false, bus);
                        } else {
                            return trigger_address_error_step(cpu, ea, !is_reg_to_mem, false, bus);
                        }
                    }
                }

                // 2. Process finished bus cycle
                if bus_in_flight {
                    if dummy_read_active {
                        if is_postinc {
                            cpu.state.write_a(reg_ea, cpu.state.micro.ea_addr);
                        }
                        cpu.state.micro.scratch[3] = 0;
                        cpu.state.micro.micro_step = cpu.state.micro.micro_step.wrapping_add(1);
                        continue;
                    }

                    if !is_reg_to_mem {
                        if !is_long {
                            let val = (cpu.state.micro.last_read as i16 as i32) as u32;
                            movem_write_reg(&mut cpu.state, bit_idx, val);
                            cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_add(2);
                            bit_idx += 1;
                        } else if sub_word == 0 {
                            cpu.state.micro.scratch[1] = (cpu.state.micro.last_read as u32) << 16;
                            sub_word = 1;
                        } else {
                            let val = cpu.state.micro.scratch[1] | (cpu.state.micro.last_read as u32);
                            movem_write_reg(&mut cpu.state, bit_idx, val);
                            cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_add(4);
                            sub_word = 0;
                            bit_idx += 1;
                        }
                    } else if is_predec {
                        if !is_long {
                            cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_sub(2);
                            bit_idx += 1;
                        } else if sub_word == 0 {
                            sub_word = 1;
                        } else {
                            cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_sub(4);
                            sub_word = 0;
                            bit_idx += 1;
                        }
                    } else if !is_long {
                        cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_add(2);
                        bit_idx += 1;
                    } else if sub_word == 0 {
                        sub_word = 1;
                    } else {
                        cpu.state.micro.ea_addr = cpu.state.micro.ea_addr.wrapping_add(4);
                        sub_word = 0;
                        bit_idx += 1;
                    }
                }

                // 3. Find next register in mask
                if sub_word == 0 {
                    while bit_idx < 16 && (mask & (1 << bit_idx)) == 0 {
                        bit_idx += 1;
                    }
                }

                // 4. Initiate transfer if register found
                if bit_idx < 16 {
                    let new_state = 1 | ((bit_idx as u32) << 1) | ((sub_word as u32) << 6);
                    cpu.state.micro.scratch[3] = new_state;

                    if !is_reg_to_mem {
                        let addr = if sub_word == 0 {
                            cpu.state.micro.ea_addr
                        } else {
                            cpu.state.micro.ea_addr.wrapping_add(2)
                        };
                        cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(addr, memory_bus::BusAccessSize::Word, fc));
                        return step_active_bus_cck1(cpu, bus);
                    } else {
                        let reg_val = movem_read_reg(&cpu.state, is_predec, bit_idx);
                        let (addr, data) = if is_predec {
                            if !is_long || sub_word == 0 {
                                (cpu.state.micro.ea_addr.wrapping_sub(2), (reg_val & 0xFFFF) as u16)
                            } else {
                                (cpu.state.micro.ea_addr.wrapping_sub(4), (reg_val >> 16) as u16)
                            }
                        } else if !is_long {
                            (cpu.state.micro.ea_addr, (reg_val & 0xFFFF) as u16)
                        } else if sub_word == 0 {
                            (cpu.state.micro.ea_addr, (reg_val >> 16) as u16)
                        } else {
                            (cpu.state.micro.ea_addr.wrapping_add(2), (reg_val & 0xFFFF) as u16)
                        };
                        cpu.initiate_bus_cycle(memory_bus::BusCycle::new_write(addr, data, memory_bus::BusAccessSize::Word, fc));
                        return step_active_bus_cck1(cpu, bus);
                    }
                }

                // 5. Conclude transfers
                if !is_reg_to_mem {
                    let new_state = 1 | (16 << 1) | (1 << 7);
                    cpu.state.micro.scratch[3] = new_state;
                    cpu.initiate_bus_cycle(memory_bus::BusCycle::new_read(cpu.state.micro.ea_addr, memory_bus::BusAccessSize::Word, fc));
                    return step_active_bus_cck1(cpu, bus);
                } else {
                    if is_predec {
                        cpu.state.write_a(reg_ea, cpu.state.micro.ea_addr);
                    }
                    cpu.state.micro.scratch[3] = 0;
                    cpu.state.micro.micro_step = cpu.state.micro.micro_step.wrapping_add(1);
                    continue;
                }
            }
            super::types::MicroAction::OriToCcr => return crate::instructions::ori::op_ori_to_ccr(cpu, bus),
            super::types::MicroAction::OriToSr => return crate::instructions::ori::op_ori_to_sr(cpu, bus),
            super::types::MicroAction::AndiToCcr => return crate::instructions::andi::op_andi_to_ccr(cpu, bus),
            super::types::MicroAction::AndiToSr => return crate::instructions::andi::op_andi_to_sr(cpu, bus),
            super::types::MicroAction::EoriToCcr => return crate::instructions::eori::op_eori_to_ccr(cpu, bus),
            super::types::MicroAction::EoriToSr => return crate::instructions::eori::op_eori_to_sr(cpu, bus),
            super::types::MicroAction::Trap => {
                let res = crate::instructions::trap::op_trap(cpu, bus);
                if cpu.state.micro.is_bus_busy() && cpu.state.micro.phase == memory_bus::CckPhase::Cck1 {
                    return step_active_bus_cck1(cpu, bus);
                }
                return res;
            }
        }
    }
    StepResult::InstructionCompleted
}
