//! Specialized Micro-Step Execution Handlers for M68000 CPU

use crate::core::Cpu;
use memory_bus::{BusAccessSize, BusResult, MemoryBus};

impl Cpu {
    /// No-op micro-step handler for pure ALU operations and timing delays.
    /// Execution timing, clock countdown, and micro-step progression are
    /// driven directly by the driver loop (`execute_micro_step`).
    #[inline(always)]
    pub fn step_alu(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        BusResult::Ready(())
    }

    // ========================================================================
    // 2-Clock Micro-Step Handlers (1 CCK = 2 CPU Clocks)
    // ========================================================================

    /// CCK1: Reads source byte from memory into `self.state.micro.source`
    pub fn step_bus_read_src_byte(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr & 0x00FF_FFFF;
        match bus.read_byte(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source = data as u32;
                self.state.micro.read_to_dest = false;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads source word from memory into `self.state.micro.source`
    pub fn step_bus_read_src_word(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, true, false, bus);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source = data as u32;
                self.state.micro.read_to_dest = false;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads destination byte from memory into `self.state.micro.destination`
    pub fn step_bus_read_dst_byte(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr & 0x00FF_FFFF;
        match bus.read_byte(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.destination = data as u32;
                self.state.micro.read_to_dest = true;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads destination word from memory into `self.state.micro.destination`
    pub fn step_bus_read_dst_word(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, true, false, bus);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.destination = data as u32;
                self.state.micro.read_to_dest = true;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads high word of 32-bit source operand into `self.state.micro.source`
    pub fn step_bus_read_src_long_high(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, true, false, bus);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source = (data as u32) << 16;
                self.state.micro.read_to_dest = false;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads low word of 32-bit source operand into `self.state.micro.source`
    pub fn step_bus_read_src_long_low(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, true, false, bus);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source = (self.state.micro.source & 0xFFFF_0000) | (data as u32);
                self.state.micro.read_to_dest = false;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads high word of 32-bit destination operand into `self.state.micro.destination`
    pub fn step_bus_read_dst_long_high(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, true, false, bus);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.destination = (data as u32) << 16;
                self.state.micro.read_to_dest = true;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads low word of 32-bit destination operand into `self.state.micro.destination`
    pub fn step_bus_read_dst_long_low(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, true, false, bus);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.destination =
                    (self.state.micro.destination & 0xFFFF_0000) | (data as u32);
                self.state.micro.read_to_dest = true;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Finishes bus read word cycle, records transaction, and releases bus for Agnus DMA
    pub fn step_bus_read_word_finish(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let fc = crate::micro::types::data_fc(&self.state);
        let val = if self.state.micro.read_to_dest {
            (self.state.micro.destination & 0xFFFF) as u16
        } else {
            (self.state.micro.source & 0xFFFF) as u16
        };
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            self.state.micro.ea_addr,
            BusAccessSize::Word,
            val,
            true,
            true,
        );
        BusResult::Ready(())
    }

    /// CCK1: Reads high word of 32-bit split source operand (predecrement) into bits 16..31 of `source`
    pub fn step_bus_read_src_split_high(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, true, false, bus);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source = (self.state.micro.source & 0x0000_FFFF) | ((data as u32) << 16);
                self.state.micro.read_to_dest = false;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads high word of 32-bit split destination operand (predecrement) into bits 16..31 of `destination`
    pub fn step_bus_read_dst_split_high(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, true, false, bus);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.destination = (self.state.micro.destination & 0x0000_FFFF) | ((data as u32) << 16);
                self.state.micro.read_to_dest = true;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Finishes bus read split high word cycle, records transaction from bits 16..31
    pub fn step_bus_read_split_high_finish(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let fc = crate::micro::types::data_fc(&self.state);
        let val = if self.state.micro.read_to_dest {
            ((self.state.micro.destination >> 16) & 0xFFFF) as u16
        } else {
            ((self.state.micro.source >> 16) & 0xFFFF) as u16
        };
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            self.state.micro.ea_addr,
            BusAccessSize::Word,
            val,
            true,
            true,
        );
        BusResult::Ready(())
    }

    /// CCK2: Finishes bus read byte cycle, records transaction, and releases bus for Agnus DMA
    pub fn step_bus_read_byte_finish(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let fc = crate::micro::types::data_fc(&self.state);
        let addr = self.state.micro.ea_addr;
        let (uds, lds) = if (addr & 1) == 0 { (true, false) } else { (false, true) };
        let val = if self.state.micro.read_to_dest {
            (self.state.micro.destination & 0xFF) as u16
        } else {
            (self.state.micro.source & 0xFF) as u16
        };
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            addr,
            BusAccessSize::Byte,
            val,
            uds,
            lds,
        );
        BusResult::Ready(())
    }

    /// CCK1: Bus write setup (idle on bus, preparing address/pins, bus free for Agnus DMA)
    #[inline(always)]
    pub fn step_bus_write_idle(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        BusResult::Ready(())
    }

    /// CCK2: Writes byte from `self.state.micro.destination` to memory
    pub fn step_bus_write_dst_byte(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        let val = (self.state.micro.destination & 0xFF) as u8;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_byte(addr_masked, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Legacy forwarding alias for retiring byte write
    #[inline(always)]
    pub fn step_bus_write_dst_byte_and_retire(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        self.step_bus_write_dst_byte(bus)
    }

    /// CCK2: Writes word from `self.state.micro.destination` to memory
    pub fn step_bus_write_dst_word(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, false, false, bus);
            return BusResult::Ready(());
        }
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_word(addr_masked, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Legacy forwarding alias for retiring word write
    #[inline(always)]
    pub fn step_bus_write_dst_word_and_retire(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        self.step_bus_write_dst_word(bus)
    }

    /// CCK2: Writes high word of 32-bit destination to memory
    pub fn step_bus_write_dst_long_high(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, false, false, bus);
            return BusResult::Ready(());
        }
        let val = ((self.state.micro.destination >> 16) & 0xFFFF) as u16;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_word(addr_masked, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Writes low word of 32-bit destination to memory + 2
    pub fn step_bus_write_dst_long_low(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, false, false, bus);
            return BusResult::Ready(());
        }
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_word(addr_masked, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Legacy forwarding alias for retiring long low write
    #[inline(always)]
    pub fn step_bus_write_dst_long_low_and_retire(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        self.step_bus_write_dst_long_low(bus)
    }

    /// CCK1: Extension word fetch from PC directly into `self.state.prefetch[0]`
    pub fn step_fetch_extension_read(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.pc & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.prefetch[0] = data;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Extension word finish - logs prefetch[0] transaction, advances PC += 2
    pub fn step_fetch_extension_finish(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let fc = crate::micro::types::prog_fc(&self.state);
        let addr = self.state.pc & 0x00FF_FFFF;
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            addr,
            memory_bus::BusAccessSize::Word,
            self.state.prefetch[0],
            true,
            true,
        );
        self.state.pc = self.state.pc.wrapping_add(2);
        BusResult::Ready(())
    }

    /// CCK1: Prefetch next opcode from PC into `self.state.micro.irc`
    pub fn step_prefetch_next_read(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.pc & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.irc = data;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Prefetch next opcode finish and latches into irc
    pub fn step_prefetch_next_finish(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let fc = crate::micro::types::prog_fc(&self.state);
        let addr = self.state.pc & 0x00FF_FFFF;
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            addr,
            memory_bus::BusAccessSize::Word,
            self.state.micro.irc,
            true,
            true,
        );
        BusResult::Ready(())
    }

    /// CCK2: Legacy forwarding alias for retiring prefetch finish
    #[inline(always)]
    pub fn step_prefetch_next_and_retire(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        self.step_prefetch_next_finish(bus)
    }

    /// CCK1: Prefetch to IRC from PC directly into `self.state.micro.irc`
    pub fn step_prefetch_irc_read(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.pc & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.irc = data;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Legacy forwarding alias for prefetch to scratch/IRC
    #[inline(always)]
    pub fn step_prefetch_scratch_read(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        self.step_prefetch_irc_read(bus)
    }

    /// CCK2: Prefetch to IRC finish - advances prefetch pipeline into IR
    pub fn step_prefetch_irc_finish(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let fc = crate::micro::types::prog_fc(&self.state);
        let addr = self.state.pc & 0x00FF_FFFF;
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            addr,
            memory_bus::BusAccessSize::Word,
            self.state.micro.irc,
            true,
            true,
        );
        self.state.ir = self.state.prefetch[0];
        self.state.prefetch[0] = self.state.micro.irc;
        self.state.pc = self.state.pc.wrapping_add(2);
        self.state.micro.prefetch_retired = true;
        BusResult::Ready(())
    }

    /// CCK2: Legacy forwarding alias for prefetch to scratch/IRC finish
    #[inline(always)]
    pub fn step_prefetch_scratch_finish(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        self.step_prefetch_irc_finish(bus)
    }
}
