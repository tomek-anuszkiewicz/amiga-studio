//! Cycle-exact Motorola 68000 CPU Execution Core

use crate::instructions::system;
use crate::micro::types;
use crate::state::CpuState;
use memory_bus::{BusAccessSize, BusResult, CckPhase, MemoryBus};

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

        // 1. If internal execution clocks remain (e.g. multi-cycle shift/div or TRAP):
        if self.state.micro.internal_clocks > 0 {
            self.state.micro.internal_clocks = self.state.micro.internal_clocks.saturating_sub(2);
            self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
            if self.state.micro.internal_clocks == 0 {
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                self.state.micro.clocks_remaining = -1;
            }
            return StepResult::StepCompleted;
        }

        // 2. Check if current instruction uses static micro-steps:
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

    /// Retires an instruction sequentially using standard prefetch refill
    #[inline(always)]
    pub(crate) fn retire_standard(&mut self, next_op: u16) -> StepResult {
        self.state.ir = self.state.prefetch[0];
        self.state.prefetch[0] = next_op;
        self.state.pc = self.state.pc.wrapping_add(2);
        self.state.micro.reset();
        self.initiate_current_instruction();
        StepResult::InstructionCompleted
    }

    /// Retires an instruction using scratch_prefetch (Class 0 RMW where prefetch precedes write)
    #[inline(always)]
    pub(crate) fn retire_scratch_prefetch(&mut self) -> StepResult {
        self.state.ir = self.state.prefetch[0];
        self.state.prefetch[0] = self.state.micro.scratch_prefetch;
        self.state.pc = self.state.pc.wrapping_add(2);
        self.state.micro.reset();
        self.initiate_current_instruction();
        StepResult::InstructionCompleted
    }

    /// Retires an instruction after branch/jump pipeline refill
    #[inline(always)]
    pub(crate) fn retire_target_refill(
        &mut self,
        target: u32,
        target_prefetch: u16,
        new_ir: u16,
    ) -> StepResult {
        self.state.ir = new_ir;
        self.state.prefetch[0] = target_prefetch;
        self.state.pc = target.wrapping_add(4);
        self.state.micro.reset();
        self.initiate_current_instruction();
        StepResult::InstructionCompleted
    }

    /// Loads the active instruction's micro-step descriptor from the static table
    #[inline(always)]
    pub(crate) fn initiate_current_instruction(&mut self) {
        let desc = &crate::micro::dispatch_table::OPCODE_DESCRIPTOR_TABLE[self.state.ir as usize];
        if !desc.steps.is_empty() {
            self.state.micro.initiate_instruction(desc);
        }
    }

    /// Fundamental 2-phase Color Clock (CCK) read word primitive with specific Function Code
    pub fn step_read_word_at_fc(
        &mut self,
        bus: &mut MemoryBus,
        addr: u32,
        function_code: u8,
    ) -> StepResult {
        let addr = addr & 0x00FF_FFFF;
        match self.state.micro.phase {
            CckPhase::Cck1 => match bus.read_word(addr) {
                BusResult::WaitState => {
                    self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                    self.state.micro.current_cycle_wait_cycles =
                        self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                    StepResult::WaitState
                }
                BusResult::Ready(data) => {
                    self.state.micro.last_read = data;
                    self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                    self.state.micro.phase = CckPhase::Cck2;
                    StepResult::StepCompleted
                }
            },
            CckPhase::Cck2 => {
                self.state.micro.record_bus_transaction(
                    true,
                    false,
                    function_code,
                    addr,
                    BusAccessSize::Word,
                    self.state.micro.last_read,
                    true,
                    true,
                );
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck1;
                self.state.micro.current_cycle_wait_cycles = 0;
                StepResult::StepCompleted
            }
        }
    }

    /// Read word in data space (FC1 or FC5)
    #[inline(always)]
    pub fn step_read_word_at(&mut self, bus: &mut MemoryBus, addr: u32) -> StepResult {
        let fc = types::data_fc(&self.state);
        self.step_read_word_at_fc(bus, addr, fc)
    }

    /// Read word in program space (FC2 or FC6)
    #[inline(always)]
    pub fn step_read_prog_word_at(&mut self, bus: &mut MemoryBus, addr: u32) -> StepResult {
        let fc = types::prog_fc(&self.state);
        self.step_read_word_at_fc(bus, addr, fc)
    }

    /// Fundamental 2-phase Color Clock (CCK) read byte primitive
    pub fn step_read_byte_at(&mut self, bus: &mut MemoryBus, addr: u32) -> StepResult {
        let addr = addr & 0x00FF_FFFF;
        match self.state.micro.phase {
            CckPhase::Cck1 => match bus.read_byte(addr) {
                BusResult::WaitState => {
                    self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                    self.state.micro.current_cycle_wait_cycles =
                        self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                    StepResult::WaitState
                }
                BusResult::Ready(data) => {
                    self.state.micro.last_read = data as u16;
                    self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                    self.state.micro.phase = CckPhase::Cck2;
                    StepResult::StepCompleted
                }
            },
            CckPhase::Cck2 => {
                let fc = types::data_fc(&self.state);
                let (uds, lds) = if (addr & 1) == 0 {
                    (true, false)
                } else {
                    (false, true)
                };
                self.state.micro.record_bus_transaction(
                    true,
                    false,
                    fc,
                    addr,
                    BusAccessSize::Byte,
                    self.state.micro.last_read,
                    uds,
                    lds,
                );
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck1;
                self.state.micro.current_cycle_wait_cycles = 0;
                StepResult::StepCompleted
            }
        }
    }

    /// Fundamental 2-phase Color Clock (CCK) write word primitive
    pub fn step_write_word_at(&mut self, bus: &mut MemoryBus, addr: u32, data: u16) -> StepResult {
        let addr = addr & 0x00FF_FFFF;
        match self.state.micro.phase {
            CckPhase::Cck1 => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                StepResult::StepCompleted
            }
            CckPhase::Cck2 => match bus.write_word(addr, data) {
                BusResult::WaitState => {
                    self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                    self.state.micro.current_cycle_wait_cycles =
                        self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                    StepResult::WaitState
                }
                BusResult::Ready(()) => {
                    let fc = types::data_fc(&self.state);
                    self.state.micro.record_bus_transaction(
                        false,
                        false,
                        fc,
                        addr,
                        BusAccessSize::Word,
                        data,
                        true,
                        true,
                    );
                    self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                    self.state.micro.phase = CckPhase::Cck1;
                    self.state.micro.current_cycle_wait_cycles = 0;
                    StepResult::StepCompleted
                }
            },
        }
    }

    /// Fundamental 2-phase Color Clock (CCK) write byte primitive
    pub fn step_write_byte_at(&mut self, bus: &mut MemoryBus, addr: u32, data: u8) -> StepResult {
        let addr = addr & 0x00FF_FFFF;
        match self.state.micro.phase {
            CckPhase::Cck1 => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                StepResult::StepCompleted
            }
            CckPhase::Cck2 => match bus.write_byte(addr, data) {
                BusResult::WaitState => {
                    self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                    self.state.micro.current_cycle_wait_cycles =
                        self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                    StepResult::WaitState
                }
                BusResult::Ready(()) => {
                    let fc = types::data_fc(&self.state);
                    let (uds, lds) = if (addr & 1) == 0 {
                        (true, false)
                    } else {
                        (false, true)
                    };
                    self.state.micro.record_bus_transaction(
                        false,
                        false,
                        fc,
                        addr,
                        BusAccessSize::Byte,
                        data as u16,
                        uds,
                        lds,
                    );
                    self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                    self.state.micro.phase = CckPhase::Cck1;
                    self.state.micro.current_cycle_wait_cycles = 0;
                    StepResult::StepCompleted
                }
            },
        }
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
        self.handle_address_error_fc(addr, is_read, fc, bus);
        StepResult::InstructionCompleted
    }

    /// Executes the active instruction's micro-step sequence directly
    pub fn execute_micro_step(&mut self, bus: &mut MemoryBus) -> StepResult {
        while (self.state.micro.micro_step as usize) < self.state.micro.current_steps.len() {
            let step = self.state.micro.current_steps[self.state.micro.micro_step as usize];

            // 1. If entering this micro-step for the first time, execute alu_fn and determine initial clocks
            if self.state.micro.clocks_remaining < 0 {
                let prev_steps_ptr = self.state.micro.current_steps.as_ptr();
                if let Some(alu) = step.alu_fn {
                    let reg_src = self.state.micro.reg_src;
                    let reg_dst = self.state.micro.reg_dst;
                    alu(&mut self.state, reg_src, reg_dst);
                }

                // If alu_fn redirected execution to a new step sequence (e.g. Bcc branch taken vs untaken),
                // restart immediately at the new sequence
                if self.state.micro.current_steps.as_ptr() != prev_steps_ptr {
                    self.state.micro.clocks_remaining = -1;
                    continue;
                }

                let clocks = if self.state.micro.internal_clocks > 0 {
                    let c = self.state.micro.internal_clocks;
                    self.state.micro.internal_clocks = 0;
                    c as i16
                } else {
                    step.base_clocks as i16
                };

                self.state.micro.clocks_remaining = clocks;
            }

            // 2. Execute step function
            let prev_micro_step = self.state.micro.micro_step;
            if let Some(res) = (step.step_fn)(self, bus) {
                if self.state.micro.micro_step != prev_micro_step {
                    self.state.micro.clocks_remaining = -1;
                }
                return res;
            } else {
                // Internal ALU or timing step (step_fn returned None, e.g. step_alu):
                // Driver loop controls cycle consumption and micro-step progression.
                if self.state.micro.clocks_remaining <= 0 {
                    // Instantaneous 0-clock step: advance immediately in the while loop
                    if self.state.micro.micro_step == prev_micro_step {
                        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                    }
                    self.state.micro.clocks_remaining = -1;
                    continue;
                } else {
                    // Multi-clock delay step: consume 2 clocks per CCK and advance when done
                    self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                    self.state.micro.clocks_remaining -= 2;
                    if self.state.micro.clocks_remaining <= 0 {
                        if self.state.micro.micro_step == prev_micro_step {
                            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                        }
                        self.state.micro.clocks_remaining = -1;
                    }
                    return StepResult::StepCompleted;
                }
            }
        }
        StepResult::InstructionCompleted
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
