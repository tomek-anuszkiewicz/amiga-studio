//! Cycle-exact Motorola 68000 CPU Execution Core

use crate::instructions::system;
use crate::micro::types::{self, MicroAction, MicroRetireMode};
use crate::state::CpuState;
use memory_bus::{BusAccessSize, BusCycle, CckPhase, MemoryBus};

/// Execution result returned by CPU step operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    /// Sub-cycle micro-step completed within the current instruction
    StepCompleted,
    /// Instruction completed execution, committed writeback, and prefetched next opcode
    InstructionCompleted,
    /// CPU stalled due to bus contention / wait state
    WaitState,
    /// CPU entered or is in stopped state (STOP instruction)
    Stopped,
    /// CPU entered halted state (double bus fault)
    Halted,
}

impl StepResult {
    /// Returns true if execution resulted in a wait state / bus stall
    #[inline]
    pub fn is_wait(&self) -> bool {
        matches!(self, StepResult::WaitState)
    }

    /// Returns true if an instruction fully completed and retired
    #[inline]
    pub fn is_completed(&self) -> bool {
        matches!(self, StepResult::InstructionCompleted)
    }
}

/// Motorola 68000 CPU Core
#[derive(Debug, Clone)]
pub struct Cpu {
    pub state: CpuState,
    /// Number of wait-state cycles currently accumulated
    pub wait_cycles: u32,
    /// Total CPU clocks executed (1 CCK = 2 CPU clocks)
    pub total_clocks: u64,
    /// CPU clocks consumed by the current instruction
    pub instruction_clocks: u32,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            state: CpuState::default(),
            wait_cycles: 0,
            total_clocks: 0,
            instruction_clocks: 0,
        }
    }

    /// Reset CPU according to Cold/Warm reset specification
    pub fn reset(&mut self, bus: &mut MemoryBus) {
        self.state.sr = 0x2700;
        self.state.stopped = false;
        self.state.halted = false;
        self.state.step = 0;
        self.state.micro.reset();
        self.wait_cycles = 0;
        self.total_clocks = 0;
        self.instruction_clocks = 0;

        // Fetch initial SSP from $000000
        let ssp_hi = bus.read_word_debug(0x000000);
        let ssp_lo = bus.read_word_debug(0x000002);
        self.state.ssp = ((ssp_hi as u32) << 16) | (ssp_lo as u32);
        self.state.set_a_long(7, self.state.ssp);

        // Fetch initial PC from $000004
        let pc_hi = bus.read_word_debug(0x000004);
        let pc_lo = bus.read_word_debug(0x000006);
        self.state.pc = ((pc_hi as u32) << 16) | (pc_lo as u32);

        // Prime prefetch pipeline
        self.state.ir = bus.read_word_debug(self.state.pc);
        self.state.pc = self.state.pc.wrapping_add(2);
        self.state.prefetch[0] = bus.read_word_debug(self.state.pc);
        self.state.pc = self.state.pc.wrapping_add(2);
        self.state.prefetch[1] = 0;
    }

    /// CCK phase stepping primitive
    pub fn step_cck(&mut self, bus: &mut MemoryBus) -> StepResult {
        if self.state.halted {
            return StepResult::Halted;
        }
        if self.state.stopped {
            return StepResult::Stopped;
        }

        // 1. If an external bus transaction is currently in flight:
        if self.state.micro.is_bus_busy() {
            let res = self.state.micro.step_cck(bus, &mut self.wait_cycles);
            self.total_clocks = self.total_clocks.wrapping_add(2);
            self.instruction_clocks = self.instruction_clocks.wrapping_add(2);

            if res.is_wait() {
                return StepResult::WaitState;
            }

            // If the bus cycle just completed, handle retirement or latch intermediate data
            if !self.state.micro.is_bus_busy() {
                if let Some(completed_res) = self.handle_bus_cycle_completion() {
                    return completed_res;
                }
            }

            return StepResult::StepCompleted;
        }

        // 2. If internal execution clocks remain:
        if self.state.micro.internal_clocks > 0 {
            self.state.micro.internal_clocks = self.state.micro.internal_clocks.saturating_sub(2);
            self.total_clocks = self.total_clocks.wrapping_add(2);
            self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
            if self.state.micro.internal_clocks == 0 {
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
            }
            return StepResult::StepCompleted;
        }

        // Check if current instruction uses static micro-steps:
        if self.state.micro.micro_step == 0 && self.state.micro.current_steps.is_empty() {
            self.initiate_current_instruction();
        }

        self.state.instruction_pc = self.state.pc.wrapping_sub(4);

        // 3. Execute via cycle-exact Micro-Step State Machine:
        if !self.state.micro.current_steps.is_empty() {
            return self.execute_micro_step(bus);
        }

        // Unimplemented / Illegal opcode:
        self.state.halted = true;
        StepResult::Halted
    }

    /// Handles post-bus-cycle latching or instruction retirement
    #[inline]
    fn handle_bus_cycle_completion(&mut self) -> Option<StepResult> {
        match self.state.micro.retire_mode {
            MicroRetireMode::None => {
                // Capture intermediate read data for multi-step sequences
                if !self.state.micro.current_steps.is_empty() {
                    let prev_idx = self.state.micro.micro_step.saturating_sub(1) as usize;
                    if prev_idx < self.state.micro.current_steps.len() {
                        match self.state.micro.current_steps[prev_idx].action {
                            MicroAction::BusPrefetchToScratch => {
                                self.state.micro.scratch_prefetch = self.state.micro.last_read;
                            }
                            MicroAction::FetchExtension => {
                                self.state.prefetch[0] = self.state.micro.last_read;
                            }
                            MicroAction::BusReadLongHigh => {
                                self.state.micro.scratch[0] =
                                    (self.state.micro.last_read as u32) << 16;
                            }
                            MicroAction::BusReadLongLow => {
                                self.state.micro.scratch[1] = self.state.micro.scratch[0]
                                    | (self.state.micro.last_read as u32);
                            }
                            MicroAction::BusReadTargetOpcode => {
                                self.state.micro.scratch_prefetch = self.state.micro.last_read;
                            }
                            MicroAction::BusPopStackHigh => {
                                self.state.micro.scratch[0] =
                                    (self.state.micro.last_read as u32) << 16;
                            }
                            MicroAction::BusPopStackLow => {
                                self.state.micro.ea_addr = self.state.micro.scratch[0]
                                    | (self.state.micro.last_read as u32);
                            }
                            _ => {}
                        }
                    }
                }
                None
            }
            MicroRetireMode::StandardPrefetch => {
                self.state.ir = self.state.prefetch[0];
                self.state.prefetch[0] = self.state.micro.last_read;
                self.state.pc = self.state.pc.wrapping_add(2);
                self.state.micro.reset();
                self.initiate_current_instruction();
                Some(StepResult::InstructionCompleted)
            }
            MicroRetireMode::ScratchPrefetch => {
                self.state.ir = self.state.prefetch[0];
                self.state.prefetch[0] = self.state.micro.scratch_prefetch;
                self.state.pc = self.state.pc.wrapping_add(2);
                self.state.micro.reset();
                self.initiate_current_instruction();
                Some(StepResult::InstructionCompleted)
            }
            MicroRetireMode::TargetRefill { target, new_ir } => {
                self.state.ir = new_ir;
                self.state.prefetch[0] = self.state.micro.last_read;
                self.state.pc = target.wrapping_add(4);
                self.state.micro.reset();
                self.initiate_current_instruction();
                Some(StepResult::InstructionCompleted)
            }
        }
    }

    /// Loads the active instruction's micro-step descriptor from the static table
    #[inline(always)]
    pub(crate) fn initiate_current_instruction(&mut self) {
        let desc = &crate::micro::dispatch_table::OPCODE_DESCRIPTOR_TABLE[self.state.ir as usize];
        if !desc.steps.is_empty() {
            self.state.micro.initiate_instruction(desc);
        }
    }

    /// Schedules a structured bus transaction onto the CPU micro-state machine
    #[inline]
    pub fn initiate_bus_cycle(&mut self, cycle: BusCycle) {
        self.state.micro.initiate_bus_cycle(cycle);
    }

    /// Executes the CCK1 sub-phase for an in-flight bus transaction
    #[inline]
    pub fn step_active_bus_cck1(&mut self, bus: &mut MemoryBus) -> StepResult {
        if self.state.micro.is_bus_busy() && self.state.micro.phase == CckPhase::Cck1 {
            let bus_res = self.state.micro.step_cck(bus, &mut self.wait_cycles);
            self.total_clocks = self.total_clocks.wrapping_add(2);
            self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
            if bus_res.is_wait() {
                return StepResult::WaitState;
            }
            return StepResult::StepCompleted;
        }
        StepResult::StepCompleted
    }

    /// Initiates a cycle-exact bus read cycle and steps CCK1
    #[inline(always)]
    pub fn initiate_read_cycle(
        &mut self,
        bus: &mut MemoryBus,
        addr: u32,
        size: BusAccessSize,
        fc: u8,
    ) -> StepResult {
        self.initiate_bus_cycle(BusCycle::new_read(addr, size, fc));
        self.step_active_bus_cck1(bus)
    }

    /// Initiates a cycle-exact bus write cycle and steps CCK1
    #[inline(always)]
    pub fn initiate_write_cycle(
        &mut self,
        bus: &mut MemoryBus,
        addr: u32,
        val: u16,
        size: BusAccessSize,
        fc: u8,
    ) -> StepResult {
        self.initiate_bus_cycle(BusCycle::new_write(addr, val, size, fc));
        self.step_active_bus_cck1(bus)
    }

    /// Triggers a cycle-exact Group 0 Address Error on unaligned word/long access
    #[inline(never)]
    pub fn trigger_address_error_step(
        &mut self,
        addr: u32,
        is_read: bool,
        is_program_space: bool,
        bus: &mut MemoryBus,
    ) -> StepResult {
        let fc = if is_program_space {
            types::prog_fc(&self.state)
        } else {
            types::data_fc(&self.state)
        };
        self.instruction_clocks = self.instruction_clocks.wrapping_add(8);
        self.total_clocks = self.total_clocks.wrapping_add(8);
        self.handle_address_error_fc(addr, is_read, fc, bus);
        StepResult::InstructionCompleted
    }

    /// Executes the active instruction's micro-step sequence directly
    pub fn execute_micro_step(&mut self, bus: &mut MemoryBus) -> StepResult {
        while (self.state.micro.micro_step as usize) < self.state.micro.current_steps.len() {
            let step = self.state.micro.current_steps[self.state.micro.micro_step as usize];
            if let Some(alu) = step.alu_fn {
                let reg_src = self.state.micro.reg_src;
                let reg_dst = self.state.micro.reg_dst;
                alu(&mut self.state, reg_src, reg_dst);
            }
            match step.action {
                MicroAction::Alu => {
                    let clocks = if self.state.micro.internal_clocks > 0 {
                        self.state.micro.internal_clocks
                    } else {
                        step.base_clocks as u16
                    };
                    if clocks > 0 {
                        self.state.micro.internal_clocks = clocks.saturating_sub(2);
                        self.total_clocks = self.total_clocks.wrapping_add(2);
                        self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                        if self.state.micro.internal_clocks == 0 {
                            self.state.micro.micro_step =
                                self.state.micro.micro_step.wrapping_add(1);
                        }
                        return StepResult::StepCompleted;
                    } else {
                        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                    }
                }
                MicroAction::BranchEval => continue,
                MicroAction::MovemTransfer => {
                    if let Some(res) =
                        crate::instructions::movem::execute_movem_transfer(self, bus)
                    {
                        return res;
                    }
                }
                MicroAction::BusReadByte
                | MicroAction::BusReadWord
                | MicroAction::BusReadLongHigh
                | MicroAction::BusReadLongLow => {
                    return self.execute_bus_read(bus, step.action);
                }
                MicroAction::BusWriteByte
                | MicroAction::BusWriteWord
                | MicroAction::BusWriteLongHigh
                | MicroAction::BusWriteLongLow
                | MicroAction::BusWriteWordAndRetire
                | MicroAction::BusWriteByteAndRetire
                | MicroAction::BusWriteLongLowAndRetire
                | MicroAction::BusWriteLongHighAndRetire => {
                    return self.execute_bus_write(bus, step.action);
                }
                MicroAction::BusPopStack
                | MicroAction::BusPopStackHigh
                | MicroAction::BusPopStackLow
                | MicroAction::BusPushStackHigh
                | MicroAction::BusPushStackLow
                | MicroAction::BusPushStackLowAndRetire => {
                    return self.execute_stack_op(bus, step.action);
                }
                MicroAction::FetchExtension
                | MicroAction::BusPrefetchToScratch
                | MicroAction::PrefetchNextOpcodeAndRetire
                | MicroAction::BusReadTargetOpcode
                | MicroAction::PrefetchTargetAndRetire => {
                    return self.execute_prefetch_and_refill(bus, step.action);
                }
                MicroAction::OriToCcr
                | MicroAction::OriToSr
                | MicroAction::AndiToCcr
                | MicroAction::AndiToSr
                | MicroAction::EoriToCcr
                | MicroAction::EoriToSr
                | MicroAction::Trap => {
                    return self.execute_system_op(bus, step.action);
                }
            }
        }
        StepResult::InstructionCompleted
    }

    #[inline]
    fn execute_bus_read(&mut self, bus: &mut MemoryBus, action: MicroAction) -> StepResult {
        let fc = types::data_fc(&self.state);
        match action {
            MicroAction::BusReadByte => {
                let addr = self.state.micro.ea_addr;
                self.initiate_read_cycle(bus, addr, BusAccessSize::Byte, fc)
            }
            MicroAction::BusReadWord => {
                let addr = self.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    return self.trigger_address_error_step(addr, true, false, bus);
                }
                self.initiate_read_cycle(bus, addr, BusAccessSize::Word, fc)
            }
            MicroAction::BusReadLongHigh => {
                let addr = self.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    return self.trigger_address_error_step(addr, true, false, bus);
                }
                self.initiate_read_cycle(bus, addr, BusAccessSize::Word, fc)
            }
            MicroAction::BusReadLongLow => {
                self.state.micro.scratch[0] = (self.state.micro.last_read as u32) << 16;
                let addr = self.state.micro.ea_addr.wrapping_add(2);
                self.initiate_read_cycle(bus, addr, BusAccessSize::Word, fc)
            }
            _ => StepResult::StepCompleted,
        }
    }

    #[inline]
    fn execute_bus_write(&mut self, bus: &mut MemoryBus, action: MicroAction) -> StepResult {
        let fc = types::data_fc(&self.state);
        match action {
            MicroAction::BusWriteByte => {
                let addr = self.state.micro.ea_addr;
                let val = (self.state.micro.write_buffer & 0xFF) as u16;
                self.initiate_write_cycle(bus, addr, val, BusAccessSize::Byte, fc)
            }
            MicroAction::BusWriteWord => {
                let addr = self.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    return self.trigger_address_error_step(addr, false, false, bus);
                }
                let val = (self.state.micro.write_buffer & 0xFFFF) as u16;
                self.initiate_write_cycle(bus, addr, val, BusAccessSize::Word, fc)
            }
            MicroAction::BusWriteLongHigh => {
                let addr = self.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    return self.trigger_address_error_step(addr, false, false, bus);
                }
                let val = ((self.state.micro.write_buffer >> 16) & 0xFFFF) as u16;
                self.initiate_write_cycle(bus, addr, val, BusAccessSize::Word, fc)
            }
            MicroAction::BusWriteLongLow => {
                let addr = self.state.micro.ea_addr.wrapping_add(2);
                if (addr & 1) != 0 {
                    return self.trigger_address_error_step(addr, false, false, bus);
                }
                let val = (self.state.micro.write_buffer & 0xFFFF) as u16;
                self.initiate_write_cycle(bus, addr, val, BusAccessSize::Word, fc)
            }
            MicroAction::BusWriteWordAndRetire => {
                let addr = self.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    self.state.ir = self.state.prefetch[0];
                    return self.trigger_address_error_step(addr, false, false, bus);
                }
                let val = (self.state.micro.write_buffer & 0xFFFF) as u16;
                self.state.micro.mark_scratch_prefetch_retire();
                self.initiate_write_cycle(bus, addr, val, BusAccessSize::Word, fc)
            }
            MicroAction::BusWriteByteAndRetire => {
                let addr = self.state.micro.ea_addr;
                let val = (self.state.micro.write_buffer & 0xFF) as u16;
                self.state.micro.mark_scratch_prefetch_retire();
                self.initiate_write_cycle(bus, addr, val, BusAccessSize::Byte, fc)
            }
            MicroAction::BusWriteLongLowAndRetire => {
                let addr = self.state.micro.ea_addr.wrapping_add(2);
                if (addr & 1) != 0 {
                    return self.trigger_address_error_step(addr, false, false, bus);
                }
                let val = (self.state.micro.write_buffer & 0xFFFF) as u16;
                self.state.micro.mark_scratch_prefetch_retire();
                self.initiate_write_cycle(bus, addr, val, BusAccessSize::Word, fc)
            }
            MicroAction::BusWriteLongHighAndRetire => {
                let addr = self.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    return self.trigger_address_error_step(addr, false, false, bus);
                }
                let val = ((self.state.micro.write_buffer >> 16) & 0xFFFF) as u16;
                self.state.micro.mark_scratch_prefetch_retire();
                self.initiate_write_cycle(bus, addr, val, BusAccessSize::Word, fc)
            }
            _ => StepResult::StepCompleted,
        }
    }

    #[inline]
    fn execute_stack_op(&mut self, bus: &mut MemoryBus, action: MicroAction) -> StepResult {
        let fc = types::data_fc(&self.state);
        match action {
            MicroAction::BusPopStack
            | MicroAction::BusPopStackHigh
            | MicroAction::BusPopStackLow => {
                let sp = self.state.read_a(7);
                if (sp & 1) != 0 {
                    return self.trigger_address_error_step(sp, true, false, bus);
                }
                self.state.write_a(7, sp.wrapping_add(2));
                self.initiate_read_cycle(bus, sp, BusAccessSize::Word, fc)
            }
            MicroAction::BusPushStackHigh => {
                let sp = self.state.read_a(7).wrapping_sub(4);
                self.state.write_a(7, sp);
                if (sp & 1) != 0 {
                    return self.trigger_address_error_step(sp, false, false, bus);
                }
                let val = ((self.state.micro.write_buffer >> 16) & 0xFFFF) as u16;
                self.initiate_write_cycle(bus, sp, val, BusAccessSize::Word, fc)
            }
            MicroAction::BusPushStackLow => {
                let sp_low = self.state.read_a(7).wrapping_add(2);
                let val = (self.state.micro.write_buffer & 0xFFFF) as u16;
                self.initiate_write_cycle(bus, sp_low, val, BusAccessSize::Word, fc)
            }
            MicroAction::BusPushStackLowAndRetire => {
                let sp_low = self.state.read_a(7).wrapping_add(2);
                let val = (self.state.micro.write_buffer & 0xFFFF) as u16;
                self.state.micro.mark_scratch_prefetch_retire();
                self.initiate_write_cycle(bus, sp_low, val, BusAccessSize::Word, fc)
            }
            _ => StepResult::StepCompleted,
        }
    }

    #[inline]
    fn execute_prefetch_and_refill(
        &mut self,
        bus: &mut MemoryBus,
        action: MicroAction,
    ) -> StepResult {
        let fc = types::prog_fc(&self.state);
        match action {
            MicroAction::FetchExtension => {
                let addr = self.state.pc;
                self.state.pc = self.state.pc.wrapping_add(2);
                self.initiate_read_cycle(bus, addr, BusAccessSize::Word, fc)
            }
            MicroAction::BusPrefetchToScratch => {
                let addr = self.state.pc;
                self.initiate_read_cycle(bus, addr, BusAccessSize::Word, fc)
            }
            MicroAction::PrefetchNextOpcodeAndRetire => {
                let addr = self.state.pc;
                self.state.micro.mark_standard_prefetch_retire();
                self.initiate_read_cycle(bus, addr, BusAccessSize::Word, fc)
            }
            MicroAction::BusReadTargetOpcode => {
                let addr = self.state.micro.ea_addr;
                if (addr & 1) != 0 {
                    return self.trigger_address_error_step(addr, true, true, bus);
                }
                self.initiate_read_cycle(bus, addr, BusAccessSize::Word, fc)
            }
            MicroAction::PrefetchTargetAndRetire => {
                let addr = self.state.micro.ea_addr.wrapping_add(2);
                let target = self.state.micro.ea_addr;
                let scratch_pref = self.state.micro.scratch_prefetch;
                self.state
                    .micro
                    .mark_target_refill_retire(target, scratch_pref);
                self.initiate_read_cycle(bus, addr, BusAccessSize::Word, fc)
            }
            _ => StepResult::StepCompleted,
        }
    }

    #[inline]
    fn execute_system_op(&mut self, bus: &mut MemoryBus, action: MicroAction) -> StepResult {
        match action {
            MicroAction::OriToCcr => system::op_ori_to_ccr(self, bus),
            MicroAction::OriToSr => system::op_ori_to_sr(self, bus),
            MicroAction::AndiToCcr => system::op_andi_to_ccr(self, bus),
            MicroAction::AndiToSr => system::op_andi_to_sr(self, bus),
            MicroAction::EoriToCcr => system::op_eori_to_ccr(self, bus),
            MicroAction::EoriToSr => system::op_eori_to_sr(self, bus),
            MicroAction::Trap => {
                let res = crate::instructions::trap::op_trap(self, bus);
                if self.state.micro.is_bus_busy() && self.state.micro.phase == CckPhase::Cck1 {
                    return self.step_active_bus_cck1(bus);
                }
                res
            }
            _ => StepResult::StepCompleted,
        }
    }

    /// Enables or disables transaction recording for cycle-exact test harnesses
    #[inline]
    pub fn enable_transaction_recording(&mut self, enabled: bool) {
        self.state.micro.enable_transaction_recording(enabled);
    }

    /// Test & verification harness helper: returns recorded bus transactions if recording is enabled
    #[inline]
    pub fn recorded_transactions(&self) -> Option<&[crate::micro::RecordedTransaction]> {
        self.state.micro.transaction_log.as_deref()
    }


    /// Records an internal CPU operation duration and schedules internal execution clocks
    #[inline]
    pub fn record_internal_clocks(&mut self, clocks: u16) {
        self.state.micro.record_internal_clocks(clocks);
    }

    /// Executes exactly one full M68000 instruction via direct table dispatch
    pub fn step_instruction(&mut self, bus: &mut MemoryBus) -> StepResult {
        if self.state.halted {
            return StepResult::Halted;
        }
        if self.state.stopped {
            return StepResult::Stopped;
        }

        self.instruction_clocks = 0;
        let mut loop_count = 0u32;
        const MAX_INSTRUCTION_CCK_STEPS: u32 = 10_000;
        loop {
            let res = self.step_cck(bus);
            if res.is_completed() || res == StepResult::Halted || res == StepResult::Stopped {
                return res;
            }
            loop_count = loop_count.wrapping_add(1);
            if loop_count >= MAX_INSTRUCTION_CCK_STEPS {
                self.state.micro.reset();
                return StepResult::InstructionCompleted;
            }
        }
    }

    /// Retire current instruction and refill prefetch pipeline for sequential instructions
    #[inline]
    pub fn retire_instruction(&mut self, bus: &mut MemoryBus) {
        self.state.ir = self.state.prefetch[0];
        self.state.prefetch[0] = bus.read_word_debug(self.state.pc & 0x00FF_FFFF);
        self.state.pc = self.state.pc.wrapping_add(2);
    }

    /// Consumes the next extension word from the prefetch pipeline
    #[inline]
    pub fn consume_extension_word(&mut self, bus: &mut MemoryBus) -> u16 {
        let ext = self.state.prefetch[0];
        self.state.prefetch[0] = bus.read_word_debug(self.state.pc & 0x00FF_FFFF);
        self.state.pc = self.state.pc.wrapping_add(2);
        ext
    }

    /// Reloads PC and prefetches the next two instruction words (after branch or jump)
    pub fn reload_pc_and_prefetch(&mut self, target_pc: u32, bus: &mut MemoryBus) {
        self.state.pc = target_pc;
        self.state.ir = bus.read_word_debug(self.state.pc & 0x00FF_FFFF);
        self.state.pc = self.state.pc.wrapping_add(2);
        self.state.prefetch[0] = bus.read_word_debug(self.state.pc & 0x00FF_FFFF);
        self.state.pc = self.state.pc.wrapping_add(2);
    }

    /// Handles address error exception with specific function code
    pub(crate) fn handle_address_error_fc(
        &mut self,
        fault_addr: u32,
        is_read: bool,
        function_code: u8,
        bus: &mut MemoryBus,
    ) {
        // Group 0 Address Error exception processing takes 50 clock periods
        self.instruction_clocks = self.instruction_clocks.wrapping_add(50);
        self.total_clocks = self.total_clocks.wrapping_add(50);
        system::push_address_error_exception(
            &mut self.state,
            fault_addr,
            is_read,
            function_code,
            bus,
        );
        self.reload_pc_and_prefetch(self.state.pc, bus);
        self.state.micro.reset();
    }
}
