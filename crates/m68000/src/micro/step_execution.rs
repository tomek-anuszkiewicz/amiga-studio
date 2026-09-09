//! Specialized Micro-Step Execution Handlers for M68000 CPU

use crate::core::Cpu;
use memory_bus::{AddressBus, BusResult};

impl Cpu {
    // ========================================================================
    // 2-Clock Micro-Step Handlers (1 CCK = 2 CPU Clocks)
    // ========================================================================

    /// CCK1: Reads source byte from memory into `self.state.micro.source`
    pub fn step_bus_read_src_byte(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr & 0x00FF_FFFF;
        match bus.read_byte(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source = data as u32;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads source word from memory into `self.state.micro.source`
    pub fn step_bus_read_src_word(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, false);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source = data as u32;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads destination byte from memory into `self.state.micro.destination`
    pub fn step_bus_read_dst_byte(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr & 0x00FF_FFFF;
        match bus.read_byte(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.destination = data as u32;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads destination word from memory into `self.state.micro.destination`
    pub fn step_bus_read_dst_word(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, false);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.destination = data as u32;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads high word of 32-bit source operand into bits 16..31 of `source`
    pub fn step_bus_read_src_long_high(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, false);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source = (data as u32) << 16;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads low word of 32-bit source operand into bits 0..15 of `source`
    pub fn step_bus_read_src_long_low(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, false);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source =
                    (self.state.micro.source & 0xFFFF_0000) | (data as u32 & 0xFFFF);
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads high word of 32-bit destination operand into bits 16..31 of `destination`
    pub fn step_bus_read_dst_long_high(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, false);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.destination = (data as u32) << 16;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads low word of 32-bit destination operand into bits 0..15 of `destination`
    pub fn step_bus_read_dst_long_low(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, false);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.destination =
                    (self.state.micro.destination & 0xFFFF_0000) | (data as u32 & 0xFFFF);
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads high word of 32-bit split source operand (predecrement) into bits 16..31 of `source`
    pub fn step_bus_read_src_split_high(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, false);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source =
                    (self.state.micro.source & 0x0000_FFFF) | ((data as u32) << 16);
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads high word of 32-bit split destination operand (predecrement) into bits 16..31 of `destination`
    pub fn step_bus_read_dst_split_high(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, false);
            return BusResult::Ready(());
        }
        let addr = addr & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.destination =
                    (self.state.micro.destination & 0x0000_FFFF) | ((data as u32) << 16);
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Writes byte from `self.state.micro.destination` to memory
    pub fn step_bus_write_dst_byte(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        let val = (self.state.micro.destination & 0xFF) as u8;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_byte(addr_masked, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK2: Writes word from `self.state.micro.destination` to memory
    pub fn step_bus_write_dst_word(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, false, false);
            return BusResult::Ready(());
        }
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_word(addr_masked, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK2: Writes high word of 32-bit destination to memory
    pub fn step_bus_write_dst_long_high(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, false, false);
            return BusResult::Ready(());
        }
        let val = ((self.state.micro.destination >> 16) & 0xFFFF) as u16;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_word(addr_masked, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK2: Writes low word of 32-bit destination to memory + 2
    pub fn step_bus_write_dst_long_low(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, false, false);
            return BusResult::Ready(());
        }
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        let addr_masked = addr & 0x00FF_FFFF;
        match bus.write_word(addr_masked, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK1: Extension word fetch from PC directly into `self.state.prefetch[0]`
    pub fn step_fetch_extension_read(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.pc & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.prefetch[0] = data;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Extension word finish - advances PC += 2
    pub fn step_fetch_extension_finish(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        self.state.pc = self.state.pc.wrapping_add(2);
        BusResult::Ready(())
    }

    /// CCK1: Prefetch next opcode from PC into `self.state.micro.irc`
    pub fn step_prefetch_next_read(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.pc & 0x00FF_FFFF;
        match bus.read_word(addr) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.irc = data;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Prefetch to IRC finish - advances prefetch pipeline into IR
    pub fn step_prefetch_irc_finish(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        self.state.ir = self.state.prefetch[0];
        self.state.prefetch[0] = self.state.micro.irc;
        self.state.pc = self.state.pc.wrapping_add(2);
        self.state.micro.prefetch_retired = true;
        BusResult::Ready(())
    }
}
