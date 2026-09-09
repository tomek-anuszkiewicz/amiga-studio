//! Cycle-exact Motorola 68000 CPU Execution Core

use crate::micro::types;
use crate::state::CpuState;
use memory_bus::{AddressBus, BusResult};

/// Motorola 68000 CPU Core
#[derive(Debug, Clone)]
pub struct Cpu {
    pub state: CpuState,
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
        }
    }

    /// Reset CPU according to Cold/Warm reset specification
    pub fn reset(&mut self, bus: &mut dyn AddressBus) {
        self.state.sr = 0x2700;
        self.state.stopped = false;
        self.state.halted = false;
        self.state.micro.reset();
        self.state.cycle_counter = 0;

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

    /// Advances the global cycle counter
    #[inline(always)]
    pub fn advance_clocks(&mut self, clocks: u32) {
        self.state.cycle_counter = self.state.cycle_counter.wrapping_add(clocks as u64);
    }

    /// Returns total elapsed CPU clock cycles since reset
    #[inline(always)]
    pub fn cycle_counter(&self) -> u64 {
        self.state.cycle_counter
    }

    /// Resets the global cycle counter to zero
    #[inline(always)]
    pub fn reset_cycle_counter(&mut self) {
        self.state.cycle_counter = 0;
    }

    /// CCK phase stepping primitive (each invocation steps exactly 1 CCK = 2 CPU clocks)
    pub fn step_cck(&mut self, bus: &mut dyn AddressBus) -> bool {
        if self.state.halted || self.state.stopped {
            return false;
        }

        // Every active CCK phase step advances global and instruction clocks by 2 CPU clocks (1 CCK)
        self.advance_clocks(2);

        // 1. Check if current instruction uses static micro-steps:
        if self.state.micro.micro_step == 0 && self.state.micro.current_steps.is_empty() {
            self.state.instruction_pc = self.state.pc.wrapping_sub(4);
            self.initiate_current_instruction();
        }

        if self.state.micro.current_steps.is_empty() {
            self.state.halted = true;
            return false;
        }

        // 2. Execute via cycle-exact Micro-Step State Machine:
        while (self.state.micro.micro_step as usize) < self.state.micro.current_steps.len() {
            let step = self.state.micro.current_steps[self.state.micro.micro_step as usize];

            // 2a. Step initialization (executed once upon entering this micro_step):
            if self.state.micro.clocks_remaining == 0 {
                self.state.micro.clocks_remaining = step.base_clocks as u16;
                let prev_steps_ptr = self.state.micro.current_steps.as_ptr();

                if let Some(alu) = step.alu_fn {
                    let reg_src = self.state.micro.reg_src;
                    let reg_dst = self.state.micro.reg_dst;
                    alu(&mut self.state, reg_src, reg_dst);
                }

                // If ALU redirected execution to a new step sequence (e.g. Bcc branch taken),
                // restart immediately at step 0 of the new sequence
                if self.state.micro.current_steps.as_ptr() != prev_steps_ptr {
                    self.state.micro.clocks_remaining = 0;
                    continue;
                }

                // Instantaneous zero-clock step (pure ALU / EA calculation):
                // Advance micro_step and continue within the same CCK.
                if step.alu_fn.is_some() && self.state.micro.clocks_remaining == 0 {
                    self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                    continue;
                }
            }

            // 2b. Unified clock-consuming bus cycle execution (consumes 1 CCK = 2 CPU clocks):
            let prev_steps_ptr = self.state.micro.current_steps.as_ptr();
            let prev_micro_step = self.state.micro.micro_step;

            let bus_res = match step.step_fn {
                Some(step_fn) => step_fn(self, bus),
                None => BusResult::Ready(()),
            };
            if self.state.micro.current_steps.is_empty() {
                return true;
            }

            // If step redirected execution to a new step sequence (e.g. Address Error),
            // conclude this CCK and begin the new sequence on next CCK.
            if self.state.micro.current_steps.as_ptr() != prev_steps_ptr {
                self.state.micro.clocks_remaining = 0;
                return false;
            }

            match bus_res {
                BusResult::WaitState => return false,
                BusResult::Ready(()) => {
                    self.state.micro.clocks_remaining =
                        self.state.micro.clocks_remaining.saturating_sub(2);

                    if (step.base_clocks > 0 || step.alu_fn.is_some())
                        && self.state.micro.clocks_remaining == 0
                        && self.state.micro.micro_step == prev_micro_step
                    {
                        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                    }

                    if (self.state.micro.micro_step as usize)
                        >= self.state.micro.current_steps.len()
                    {
                        self.retire_current_instruction();
                        return true;
                    }

                    return false;
                }
            }
        }

        self.retire_current_instruction();
        true
    }

    /// Retires the active instruction sequentially or via branch target refill
    #[inline(always)]
    pub(crate) fn retire_current_instruction(&mut self) {
        if self.state.micro.target_refill {
            let target = self.state.micro.ea_addr;
            let new_ir = self.state.micro.irc;
            self.state.ir = new_ir;
            self.state.pc = target.wrapping_add(4);
        } else if !self.state.micro.prefetch_retired {
            self.state.ir = self.state.prefetch[0];
            self.state.prefetch[0] = self.state.micro.irc;
            self.state.pc = self.state.pc.wrapping_add(2);
        }
        self.state.micro.reset();
        self.initiate_current_instruction();
    }

    /// Loads the active instruction's micro-step descriptor from the static table
    #[inline(always)]
    pub(crate) fn initiate_current_instruction(&mut self) {
        let desc = &crate::micro::dispatch_table::OPCODE_DESCRIPTOR_TABLE[self.state.ir as usize];
        if !desc.steps.is_empty() {
            self.state.micro.initiate_instruction(desc);
        }
    }

    /// Triggers a cycle-exact Group 0 Address Error on unaligned word/long access (50 CPU clocks / 25 CCKs)
    #[inline(never)]
    pub fn trigger_address_error(
        &mut self,
        fault_addr: u32,
        is_read: bool,
        is_program_space: bool,
    ) {
        let fc = if is_program_space {
            types::prog_fc(&self.state)
        } else {
            types::data_fc(&self.state)
        };
        let rw_bit = if is_read { 0x10 } else { 0x00 };
        let info_word = (self.state.ir & 0xFFE0) | rw_bit | ((fc as u16) & 0x07);

        self.state.micro.fault_addr = fault_addr;
        self.state.micro.info_word = info_word;
        self.state.micro.current_steps = &crate::micro::common::STEPS_ADDRESS_ERROR;
        self.state.micro.micro_step = 0;
        self.state.micro.clocks_remaining = 0;
    }

    /// Executes exactly one full M68000 instruction via direct table dispatch
    pub fn step_instruction(&mut self, bus: &mut dyn AddressBus) -> u32 {
        if self.state.halted || self.state.stopped {
            return 0;
        }

        let start_cycles = self.state.cycle_counter;
        let mut loop_count = 0u32;
        const MAX_INSTRUCTION_CCK_STEPS: u32 = 10_000;
        loop {
            let completed = self.step_cck(bus);
            if completed || self.state.halted || self.state.stopped {
                return self.state.cycle_counter.wrapping_sub(start_cycles) as u32;
            }
            loop_count = loop_count.wrapping_add(1);
            if loop_count >= MAX_INSTRUCTION_CCK_STEPS {
                self.state.micro.reset();
                return self.state.cycle_counter.wrapping_sub(start_cycles) as u32;
            }
        }
    }

    /// Reloads PC and prefetches the next two instruction words (after branch or jump)
    pub fn reload_pc_and_prefetch(&mut self, target_pc: u32, bus: &mut dyn AddressBus) {
        self.state.pc = target_pc;
        self.state.ir = bus.read_word_debug(self.state.pc & 0x00FF_FFFF);
        self.state.pc = self.state.pc.wrapping_add(2);
        self.state.prefetch[0] = bus.read_word_debug(self.state.pc & 0x00FF_FFFF);
        self.state.pc = self.state.pc.wrapping_add(2);
    }
}
