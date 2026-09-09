//! Specialized Micro-Step Execution Handlers for M68000 CPU

use crate::core::{Cpu, StepResult};
use crate::micro::types::MicroStep;
use memory_bus::{BusAccessSize, BusResult, CckPhase, MemoryBus};

impl Cpu {
    pub(crate) fn execute_alu_step(&mut self, step: MicroStep) -> Option<StepResult> {
        let clocks = if self.state.micro.internal_clocks > 0 {
            self.state.micro.internal_clocks
        } else {
            step.base_clocks as u16
        };
        if clocks > 0 {
            self.state.micro.internal_clocks = clocks.saturating_sub(2);
            self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
            if self.state.micro.internal_clocks == 0 {
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
            }
            Some(StepResult::StepCompleted)
        } else {
            self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
            None
        }
    }

    pub fn step_alu(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        let step = self.state.micro.current_steps[self.state.micro.micro_step as usize];
        self.execute_alu_step(step)
    }

    // ========================================================================
    // 2-Clock Micro-Step Handlers (1 CCK = 2 CPU Clocks)
    // ========================================================================

    /// CCK1: Reads source byte from memory into `self.state.micro.source`
    pub fn step_bus_read_src_byte(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr & 0x00FF_FFFF;
        match bus.read_byte(addr) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(data) => {
                self.state.micro.last_read = data as u16;
                self.state.micro.source = data as u32;
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK1: Reads source word from memory into `self.state.micro.source`
    pub fn step_bus_read_src_word(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, true, false, bus));
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(data) => {
                self.state.micro.last_read = data;
                self.state.micro.source = data as u32;
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK1: Reads destination byte from memory into `self.state.micro.destination`
    pub fn step_bus_read_dst_byte(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr & 0x00FF_FFFF;
        match bus.read_byte(addr) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(data) => {
                self.state.micro.last_read = data as u16;
                self.state.micro.destination = data as u32;
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK1: Reads destination word from memory into `self.state.micro.destination`
    pub fn step_bus_read_dst_word(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, true, false, bus));
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(data) => {
                self.state.micro.last_read = data;
                self.state.micro.destination = data as u32;
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK1: Reads high word of 32-bit source operand into `self.state.micro.source`
    pub fn step_bus_read_src_long_high(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, true, false, bus));
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(data) => {
                self.state.micro.last_read = data;
                self.state.micro.source = (data as u32) << 16;
                self.state.micro.scratch[0] = (data as u32) << 16;
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK1: Reads low word of 32-bit source operand into `self.state.micro.source`
    pub fn step_bus_read_src_long_low(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, true, false, bus));
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(data) => {
                self.state.micro.last_read = data;
                self.state.micro.source = (self.state.micro.source & 0xFFFF_0000) | (data as u32);
                self.state.micro.scratch[1] = self.state.micro.source;
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK1: Reads high word of 32-bit destination operand into `self.state.micro.destination`
    pub fn step_bus_read_dst_long_high(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, true, false, bus));
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(data) => {
                self.state.micro.last_read = data;
                self.state.micro.destination = (data as u32) << 16;
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK1: Reads low word of 32-bit destination operand into `self.state.micro.destination`
    pub fn step_bus_read_dst_long_low(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, true, false, bus));
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(data) => {
                self.state.micro.last_read = data;
                self.state.micro.destination =
                    (self.state.micro.destination & 0xFFFF_0000) | (data as u32);
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck2;
                self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
                Some(StepResult::StepCompleted)
            }
        }
    }

    /// CCK2: Finishes bus read word cycle, records transaction, and releases bus for Agnus DMA
    pub fn step_bus_read_word_finish(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        let fc = crate::micro::types::data_fc(&self.state);
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            self.state.micro.ea_addr,
            BusAccessSize::Word,
            self.state.micro.last_read,
            true,
            true,
        );
        self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
        self.state.micro.phase = CckPhase::Cck1;
        self.state.micro.current_cycle_wait_cycles = 0;
        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        Some(StepResult::StepCompleted)
    }

    /// CCK2: Finishes bus read byte cycle, records transaction, and releases bus for Agnus DMA
    pub fn step_bus_read_byte_finish(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        let fc = crate::micro::types::data_fc(&self.state);
        let addr = self.state.micro.ea_addr;
        let (uds, lds) = if (addr & 1) == 0 { (true, false) } else { (false, true) };
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
        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        Some(StepResult::StepCompleted)
    }

    /// CCK1: Bus write setup (idle on bus, preparing address/pins, bus free for Agnus DMA)
    pub fn step_bus_write_idle(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
        self.state.micro.phase = CckPhase::Cck2;
        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        Some(StepResult::StepCompleted)
    }

    /// CCK2: Writes byte from `self.state.micro.destination` to memory and retires
    pub fn step_bus_write_dst_byte_and_retire(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        let val = (self.state.micro.destination & 0xFF) as u8;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_byte(addr_masked, val) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(()) => {
                let fc = crate::micro::types::data_fc(&self.state);
                let (uds, lds) = if (addr & 1) == 0 { (true, false) } else { (false, true) };
                self.state.micro.record_bus_transaction(
                    false,
                    false,
                    fc,
                    addr_masked,
                    BusAccessSize::Byte,
                    val as u16,
                    uds,
                    lds,
                );
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.phase = CckPhase::Cck1;
                self.state.micro.current_cycle_wait_cycles = 0;
                Some(self.retire_scratch_prefetch())
            }
        }
    }

    /// CCK2: Writes word from `self.state.micro.destination` to memory and retires
    pub fn step_bus_write_dst_word_and_retire(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.state.ir = self.state.prefetch[0];
            return Some(self.trigger_address_error_step(addr, false, false, bus));
        }
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_word(addr_masked, val) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(()) => {
                let fc = crate::micro::types::data_fc(&self.state);
                self.state.micro.record_bus_transaction(
                    false,
                    false,
                    fc,
                    addr_masked,
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

    /// CCK2: Writes word from `self.state.micro.destination` to memory (non-retiring)
    pub fn step_bus_write_dst_word(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, false, false, bus));
        }
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_word(addr_masked, val) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(()) => {
                let fc = crate::micro::types::data_fc(&self.state);
                self.state.micro.record_bus_transaction(
                    false,
                    false,
                    fc,
                    addr_masked,
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


    /// CCK2: Writes high word of 32-bit destination to memory
    pub fn step_bus_write_dst_long_high(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, false, false, bus));
        }
        let val = ((self.state.micro.destination >> 16) & 0xFFFF) as u16;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_word(addr_masked, val) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(()) => {
                let fc = crate::micro::types::data_fc(&self.state);
                self.state.micro.record_bus_transaction(
                    false,
                    false,
                    fc,
                    addr_masked,
                    memory_bus::BusAccessSize::Word,
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

    /// CCK2: Writes low word of 32-bit destination to memory + 2 and retires
    pub fn step_bus_write_dst_long_low_and_retire(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            return Some(self.trigger_address_error_step(addr, false, false, bus));
        }
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_word(addr_masked, val) {
            BusResult::WaitState => {
                self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
                self.state.micro.current_cycle_wait_cycles =
                    self.state.micro.current_cycle_wait_cycles.wrapping_add(1);
                Some(StepResult::WaitState)
            }
            BusResult::Ready(()) => {
                let fc = crate::micro::types::data_fc(&self.state);
                self.state.micro.record_bus_transaction(
                    false,
                    false,
                    fc,
                    addr_masked,
                    memory_bus::BusAccessSize::Word,
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


    /// CCK1: Extension word fetch from PC into `self.state.micro.last_read`
    pub fn step_fetch_extension_read(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.pc & 0x00FF_FFFF;
        match bus.read_word(addr) {
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

    /// CCK2: Extension word finish - latches into prefetch[0], advances PC += 2
    pub fn step_fetch_extension_finish(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        let fc = crate::micro::types::prog_fc(&self.state);
        let addr = self.state.pc & 0x00FF_FFFF;
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            addr,
            memory_bus::BusAccessSize::Word,
            self.state.micro.last_read,
            true,
            true,
        );
        self.state.prefetch[0] = self.state.micro.last_read;
        self.state.pc = self.state.pc.wrapping_add(2);
        self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
        self.state.micro.phase = CckPhase::Cck1;
        self.state.micro.current_cycle_wait_cycles = 0;
        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        Some(StepResult::StepCompleted)
    }

    /// CCK1: Prefetch next opcode from PC into `self.state.micro.last_read`
    pub fn step_prefetch_next_read(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.pc & 0x00FF_FFFF;
        match bus.read_word(addr) {
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

    /// CCK2: Prefetch next opcode finish and retire instruction sequentially
    pub fn step_prefetch_next_and_retire(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        let fc = crate::micro::types::prog_fc(&self.state);
        let addr = self.state.pc & 0x00FF_FFFF;
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            addr,
            memory_bus::BusAccessSize::Word,
            self.state.micro.last_read,
            true,
            true,
        );
        let word = self.state.micro.last_read;
        self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
        self.state.micro.phase = CckPhase::Cck1;
        self.state.micro.current_cycle_wait_cycles = 0;
        Some(self.retire_standard(word))
    }

    /// CCK1: Prefetch to scratch buffer from PC into `self.state.micro.last_read`
    pub fn step_prefetch_scratch_read(&mut self, bus: &mut MemoryBus) -> Option<StepResult> {
        let addr = self.state.pc & 0x00FF_FFFF;
        match bus.read_word(addr) {
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

    /// CCK2: Prefetch to scratch buffer finish - latches into scratch_prefetch
    pub fn step_prefetch_scratch_finish(&mut self, _bus: &mut MemoryBus) -> Option<StepResult> {
        let fc = crate::micro::types::prog_fc(&self.state);
        let addr = self.state.pc & 0x00FF_FFFF;
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            addr,
            memory_bus::BusAccessSize::Word,
            self.state.micro.last_read,
            true,
            true,
        );
        self.state.micro.scratch_prefetch = self.state.micro.last_read;
        self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
        self.state.micro.phase = CckPhase::Cck1;
        self.state.micro.current_cycle_wait_cycles = 0;
        self.state.micro.micro_step = self.state.micro.micro_step.wrapping_add(1);
        Some(StepResult::StepCompleted)
    }
}
