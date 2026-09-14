//! 2-Clock Micro-Step Execution Handlers for Control Flow, Stack, and Branches
//!
//! Implements cycle-exact 2-clock micro-steps (1 CCK = 2 CPU clocks) for:
//! - Target instruction pipeline refill (BRA, Bcc, BSR, JMP, JSR, RTS)
//! - Stack push operations (PEA, BSR, JSR)
//! - Stack pop operations (RTS)

use crate::core::Cpu;
use physical_memory::{AddressBus, BusResult};

impl Cpu {
    // ========================================================================
    // Target Opcode Refill Handlers (2 Clocks / 1 CCK per step)
    // ========================================================================

    /// CCK1: Reads first word of target instruction directly into `self.state.micro.irc`
    pub fn step_bus_read_target_opcode_read(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, true);
            return BusResult::Ready(());
        }
        match bus.read_word(addr & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.irc = data;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads second word of target pipeline directly into `self.state.prefetch[0]`
    pub fn step_prefetch_target_read(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, true);
            return BusResult::Ready(());
        }
        match bus.read_word(addr & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.prefetch[0] = data;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Finishes prefetch target read and arms target refill retirement
    pub fn step_prefetch_target_finish(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        self.state.micro.target_refill = true;
        BusResult::Ready(())
    }

    // ========================================================================
    // Stack Push Handlers (PEA, JSR, BSR)
    // ========================================================================

    /// CCK1: Decrements SP -= 4, validates alignment, and idles bus for high word write
    pub fn step_bus_push_stack_high_idle(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7).wrapping_sub(4);
        self.state.write_a(7, sp);
        if (sp & 1) != 0 {
            self.trigger_address_error(sp, false, false);
            return BusResult::Ready(());
        }
        BusResult::Ready(())
    }

    /// CCK2: Writes high word of destination to SP
    pub fn step_bus_push_stack_high_write(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7) & 0x00FF_FFFF;
        let val = ((self.state.micro.destination >> 16) & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK2: Writes low word of destination to SP + 2
    pub fn step_bus_push_stack_low_write(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp_low = self.state.read_a(7).wrapping_add(2) & 0x00FF_FFFF;
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        match bus.write_word(sp_low, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    // ========================================================================
    // Stack Pop Handlers (RTS)
    // ========================================================================

    /// CCK1: Reads high word of return PC from (SP) into `ea_high`
    pub fn step_bus_pop_stack_high_read(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7);
        if (sp & 1) != 0 {
            self.trigger_address_error(sp, true, false);
            return BusResult::Ready(());
        }
        match bus.read_word(sp & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.ea_high = (data as u32) << 16;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Latches high word of return PC, advances SP += 2
    pub fn step_bus_pop_stack_high_finish(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7);
        self.state.write_a(7, sp.wrapping_add(2));
        BusResult::Ready(())
    }

    /// CCK1: Reads low word of return PC from (SP) and combines into `ea_addr`
    pub fn step_bus_pop_stack_low_read(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7);
        if (sp & 1) != 0 {
            self.trigger_address_error(sp, true, false);
            return BusResult::Ready(());
        }
        match bus.read_word(sp & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.ea_addr = self.state.micro.ea_high | (data as u32);
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Latches full return PC into `ea_addr`, advances SP += 2
    pub fn step_bus_pop_stack_low_finish(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7);
        self.state.write_a(7, sp.wrapping_add(2));
        BusResult::Ready(())
    }

    // ========================================================================
    // Exception Processing Handlers (TRAP, Interrupts, Exceptions)
    // ========================================================================

    /// CCK1: Validates stack alignment and idles bus for return PC low word write to SP - 2
    pub fn step_bus_write_trap_pclo_idle(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7).wrapping_sub(2);
        if (sp & 1) != 0 {
            self.trigger_address_error(sp, false, false);
            return BusResult::Ready(());
        }
        BusResult::Ready(())
    }

    /// CCK2: Writes return PC low word to SP - 2
    pub fn step_bus_write_trap_pclo_write(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7).wrapping_sub(2) & 0x00FF_FFFF;
        let val = (self.state.micro.source & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK1: Validates stack alignment and idles bus for old SR write to SP - 6
    pub fn step_bus_write_trap_sr_idle(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7).wrapping_sub(6);
        if (sp & 1) != 0 {
            self.trigger_address_error(sp, false, false);
            return BusResult::Ready(());
        }
        BusResult::Ready(())
    }

    /// CCK2: Writes old SR to SP - 6
    pub fn step_bus_write_trap_sr_write(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7).wrapping_sub(6) & 0x00FF_FFFF;
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK1: Validates stack alignment and idles bus for return PC high word write to SP - 4
    pub fn step_bus_write_trap_pchi_idle(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7).wrapping_sub(4);
        if (sp & 1) != 0 {
            self.trigger_address_error(sp, false, false);
            return BusResult::Ready(());
        }
        BusResult::Ready(())
    }

    /// CCK2: Writes return PC high word to SP - 4 and commits updated SP = SP - 6
    pub fn step_bus_write_trap_pchi_write(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp_base = self.state.read_a(7);
        let sp = sp_base.wrapping_sub(4) & 0x00FF_FFFF;
        let val = ((self.state.micro.source >> 16) & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => {
                self.state.write_a(7, sp_base.wrapping_sub(6));
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads exception vector high word from `ea_addr` into `ea_high`
    pub fn step_bus_read_vector_high_read(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, false);
            return BusResult::Ready(());
        }
        match bus.read_word(addr & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.ea_high = (data as u32) << 16;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads exception vector low word from `ea_addr + 2` into `source`
    pub fn step_bus_read_vector_low_read(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, false);
            return BusResult::Ready(());
        }
        match bus.read_word(addr & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source = data as u32;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Finishes exception vector low word read, checks target alignment, and updates `ea_addr`
    pub fn step_bus_read_vector_low_finish(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let val = (self.state.micro.source & 0xFFFF) as u16;
        let target = self.state.micro.ea_high | (val as u32);
        if (target & 1) != 0 {
            if self.state.micro.current_steps.as_ptr()
                == crate::micro::common::STEPS_ADDRESS_ERROR.as_ptr()
            {
                self.state.halted = true;
                return BusResult::Ready(());
            }
            self.trigger_address_error(target, true, true);
            return BusResult::Ready(());
        }
        self.state.micro.ea_addr = target;
        BusResult::Ready(())
    }

    // ========================================================================
    // Group 0 Address Error Exception Handlers
    // ========================================================================

    /// CCK1: Validates SSP alignment and idles bus for PC low word write to SSP - 2
    pub fn step_bus_write_aerr_pclo_idle(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(2);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        BusResult::Ready(())
    }

    /// CCK2: Writes return PC low word to SSP - 2
    pub fn step_bus_write_aerr_pclo_write(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(2) & 0x00FF_FFFF;
        let val = (self.state.micro.source & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK1: Validates SSP alignment and idles bus for old SR write to SSP - 6
    pub fn step_bus_write_aerr_sr_idle(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(6);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        BusResult::Ready(())
    }

    /// CCK2: Writes old SR to SSP - 6
    pub fn step_bus_write_aerr_sr_write(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(6) & 0x00FF_FFFF;
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK1: Validates SSP alignment and idles bus for return PC high word write to SSP - 4
    pub fn step_bus_write_aerr_pchi_idle(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(4);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        BusResult::Ready(())
    }

    /// CCK2: Writes return PC high word to SSP - 4
    pub fn step_bus_write_aerr_pchi_write(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(4) & 0x00FF_FFFF;
        let val = ((self.state.micro.source >> 16) & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK1: Validates SSP alignment and idles bus for instruction register (IR) write to SSP - 8
    pub fn step_bus_write_aerr_ir_idle(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(8);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        BusResult::Ready(())
    }

    /// CCK2: Writes instruction register (IR) to SSP - 8
    pub fn step_bus_write_aerr_ir_write(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(8) & 0x00FF_FFFF;
        let val = self.state.ir;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK1: Validates SSP alignment and idles bus for access address low word write to SSP - 10
    pub fn step_bus_write_aerr_addr_lo_idle(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(10);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        BusResult::Ready(())
    }

    /// CCK2: Writes access address low word to SSP - 10
    pub fn step_bus_write_aerr_addr_lo_write(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(10) & 0x00FF_FFFF;
        let val = (self.state.micro.fault_addr & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK1: Validates SSP alignment and idles bus for internal information word write to SSP - 14
    pub fn step_bus_write_aerr_info_idle(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(14);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        BusResult::Ready(())
    }

    /// CCK2: Writes internal information word to SSP - 14
    pub fn step_bus_write_aerr_info_write(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(14) & 0x00FF_FFFF;
        let val = self.state.micro.info_word;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => BusResult::Ready(()),
        }
    }

    /// CCK1: Validates SSP alignment and idles bus for access address high word write to SSP - 12
    pub fn step_bus_write_aerr_addr_hi_idle(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(12);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        BusResult::Ready(())
    }

    /// CCK2: Writes access address high word to SSP - 12 and commits updated SSP = SSP - 14
    pub fn step_bus_write_aerr_addr_hi_write(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp_base = self.state.micro.ssp_base;
        let sp = sp_base.wrapping_sub(12) & 0x00FF_FFFF;
        let val = ((self.state.micro.fault_addr >> 16) & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(()) => {
                self.state.write_a(7, sp_base.wrapping_sub(14));
                BusResult::Ready(())
            }
        }
    }

    // ========================================================================
    // Prefetch Queue Refill Handlers (SR / CCR Modifications)
    // ========================================================================

    /// CCK1: Refills first word of prefetch queue from PC - 2 directly into `prefetch[0]`
    pub fn step_bus_read_refill_first(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.pc.wrapping_sub(2);
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, true);
            return BusResult::Ready(());
        }
        match bus.read_word(addr & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.prefetch[0] = data;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Refills second word of prefetch queue from PC directly into `micro.irc`
    pub fn step_bus_read_refill_second(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let addr = self.state.pc;
        if (addr & 1) != 0 {
            self.trigger_address_error(addr, true, true);
            return BusResult::Ready(());
        }
        match bus.read_word(addr & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.irc = data;
                BusResult::Ready(())
            }
        }
    }

    // ========================================================================
    // Stack Pop Handlers for SR and CCR (RTE, RTR)
    // ========================================================================

    /// CCK1: Reads status word from (SP) into `micro.source`
    pub fn step_bus_pop_stack_sr_read(&mut self, bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7);
        if (sp & 1) != 0 {
            self.trigger_address_error(sp, true, false);
            return BusResult::Ready(());
        }
        match bus.read_word(sp & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source = data as u32;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Latches status word into `micro.source`, advances SP += 2
    pub fn step_bus_pop_stack_sr_finish(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7);
        self.state.write_a(7, sp.wrapping_add(2));
        BusResult::Ready(())
    }

    /// CCK2: Latches full return PC and sets restored SR for RTE, advances SP += 2
    pub fn step_bus_pop_stack_rte_finish(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7);
        self.state.write_a(7, sp.wrapping_add(2));
        self.state.set_sr(self.state.micro.source as u16);
        BusResult::Ready(())
    }

    /// CCK2: Latches low byte of status word into CCR, advances SP += 2
    pub fn step_bus_pop_stack_ccr_finish(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        let sp = self.state.read_a(7);
        self.state.write_a(7, sp.wrapping_add(2));
        self.state.set_ccr((self.state.micro.source & 0xFF) as u8);
        BusResult::Ready(())
    }
}
