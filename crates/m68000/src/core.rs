//! Cycle-exact Motorola 68000 CPU Execution Core

use crate::instructions::*;
use crate::state::CpuState;
use memory_bus::MemoryBus;

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

            // If the bus cycle just completed, check retirement mode
            if !self.state.micro.is_bus_busy() {
                match self.state.micro.retire_mode {
                    crate::micro::MicroRetireMode::None => {
                        // Capture intermediate read data for multi-step sequences
                        if !self.state.micro.current_steps.is_empty() {
                            let prev_idx = self.state.micro.micro_step.saturating_sub(1) as usize;
                            if prev_idx < self.state.micro.current_steps.len() {
                                match self.state.micro.current_steps[prev_idx].action {
                                    crate::micro::MicroAction::BusPrefetchToScratch => {
                                        self.state.micro.scratch_prefetch = self.state.micro.last_read;
                                    }
                                    crate::micro::MicroAction::FetchExtension => {
                                        self.state.prefetch[0] = self.state.micro.last_read;
                                    }
                                    crate::micro::MicroAction::BusReadLongHigh => {
                                        self.state.micro.scratch[0] = (self.state.micro.last_read as u32) << 16;
                                    }
                                    crate::micro::MicroAction::BusReadLongLow => {
                                        self.state.micro.scratch[1] = self.state.micro.scratch[0] | (self.state.micro.last_read as u32);
                                    }
                                    crate::micro::MicroAction::BusReadTargetOpcode => {
                                        self.state.micro.scratch_prefetch = self.state.micro.last_read;
                                    }
                                    crate::micro::MicroAction::BusPopStackHigh => {
                                        self.state.micro.scratch[0] = (self.state.micro.last_read as u32) << 16;
                                    }
                                    crate::micro::MicroAction::BusPopStackLow => {
                                        self.state.micro.ea_addr = self.state.micro.scratch[0] | (self.state.micro.last_read as u32);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    crate::micro::MicroRetireMode::StandardPrefetch => {
                        self.state.ir = self.state.prefetch[0];
                        self.state.prefetch[0] = self.state.micro.last_read;
                        self.state.pc = self.state.pc.wrapping_add(2);
                        self.state.micro.reset();
                        self.initiate_current_instruction();
                        return StepResult::InstructionCompleted;
                    }
                    crate::micro::MicroRetireMode::ScratchPrefetch => {
                        self.state.ir = self.state.prefetch[0];
                        self.state.prefetch[0] = self.state.micro.scratch_prefetch;
                        self.state.pc = self.state.pc.wrapping_add(2);
                        self.state.micro.reset();
                        self.initiate_current_instruction();
                        return StepResult::InstructionCompleted;
                    }
                    crate::micro::MicroRetireMode::TargetRefill { target, new_ir } => {
                        self.state.ir = new_ir;
                        self.state.prefetch[0] = self.state.micro.last_read;
                        self.state.pc = target.wrapping_add(4);
                        self.state.micro.reset();
                        self.initiate_current_instruction();
                        return StepResult::InstructionCompleted;
                    }
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
            return crate::micro::engine::execute_micro_step(self, bus);
        }

        // Unimplemented / Illegal opcode:
        self.state.halted = true;
        StepResult::Halted
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
    pub fn initiate_bus_cycle(&mut self, cycle: memory_bus::BusCycle) {
        self.state.micro.initiate_bus_cycle(cycle);
    }

    /// Schedules an instruction prefetch read cycle from PC
    #[inline]
    pub fn initiate_prefetch(&mut self) {
        let fc = if self.state.is_supervisor() {
            memory_bus::function_code::SUPERVISOR_PROGRAM
        } else {
            memory_bus::function_code::USER_PROGRAM
        };
        let cycle =
            memory_bus::BusCycle::new_read(self.state.pc, memory_bus::BusAccessSize::Word, fc);
        self.state.micro.initiate_bus_cycle(cycle);
    }

    /// Enables or disables transaction recording for cycle-exact test harnesses
    #[inline]
    pub fn enable_transaction_recording(&mut self, enabled: bool) {
        self.state.micro.enable_transaction_recording(enabled);
    }

    /// Returns recorded bus transactions if recording is enabled
    #[inline]
    pub fn recorded_transactions(&self) -> Option<&[crate::micro::RecordedTransaction]> {
        self.state.micro.transaction_log.as_deref()
    }

    /// Clears the recorded transactions buffer
    #[inline]
    pub fn clear_transactions(&mut self) {
        if let Some(ref mut log) = self.state.micro.transaction_log {
            log.clear();
        }
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
