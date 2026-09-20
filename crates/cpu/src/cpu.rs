//! Motorola 68000 CPU Emulator Core for the Commodore Amiga 500
//!
//! Provides cycle-exact micro-step state machine execution, effective address resolution,
//! condition code evaluation, and a 65,536-entry compile-time static opcode descriptor table.

pub(crate) mod instructions;
pub mod micro {
    pub mod common;
    pub mod dispatch_table;
    pub mod ea;
    pub mod engine;
    pub mod step_control;
    pub mod step_execution;
    pub mod types;

    pub use common::*;
    pub use dispatch_table::*;
    pub use ea::*;
    pub use engine::*;
    pub use types::*;
}
pub mod state;

pub use micro::{
    common::VECTOR_ADDRESS_ERROR, CpuMicroState, MicroStep, OpcodeDescriptor, Size,
    OPCODE_DESCRIPTOR_TABLE,
};
pub use state::{
    function_code, vector, CpuState, CCR_ALL, CCR_C, CCR_N, CCR_V, CCR_X, CCR_Z, SR_I_MASK,
    SR_MASK, SR_RESET_DEFAULT, SR_S, SR_T,
};

use crate::micro::types;
use physical_memory::{AddressBus, BusResult};
use serde::{Deserialize, Serialize};

/// Motorola 68000 CPU Core
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Reset: Zeroes data/address registers and USP, then initialises supervisor vectors
    pub fn reset(&mut self, bus: &mut dyn AddressBus) {
        self.state.clear_registers();
        self.reset_internal(bus);
    }

    /// Warm Reset: Preserves data/address registers and USP intact, then reloads supervisor vectors
    pub fn reset_warm(&mut self, bus: &mut dyn AddressBus) {
        self.reset_internal(bus);
    }

    fn reset_internal(&mut self, bus: &mut dyn AddressBus) {
        self.state.sr = crate::state::SR_RESET_DEFAULT;
        self.state.stopped = false;
        self.state.halted = false;
        self.state.reset_line_asserted = false;
        self.state.micro.reset();
        self.state.cycle_counter = 0;

        // Fetch initial SSP from Vector 0 ($000000)
        let ssp_addr = crate::state::vector::addr(crate::state::vector::RESET_SSP);
        let ssp_hi = bus.read_word_debug(ssp_addr);
        let ssp_lo = bus.read_word_debug(ssp_addr + 2);
        self.state.ssp = ((ssp_hi as u32) << 16) | (ssp_lo as u32);
        self.state.set_a_long(7, self.state.ssp);

        // Fetch initial PC from Vector 1 ($000004)
        let pc_addr = crate::state::vector::addr(crate::state::vector::RESET_PC);
        let pc_hi = bus.read_word_debug(pc_addr);
        let pc_lo = bus.read_word_debug(pc_addr + 2);
        self.state.pc = ((pc_hi as u32) << 16) | (pc_lo as u32);
        self.state.instruction_pc = self.state.pc;

        // Hardware M68000 Reset Vector Invariants:
        // On real 68000 silicon, Vector 0 ($000000) is loaded into SSP and Vector 1 ($000004)
        // is loaded into PC without any sanitization or default substitution.
        // If the initial PC address has bit 0 set (odd address), instruction prefetch cannot
        // proceed across the 16-bit data bus. An address error during reset exception processing
        // triggers an immediate Double Bus Fault, halting the CPU.
        if (self.state.pc & 1) != 0 {
            self.state.halted = true;
            return;
        }

        // Prime prefetch pipeline
        self.state.ir = bus.read_word_debug(self.state.pc);
        self.state.pc = self.state.pc.wrapping_add(2);
        self.state.prefetch = bus.read_word_debug(self.state.pc);
        self.state.pc = self.state.pc.wrapping_add(2);

        // Initiate the initial instruction descriptor immediately so CPU micro-state is fully ready
        self.initiate_current_instruction();
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

    /// Ensures active instruction micro-steps are initialized before execution begins.
    /// Returns `true` if valid steps are ready, or `false` if unmapped/empty (halting the CPU).
    #[inline(always)]
    fn ensure_instruction_ready(&mut self) -> bool {
        let needs_initialization =
            self.state.micro.micro_step == 0 && self.state.micro.current_steps.is_empty();
        if needs_initialization {
            self.initiate_current_instruction();
        }

        if self.state.micro.current_steps.is_empty() {
            self.state.halted = true;
            return false;
        }

        true
    }

    /// CCK phase stepping primitive (each invocation steps exactly 1 CCK = 2 CPU clocks).
    /// Performs instruction boundary validation and initiates the active instruction if uninitialized.
    #[inline]
    pub fn step_cck(&mut self, bus: &mut dyn AddressBus) -> bool {
        if self.state.halted {
            return false;
        }

        if self.state.stopped {
            if !self.check_and_trigger_interrupt() {
                return false;
            }
        }

        if !self.ensure_instruction_ready() {
            return false;
        }

        self.step_cck_internal(bus)
    }

    /// Internal cycle-exact Color Clock stepping primitive (1 CCK = 2 CPU clocks).
    /// Executes the active micro-step without repeating instruction-boundary setup.
    #[inline(always)]
    fn step_cck_internal(&mut self, bus: &mut dyn AddressBus) -> bool {
        // Every active CCK phase step advances global and instruction clocks by 2 CPU clocks (1 CCK)
        self.advance_clocks(2);

        // Execute via cycle-exact Micro-Step State Machine:
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
                let sequence_redirected = self.state.micro.current_steps.as_ptr() != prev_steps_ptr;
                if sequence_redirected {
                    self.state.micro.clocks_remaining = 0;
                    continue;
                }

                // Instantaneous zero-clock step (pure ALU / EA calculation):
                // Advance micro_step and continue within the same CCK.
                let is_instantaneous_step =
                    step.is_instantaneous() && self.state.micro.clocks_remaining == 0;
                if is_instantaneous_step {
                    self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                    continue;
                }
            }

            // 2b. Unified clock-consuming bus cycle execution (consumes 1 CCK = 2 CPU clocks):
            let prev_steps_ptr = self.state.micro.current_steps.as_ptr();
            let prev_micro_step = self.state.micro.micro_step;

            let bus_res = match step.bus_fn {
                Some(bus_fn) => bus_fn(self, bus),
                None => BusResult::Ready(()),
            };
            if self.state.micro.current_steps.is_empty() {
                return true;
            }

            // If step redirected execution to a new step sequence (e.g. Address Error),
            // conclude this CCK and begin the new sequence on next CCK.
            let sequence_redirected = self.state.micro.current_steps.as_ptr() != prev_steps_ptr;
            if sequence_redirected {
                self.state.micro.clocks_remaining = 0;
                return false;
            }

            match bus_res {
                BusResult::WaitState => return false,
                BusResult::Ready(()) => {
                    self.state.micro.clocks_remaining =
                        self.state.micro.clocks_remaining.saturating_sub(2);

                    let step_has_work = step.has_work();
                    let step_clocks_exhausted = self.state.micro.clocks_remaining == 0;
                    let sequence_did_not_branch = self.state.micro.micro_step == prev_micro_step;

                    // Advance to next micro-step once this step's clocks are exhausted
                    // and execution did not divert internally:
                    if step_has_work && step_clocks_exhausted && sequence_did_not_branch {
                        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                    }

                    let instruction_steps_completed = (self.state.micro.micro_step as usize)
                        >= self.state.micro.current_steps.len();

                    if instruction_steps_completed {
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
            self.state.ir = self.state.prefetch;
            self.state.prefetch = self.state.micro.irc;
            self.state.pc = self.state.pc.wrapping_add(2);
        }
        self.state.instruction_pc = self.state.pc.wrapping_sub(4);
        self.state.micro.reset();
        if self.check_and_trigger_interrupt() {
            return;
        }
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

    /// Checks if an interrupt is pending according to M68000 priority rules,
    /// and if so, initiates the autovector exception processing sequence.
    #[inline(always)]
    pub fn check_and_trigger_interrupt(&mut self) -> bool {
        if let Some(level) = self.state.is_interrupt_pending() {
            self.trigger_interrupt(level);
            true
        } else {
            false
        }
    }

    /// Triggers an Autovector Interrupt exception sequence (44 CPU clocks / 22 CCKs)
    #[inline(never)]
    pub fn trigger_interrupt(&mut self, level: u8) {
        self.state.ipl = level;
        self.state.micro.reset();
        self.state.micro.current_steps = &crate::micro::common::STEPS_INTERRUPT;
        self.state.micro.micro_step = 0;
        self.state.micro.clocks_remaining = 0;
    }

    /// Restores the CPU architectural and micro-state from a snapshot,
    /// re-hydrating ephemeral static execution pointers from the dispatch table.
    #[inline]
    pub fn restore_state(&mut self, state: CpuState) {
        self.state = state;
        let desc = &crate::micro::dispatch_table::OPCODE_DESCRIPTOR_TABLE[self.state.ir as usize];
        if !desc.steps.is_empty() {
            self.state.micro.current_steps = desc.steps;
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
        if self.state.halted {
            return 0;
        }

        if self.state.stopped {
            if !self.check_and_trigger_interrupt() {
                return 0;
            }
        }

        if !self.ensure_instruction_ready() {
            return 0;
        }

        let start_cycles = self.state.cycle_counter;
        loop {
            let completed = self.step_cck_internal(bus);
            let execution_finished = completed || self.state.halted || self.state.stopped;
            if execution_finished {
                return self.state.cycle_counter.wrapping_sub(start_cycles) as u32;
            }
        }
    }

    /// Test & Debugger helper: Sets the Program Counter to `target_pc` and primes
    /// the 2-word prefetch queue (`ir` and `prefetch`) via non-intrusive debug reads.
    ///
    /// This bypasses the 68000 vector table cold reset sequence to allow immediate
    /// execution in unit tests or synthetic debugger harnesses.
    #[inline]
    pub fn set_pc_and_prime_prefetch(&mut self, target_pc: u32, bus: &mut dyn AddressBus) {
        self.state.instruction_pc = target_pc;
        self.state.pc = target_pc;
        self.state.ir = bus.read_word_debug(self.state.pc & 0x00FF_FFFF);
        self.state.pc = self.state.pc.wrapping_add(2);
        self.state.prefetch = bus.read_word_debug(self.state.pc & 0x00FF_FFFF);
        self.state.pc = self.state.pc.wrapping_add(2);
        self.state.micro.reset();
        self.initiate_current_instruction();
    }
}
