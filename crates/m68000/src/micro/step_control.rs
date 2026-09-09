//! 2-Clock Micro-Step Execution Handlers for Control Flow, Stack, and Branches
//!
//! Implements cycle-exact 2-clock micro-steps (1 CCK = 2 CPU clocks) for:
//! - Target instruction pipeline refill (BRA, Bcc, BSR, JMP, JSR, RTS)
//! - Stack push operations (PEA, BSR, JSR)
//! - Stack pop operations (RTS)

use crate::core::{Cpu, StepResult};
use crate::micro::types::{data_fc, prog_fc};
use memory_bus::{BusAccessSize, BusResult, CckPhase, MemoryBus};

impl Cpu {
    // ========================================================================
    // Target Opcode Refill Handlers (2 Clocks / 1 CCK per step)
    // ========================================================================

    /// CCK1: Reads first word of target instruction from `ea_addr`
    pub fn step_bus_read_target_opcode_read(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, true, true, bus));
        }
        match bus.read_word(addr & 0x00FF_FFFF) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(data) => {
                self.state.micro.last_read = data;
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK2: Latches target opcode into `scratch_prefetch` and logs transaction
    pub fn step_bus_read_target_opcode_finish(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr & 0x00FF_FFFF;
        let data = self.state.micro.last_read;
        self.state.micro.scratch_prefetch = data;
        let fc = prog_fc(&self.state);
        self.state.micro.record_bus_transaction(
            true,
            true,
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
        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        Some(StepResult::StepCompleted)
    }

    /// CCK1: Reads second word of target pipeline from `ea_addr + 2`
    pub fn step_prefetch_target_read(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, true, true, bus));
        }
        match bus.read_word(addr & 0x00FF_FFFF) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(data) => {
                self.state.micro.last_read = data;
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK2: Logs second target word transaction and completes pipeline refill retirement
    pub fn step_prefetch_target_and_retire_2clk(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr.wrapping_add(2) & 0x00FF_FFFF;
        let target_prefetch = self.state.micro.last_read;
        let fc = prog_fc(&self.state);
        self.state.micro.record_bus_transaction(
            true,
            true,
            fc,
            addr,
            BusAccessSize::Word,
            target_prefetch,
            true,
            true,
        );
        self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
        self.state.micro.phase = CckPhase::Cck1;
        self.state.micro.current_cycle_wait_cycles = 0;
        let target = self.state.micro.ea_addr;
        let new_ir = self.state.micro.scratch_prefetch;
        Some(self.retire_target_refill(target, target_prefetch, new_ir))
    }

    // ========================================================================
    // Stack Push Handlers (PEA, JSR, BSR)
    // ========================================================================

    /// CCK1: Decrements SP -= 4, validates alignment, and idles bus for high word write
    pub fn step_bus_push_stack_high_idle(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let sp = self.state.read_a(7).wrapping_sub(4);
        self.state.write_a(7, sp);
        if (sp & 1) != 0 {
            return Some(self.trigger_address_error_step(sp, false, false, bus));
        }
        self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
        self.state.micro.phase = CckPhase::Cck2;
        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        Some(StepResult::StepCompleted)
    }

    /// CCK2: Writes high word of destination/write_buffer to SP
    pub fn step_bus_push_stack_high_write(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let sp = self.state.read_a(7) & 0x00FF_FFFF;
        let src = if self.state.micro.destination != 0 {
            self.state.micro.destination
        } else {
            self.state.micro.write_buffer
        };
        let val = ((src >> 16) & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(()) => {
                let fc = data_fc(&self.state);
                self.state.micro.record_bus_transaction(
                    false,
                    false,
                    fc,
                    sp,
                    BusAccessSize::Word,
                    val,
                    true,
                    true,
                );
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck1;
                self.state.micro.current_cycle_wait_cycles = 0;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK2: Writes low word of destination/write_buffer to SP + 2 (non-retiring, for JSR/BSR)
    pub fn step_bus_push_stack_low_write(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let sp_low = self.state.read_a(7).wrapping_add(2) & 0x00FF_FFFF;
        let src = if self.state.micro.destination != 0 {
            self.state.micro.destination
        } else {
            self.state.micro.write_buffer
        };
        let val = (src & 0xFFFF) as u16;
        match bus.write_word(sp_low, val) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(()) => {
                let fc = data_fc(&self.state);
                self.state.micro.record_bus_transaction(
                    false,
                    false,
                    fc,
                    sp_low,
                    BusAccessSize::Word,
                    val,
                    true,
                    true,
                );
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck1;
                self.state.micro.current_cycle_wait_cycles = 0;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK2: Writes low word to SP + 2 and retires with scratch prefetch (for PEA)
    pub fn step_bus_push_stack_low_write_and_retire(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let sp_low = self.state.read_a(7).wrapping_add(2) & 0x00FF_FFFF;
        let src = if self.state.micro.destination != 0 {
            self.state.micro.destination
        } else {
            self.state.micro.write_buffer
        };
        let val = (src & 0xFFFF) as u16;
        match bus.write_word(sp_low, val) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(()) => {
                let fc = data_fc(&self.state);
                self.state.micro.record_bus_transaction(
                    false,
                    false,
                    fc,
                    sp_low,
                    BusAccessSize::Word,
                    val,
                    true,
                    true,
                );
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck1;
                self.state.micro.current_cycle_wait_cycles = 0;
                Some(self.retire_scratch_prefetch())
            }
        }
    }

    // ========================================================================
    // Stack Pop Handlers (RTS)
    // ========================================================================

    /// CCK1: Reads high word of return PC from (SP)
    pub fn step_bus_pop_stack_high_read(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let sp = self.state.read_a(7);
        if (sp & 1) != 0 {
            return Some(self.trigger_address_error_step(sp, true, false, bus));
        }
        match bus.read_word(sp & 0x00FF_FFFF) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(data) => {
                self.state.micro.last_read = data;
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK2: Latches high word of return PC, advances SP += 2, and logs transaction
    pub fn step_bus_pop_stack_high_finish(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        let sp = self.state.read_a(7);
        let data = self.state.micro.last_read;
        self.state.write_a(7, sp.wrapping_add(2));
        self.state.micro.scratch[0] = (data as u32) << 16;
        let fc = data_fc(&self.state);
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            sp & 0x00FF_FFFF,
            BusAccessSize::Word,
            data,
            true,
            true,
        );
        self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
        self.state.micro.phase = CckPhase::Cck1;
        self.state.micro.current_cycle_wait_cycles = 0;
        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        Some(StepResult::StepCompleted)
    }

    /// CCK1: Reads low word of return PC from (SP)
    pub fn step_bus_pop_stack_low_read(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let sp = self.state.read_a(7);
        if (sp & 1) != 0 {
            return Some(self.trigger_address_error_step(sp, true, false, bus));
        }
        match bus.read_word(sp & 0x00FF_FFFF) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(data) => {
                self.state.micro.last_read = data;
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK2: Latches full return PC into `ea_addr`, advances SP += 2, and logs transaction
    pub fn step_bus_pop_stack_low_finish(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        let sp = self.state.read_a(7);
        let data = self.state.micro.last_read;
        self.state.write_a(7, sp.wrapping_add(2));
        self.state.micro.ea_addr = self.state.micro.scratch[0] | (data as u32);
        let fc = data_fc(&self.state);
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            sp & 0x00FF_FFFF,
            BusAccessSize::Word,
            data,
            true,
            true,
        );
        self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
        self.state.micro.phase = CckPhase::Cck1;
        self.state.micro.current_cycle_wait_cycles = 0;
        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        Some(StepResult::StepCompleted)
    }
}
