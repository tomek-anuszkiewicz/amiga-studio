//! 2-Clock Micro-Step Execution Handlers for Control Flow, Stack, and Branches
//!
//! Implements cycle-exact 2-clock micro-steps (1 CCK = 2 CPU clocks) for:
//! - Target instruction pipeline refill (BRA, Bcc, BSR, JMP, JSR, RTS)
//! - Stack push operations (PEA, BSR, JSR)
//! - Stack pop operations (RTS)

use crate::core::Cpu;
use crate::micro::types::{data_fc, prog_fc};
use memory_bus::{BusAccessSize, BusResult, CckPhase, MemoryBus};

impl Cpu {
    // ========================================================================
    // Target Opcode Refill Handlers (2 Clocks / 1 CCK per step)
    // ========================================================================

    /// CCK1: Reads first word of target instruction directly into `self.state.micro.irc`
    pub fn step_bus_read_target_opcode_read(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, true, true, bus);
            return BusResult::Ready(());
        }
        match bus.read_word(addr & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.irc = data;
                self.state.micro.phase = CckPhase::Cck2;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Logs target opcode transaction from `irc`
    pub fn step_bus_read_target_opcode_finish(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr & 0x00FF_FFFF;
        let data = self.state.micro.irc;
        let fc = prog_fc(&self.state);
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            addr,
            BusAccessSize::Word,
            data,
            true,
            true,
        );
        self.state.micro.phase = CckPhase::Cck1;
        BusResult::Ready(())
    }

    /// CCK1: Reads second word of target pipeline directly into `self.state.prefetch[0]`
    pub fn step_prefetch_target_read(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, true, true, bus);
            return BusResult::Ready(());
        }
        match bus.read_word(addr & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.prefetch[0] = data;
                self.state.micro.phase = CckPhase::Cck2;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Logs second target word transaction from `prefetch[0]` and arms target refill retirement
    pub fn step_prefetch_target_finish(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr.wrapping_add(2) & 0x00FF_FFFF;
        let target_prefetch = self.state.prefetch[0];
        let fc = prog_fc(&self.state);
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            addr,
            BusAccessSize::Word,
            target_prefetch,
            true,
            true,
        );
        self.state.micro.phase = CckPhase::Cck1;
        self.state.micro.target_refill = true;
        BusResult::Ready(())
    }

    /// CCK2: Legacy forwarding alias for target refill
    #[inline(always)]
    pub fn step_prefetch_target_and_retire_2clk(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        self.step_prefetch_target_finish(bus)
    }

    // ========================================================================
    // Stack Push Handlers (PEA, JSR, BSR)
    // ========================================================================

    /// CCK1: Decrements SP -= 4, validates alignment, and idles bus for high word write
    pub fn step_bus_push_stack_high_idle(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.read_a(7).wrapping_sub(4);
        self.state.write_a(7, sp);
        if (sp & 1) != 0 {
            self.trigger_address_error_step(sp, false, false, bus);
            return BusResult::Ready(());
        }
        self.state.micro.phase = CckPhase::Cck2;
        BusResult::Ready(())
    }

    /// CCK2: Writes high word of destination to SP
    pub fn step_bus_push_stack_high_write(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.read_a(7) & 0x00FF_FFFF;
        let val = ((self.state.micro.destination >> 16) & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                self.state.micro.phase = CckPhase::Cck1;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Writes low word of destination to SP + 2
    pub fn step_bus_push_stack_low_write(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp_low = self.state.read_a(7).wrapping_add(2) & 0x00FF_FFFF;
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        match bus.write_word(sp_low, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                self.state.micro.phase = CckPhase::Cck1;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Legacy forwarding alias for retiring low word push
    #[inline(always)]
    pub fn step_bus_push_stack_low_write_and_retire(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        self.step_bus_push_stack_low_write(bus)
    }

    // ========================================================================
    // Stack Pop Handlers (RTS)
    // ========================================================================

    /// CCK1: Reads high word of return PC from (SP) into `ea_high`
    pub fn step_bus_pop_stack_high_read(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.read_a(7);
        if (sp & 1) != 0 {
            self.trigger_address_error_step(sp, true, false, bus);
            return BusResult::Ready(());
        }
        match bus.read_word(sp & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.ea_high = (data as u32) << 16;
                self.state.micro.phase = CckPhase::Cck2;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Latches high word of return PC, advances SP += 2, and logs transaction
    pub fn step_bus_pop_stack_high_finish(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.read_a(7);
        let data = ((self.state.micro.ea_high >> 16) & 0xFFFF) as u16;
        self.state.write_a(7, sp.wrapping_add(2));
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
        self.state.micro.phase = CckPhase::Cck1;
        BusResult::Ready(())
    }

    /// CCK1: Reads low word of return PC from (SP) and combines into `ea_addr`
    pub fn step_bus_pop_stack_low_read(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.read_a(7);
        if (sp & 1) != 0 {
            self.trigger_address_error_step(sp, true, false, bus);
            return BusResult::Ready(());
        }
        match bus.read_word(sp & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.ea_addr = self.state.micro.ea_high | (data as u32);
                self.state.micro.phase = CckPhase::Cck2;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Latches full return PC into `ea_addr`, advances SP += 2, and logs transaction
    pub fn step_bus_pop_stack_low_finish(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.read_a(7);
        let data = (self.state.micro.ea_addr & 0xFFFF) as u16;
        self.state.write_a(7, sp.wrapping_add(2));
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
        self.state.micro.phase = CckPhase::Cck1;
        BusResult::Ready(())
    }

    // ========================================================================
    // Exception Processing Handlers (TRAP, Interrupts, Exceptions)
    // ========================================================================

    /// CCK1: Validates stack alignment and idles bus for return PC low word write to SP - 2
    pub fn step_bus_write_trap_pclo_idle(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.read_a(7).wrapping_sub(2);
        if (sp & 1) != 0 {
            self.trigger_address_error_step(sp, false, false, bus);
            return BusResult::Ready(());
        }
        self.state.micro.phase = CckPhase::Cck2;
        BusResult::Ready(())
    }

    /// CCK2: Writes return PC low word to SP - 2
    pub fn step_bus_write_trap_pclo_write(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.read_a(7).wrapping_sub(2) & 0x00FF_FFFF;
        let val = (self.state.micro.source & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                self.state.micro.phase = CckPhase::Cck1;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Validates stack alignment and idles bus for old SR write to SP - 6
    pub fn step_bus_write_trap_sr_idle(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.read_a(7).wrapping_sub(6);
        if (sp & 1) != 0 {
            self.trigger_address_error_step(sp, false, false, bus);
            return BusResult::Ready(());
        }
        self.state.micro.phase = CckPhase::Cck2;
        BusResult::Ready(())
    }

    /// CCK2: Writes old SR to SP - 6
    pub fn step_bus_write_trap_sr_write(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.read_a(7).wrapping_sub(6) & 0x00FF_FFFF;
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                self.state.micro.phase = CckPhase::Cck1;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Validates stack alignment and idles bus for return PC high word write to SP - 4
    pub fn step_bus_write_trap_pchi_idle(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.read_a(7).wrapping_sub(4);
        if (sp & 1) != 0 {
            self.trigger_address_error_step(sp, false, false, bus);
            return BusResult::Ready(());
        }
        self.state.micro.phase = CckPhase::Cck2;
        BusResult::Ready(())
    }

    /// CCK2: Writes return PC high word to SP - 4 and commits updated SP = SP - 6
    pub fn step_bus_write_trap_pchi_write(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp_base = self.state.read_a(7);
        let sp = sp_base.wrapping_sub(4) & 0x00FF_FFFF;
        let val = ((self.state.micro.source >> 16) & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                self.state.write_a(7, sp_base.wrapping_sub(6));
                self.state.micro.phase = CckPhase::Cck1;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Reads exception vector high word from `ea_addr` into `ea_high`
    pub fn step_bus_read_vector_high_read(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr;
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, true, false, bus);
            return BusResult::Ready(());
        }
        match bus.read_word(addr & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.ea_high = (data as u32) << 16;
                self.state.micro.phase = CckPhase::Cck2;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Logs exception vector high word read transaction
    pub fn step_bus_read_vector_high_finish(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let fc = data_fc(&self.state);
        let addr = self.state.micro.ea_addr & 0x00FF_FFFF;
        let val = ((self.state.micro.ea_high >> 16) & 0xFFFF) as u16;
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            addr,
            BusAccessSize::Word,
            val,
            true,
            true,
        );
        self.state.micro.phase = CckPhase::Cck1;
        BusResult::Ready(())
    }

    /// CCK1: Reads exception vector low word from `ea_addr + 2` into `source`
    pub fn step_bus_read_vector_low_read(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let addr = self.state.micro.ea_addr.wrapping_add(2);
        if (addr & 1) != 0 {
            self.trigger_address_error_step(addr, true, false, bus);
            return BusResult::Ready(());
        }
        match bus.read_word(addr & 0x00FF_FFFF) {
            BusResult::WaitState => BusResult::WaitState,
            BusResult::Ready(data) => {
                self.state.micro.source = data as u32;
                self.state.micro.phase = CckPhase::Cck2;
                BusResult::Ready(())
            }
        }
    }

    /// CCK2: Logs exception vector low word read transaction, checks target alignment, and updates `ea_addr`
    pub fn step_bus_read_vector_low_finish(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let fc = data_fc(&self.state);
        let addr = self.state.micro.ea_addr.wrapping_add(2) & 0x00FF_FFFF;
        let val = (self.state.micro.source & 0xFFFF) as u16;
        self.state.micro.record_bus_transaction(
            true,
            false,
            fc,
            addr,
            BusAccessSize::Word,
            val,
            true,
            true,
        );
        let target = (self.state.micro.ea_high | (val as u32)) & 0x00FF_FFFF;
        if (target & 1) != 0 {
            if self.state.micro.current_steps.as_ptr() == crate::micro::common::STEPS_ADDRESS_ERROR.as_ptr() {
                self.state.halted = true;
                return BusResult::Ready(());
            }
            self.trigger_address_error_step(target, true, true, bus);
            return BusResult::Ready(());
        }
        self.state.micro.ea_addr = target;
        self.state.micro.phase = CckPhase::Cck1;
        BusResult::Ready(())
    }

    /// CCK1/CCK2: 2-clock internal ALU/processing cycle, recording internal transaction if enabled
    pub fn step_alu_internal_2clk(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        self.state.micro.record_internal_transaction(2);
        BusResult::Ready(())
    }

    // ========================================================================
    // Group 0 Address Error Exception Handlers
    // ========================================================================

    /// CCK1: Validates SSP alignment and idles bus for PC low word write to SSP - 2
    pub fn step_bus_write_aerr_pclo_idle(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(2);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        self.state.micro.phase = CckPhase::Cck2;
        BusResult::Ready(())
    }

    /// CCK2: Writes return PC low word to SSP - 2
    pub fn step_bus_write_aerr_pclo_write(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(2) & 0x00FF_FFFF;
        let val = (self.state.micro.source & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                self.state.micro.phase = CckPhase::Cck1;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Validates SSP alignment and idles bus for old SR write to SSP - 6
    pub fn step_bus_write_aerr_sr_idle(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(6);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        self.state.micro.phase = CckPhase::Cck2;
        BusResult::Ready(())
    }

    /// CCK2: Writes old SR to SSP - 6
    pub fn step_bus_write_aerr_sr_write(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(6) & 0x00FF_FFFF;
        let val = (self.state.micro.destination & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                self.state.micro.phase = CckPhase::Cck1;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Validates SSP alignment and idles bus for return PC high word write to SSP - 4
    pub fn step_bus_write_aerr_pchi_idle(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(4);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        self.state.micro.phase = CckPhase::Cck2;
        BusResult::Ready(())
    }

    /// CCK2: Writes return PC high word to SSP - 4
    pub fn step_bus_write_aerr_pchi_write(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(4) & 0x00FF_FFFF;
        let val = ((self.state.micro.source >> 16) & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                self.state.micro.phase = CckPhase::Cck1;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Validates SSP alignment and idles bus for instruction register (IR) write to SSP - 8
    pub fn step_bus_write_aerr_ir_idle(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(8);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        self.state.micro.phase = CckPhase::Cck2;
        BusResult::Ready(())
    }

    /// CCK2: Writes instruction register (IR) to SSP - 8
    pub fn step_bus_write_aerr_ir_write(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(8) & 0x00FF_FFFF;
        let val = self.state.ir;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                self.state.micro.phase = CckPhase::Cck1;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Validates SSP alignment and idles bus for access address low word write to SSP - 10
    pub fn step_bus_write_aerr_addr_lo_idle(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(10);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        self.state.micro.phase = CckPhase::Cck2;
        BusResult::Ready(())
    }

    /// CCK2: Writes access address low word to SSP - 10
    pub fn step_bus_write_aerr_addr_lo_write(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(10) & 0x00FF_FFFF;
        let val = (self.state.micro.fault_addr & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                self.state.micro.phase = CckPhase::Cck1;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Validates SSP alignment and idles bus for internal information word write to SSP - 14
    pub fn step_bus_write_aerr_info_idle(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(14);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        self.state.micro.phase = CckPhase::Cck2;
        BusResult::Ready(())
    }

    /// CCK2: Writes internal information word to SSP - 14
    pub fn step_bus_write_aerr_info_write(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(14) & 0x00FF_FFFF;
        let val = self.state.micro.info_word;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                self.state.micro.phase = CckPhase::Cck1;
                BusResult::Ready(())
            }
        }
    }

    /// CCK1: Validates SSP alignment and idles bus for access address high word write to SSP - 12
    pub fn step_bus_write_aerr_addr_hi_idle(&mut self, _bus: &mut MemoryBus) -> BusResult<()> {
        let sp = self.state.micro.ssp_base.wrapping_sub(12);
        if (sp & 1) != 0 {
            self.state.halted = true;
            return BusResult::Ready(());
        }
        self.state.micro.phase = CckPhase::Cck2;
        BusResult::Ready(())
    }

    /// CCK2: Writes access address high word to SSP - 12 and commits updated SSP = SSP - 14
    pub fn step_bus_write_aerr_addr_hi_write(&mut self, bus: &mut MemoryBus) -> BusResult<()> {
        let sp_base = self.state.micro.ssp_base;
        let sp = sp_base.wrapping_sub(12) & 0x00FF_FFFF;
        let val = ((self.state.micro.fault_addr >> 16) & 0xFFFF) as u16;
        match bus.write_word(sp, val) {
            BusResult::WaitState => BusResult::WaitState,
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
                self.state.write_a(7, sp_base.wrapping_sub(14));
                self.state.micro.phase = CckPhase::Cck1;
                BusResult::Ready(())
            }
        }
    }
}
