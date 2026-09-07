//! Cycle-exact Motorola 68000 CPU Execution Core

use crate::addressing::{AddressingMode, EaError, Size};
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
        self.state.a[7] = self.state.ssp;

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
                    crate::micro::MicroRetireMode::None => {}
                    crate::micro::MicroRetireMode::StandardPrefetch => {
                        self.state.ir = self.state.prefetch[0];
                        self.state.prefetch[0] = self.state.micro.last_read;
                        self.state.pc = self.state.pc.wrapping_add(2);
                        self.state.micro.reset();
                        return StepResult::InstructionCompleted;
                    }
                    crate::micro::MicroRetireMode::ScratchPrefetch => {
                        self.state.ir = self.state.prefetch[0];
                        self.state.prefetch[0] = self.state.micro.scratch_prefetch;
                        self.state.pc = self.state.pc.wrapping_add(2);
                        self.state.micro.reset();
                        return StepResult::InstructionCompleted;
                    }
                    crate::micro::MicroRetireMode::TargetRefill { target, new_ir } => {
                        self.state.ir = new_ir;
                        self.state.prefetch[0] = self.state.micro.last_read;
                        self.state.pc = target.wrapping_add(4);
                        self.state.micro.reset();
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

        // 3. Dispatch current micro-step of current instruction
        self.state.instruction_pc = self.state.pc.wrapping_sub(2);
        let handler = crate::dispatch_table::DISPATCH_TABLE[self.state.ir as usize];
        let res = handler(self, bus);

        // If the handler initiated a bus cycle on CCK1, immediately execute CCK1 for that cycle
        if self.state.micro.is_bus_busy() && self.state.micro.phase == memory_bus::CckPhase::Cck1 {
            let bus_res = self.state.micro.step_cck(bus, &mut self.wait_cycles);
            self.total_clocks = self.total_clocks.wrapping_add(2);
            self.instruction_clocks = self.instruction_clocks.wrapping_add(2);
            if bus_res.is_wait() {
                return StepResult::WaitState;
            }
            return StepResult::StepCompleted;
        }

        // If handler completed synchronously (for non-migrated opcodes):
        if res.is_completed() {
            if self.instruction_clocks == 0 {
                self.total_clocks = self.total_clocks.wrapping_add(4);
                self.instruction_clocks = self.instruction_clocks.wrapping_add(4);
            }
            self.state.micro.reset();
            return StepResult::InstructionCompleted;
        }

        res
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

    /// Read value from effective address
    pub fn read_ea_value(
        &mut self,
        ea: &AddressingMode,
        size: Size,
        bus: &mut MemoryBus,
    ) -> Result<u32, EaError> {
        match *ea {
            AddressingMode::DataDirect(reg) => {
                let d = self.state.d[reg as usize];
                Ok(match size {
                    Size::Byte => d & 0xFF,
                    Size::Word => d & 0xFFFF,
                    Size::Long => d,
                })
            }
            AddressingMode::AddressDirect(reg) => {
                let a = self.state.read_a(reg as usize);
                Ok(match size {
                    Size::Byte => a & 0xFF,
                    Size::Word => a & 0xFFFF,
                    Size::Long => a,
                })
            }
            AddressingMode::Immediate(val) => Ok(val),
            _ => {
                let addr = ea.resolve_address(&mut self.state, size)?;
                match size {
                    Size::Byte => Ok(bus.read_byte_debug(addr) as u32),
                    Size::Word => Ok(bus.read_word_debug(addr) as u32),
                    Size::Long => {
                        let hi = bus.read_word_debug(addr) as u32;
                        let lo = bus.read_word_debug(addr.wrapping_add(2)) as u32;
                        Ok((hi << 16) | lo)
                    }
                }
            }
        }
    }

    /// Write value to effective address
    pub fn write_ea_value(
        &mut self,
        ea: &AddressingMode,
        val: u32,
        size: Size,
        bus: &mut MemoryBus,
    ) -> Result<(), EaError> {
        match *ea {
            AddressingMode::DataDirect(reg) => {
                self.write_d_reg(reg as usize, val, size);
                Ok(())
            }
            AddressingMode::AddressDirect(reg) => {
                // Writing to address register sign-extends on Word size
                let final_val = if size == Size::Word {
                    movea::sign_extend_word(val as u16)
                } else {
                    val
                };
                self.state.write_a(reg as usize, final_val);
                Ok(())
            }
            AddressingMode::Immediate(_) => Err(EaError::IllegalAddressingMode),
            AddressingMode::Postincrement(reg) => {
                let a = self.state.read_a(reg as usize);
                if size != Size::Byte && (a & 1) != 0 {
                    return Err(EaError::AddressError {
                        addr: a,
                        is_read: false,
                    });
                }
                let delta = if size == Size::Byte && reg == 7 {
                    2
                } else {
                    size.byte_count()
                };
                self.state.write_a(reg as usize, a.wrapping_add(delta));
                match size {
                    Size::Byte => {
                        bus.write_byte_debug(a, (val & 0xFF) as u8);
                    }
                    Size::Word => {
                        bus.write_word_debug(a, (val & 0xFFFF) as u16);
                    }
                    Size::Long => {
                        bus.write_word_debug(a, (val >> 16) as u16);
                        bus.write_word_debug(a.wrapping_add(2), (val & 0xFFFF) as u16);
                    }
                }
                Ok(())
            }
            AddressingMode::Predecrement(reg) => {
                let a = self.state.read_a(reg as usize);
                if size == Size::Long {
                    let addr_low = a.wrapping_sub(2);
                    self.state.write_a(reg as usize, addr_low);
                    if (addr_low & 1) != 0 {
                        return Err(EaError::AddressError {
                            addr: addr_low,
                            is_read: false,
                        });
                    }
                    let addr_high = a.wrapping_sub(4);
                    self.state.write_a(reg as usize, addr_high);
                    bus.write_word_debug(addr_low, (val & 0xFFFF) as u16);
                    bus.write_word_debug(addr_high, (val >> 16) as u16);
                    Ok(())
                } else {
                    let delta = if size == Size::Byte && reg == 7 {
                        2
                    } else {
                        size.byte_count()
                    };
                    let new_a = a.wrapping_sub(delta);
                    self.state.write_a(reg as usize, new_a);
                    if size != Size::Byte && (new_a & 1) != 0 {
                        return Err(EaError::AddressError {
                            addr: new_a,
                            is_read: false,
                        });
                    }
                    match size {
                        Size::Byte => bus.write_byte_debug(new_a, (val & 0xFF) as u8),
                        Size::Word => bus.write_word_debug(new_a, (val & 0xFFFF) as u16),
                        Size::Long => unreachable!(),
                    }
                    Ok(())
                }
            }
            _ => {
                let addr = ea
                    .resolve_address(&mut self.state, size)
                    .map_err(|e| match e {
                        EaError::AddressError { addr, .. } => EaError::AddressError {
                            addr,
                            is_read: false,
                        },
                        other => other,
                    })?;
                match size {
                    Size::Byte => {
                        bus.write_byte_debug(addr, (val & 0xFF) as u8);
                    }
                    Size::Word => {
                        bus.write_word_debug(addr, (val & 0xFFFF) as u16);
                    }
                    Size::Long => {
                        bus.write_word_debug(addr, (val >> 16) as u16);
                        bus.write_word_debug(addr.wrapping_add(2), (val & 0xFFFF) as u16);
                    }
                }
                Ok(())
            }
        }
    }

    /// Reads value from EA for a Read-Modify-Write operation.
    /// If EA is memory-based, resolves the address ONCE and returns `(value, Some(addr))`.
    /// If EA is register-based (`DataDirect`), returns `(value, None)`.
    pub fn read_ea_modify(
        &mut self,
        ea: &AddressingMode,
        size: Size,
        bus: &mut MemoryBus,
    ) -> Result<(u32, Option<u32>), EaError> {
        match *ea {
            AddressingMode::DataDirect(reg) => {
                let d = self.state.d[reg as usize];
                let val = match size {
                    Size::Byte => d & 0xFF,
                    Size::Word => d & 0xFFFF,
                    Size::Long => d,
                };
                Ok((val, None))
            }
            AddressingMode::AddressDirect(reg) => {
                let a = self.state.read_a(reg as usize);
                let val = match size {
                    Size::Byte => a & 0xFF,
                    Size::Word => a & 0xFFFF,
                    Size::Long => a,
                };
                Ok((val, None))
            }
            AddressingMode::Immediate(val) => Ok((val, None)),
            _ => {
                let addr = ea.resolve_address(&mut self.state, size)?;
                let val = match size {
                    Size::Byte => bus.read_byte_debug(addr) as u32,
                    Size::Word => bus.read_word_debug(addr) as u32,
                    Size::Long => {
                        let hi = bus.read_word_debug(addr) as u32;
                        let lo = bus.read_word_debug(addr.wrapping_add(2)) as u32;
                        (hi << 16) | lo
                    }
                };
                Ok((val, Some(addr)))
            }
        }
    }

    /// Writes modified value back to EA without re-resolving address.
    pub fn write_ea_modify(
        &mut self,
        ea: &AddressingMode,
        addr: Option<u32>,
        val: u32,
        size: Size,
        bus: &mut MemoryBus,
    ) -> Result<(), EaError> {
        match addr {
            Some(addr) => {
                match size {
                    Size::Byte => {
                        bus.write_byte_debug(addr, (val & 0xFF) as u8);
                    }
                    Size::Word => {
                        bus.write_word_debug(addr, (val & 0xFFFF) as u16);
                    }
                    Size::Long => {
                        bus.write_word_debug(addr, (val >> 16) as u16);
                        bus.write_word_debug(addr.wrapping_add(2), (val & 0xFFFF) as u16);
                    }
                }
                Ok(())
            }
            None => match *ea {
                AddressingMode::DataDirect(reg) => {
                    self.write_d_reg(reg as usize, val, size);
                    Ok(())
                }
                AddressingMode::AddressDirect(reg) => {
                    let final_val = if size == Size::Word {
                        movea::sign_extend_word(val as u16)
                    } else {
                        val
                    };
                    self.state.write_a(reg as usize, final_val);
                    Ok(())
                }
                _ => Err(EaError::IllegalAddressingMode),
            },
        }
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
    }

    /// Handles address error exception by pushing 7-word stack frame
    pub(crate) fn handle_address_error(
        &mut self,
        fault_addr: u32,
        is_read: bool,
        bus: &mut MemoryBus,
    ) {
        let function_code = if self.state.is_supervisor() { 5 } else { 1 };
        self.handle_address_error_fc(fault_addr, is_read, function_code, bus);
    }

    #[allow(dead_code)]
    pub(crate) fn handle_address_error_for_ea(
        &mut self,
        ea: &AddressingMode,
        fault_addr: u32,
        is_read: bool,
        bus: &mut MemoryBus,
    ) {
        let is_sup = self.state.is_supervisor();
        let function_code = if ea.is_program_space() {
            if is_sup {
                6
            } else {
                2
            }
        } else {
            if is_sup {
                5
            } else {
                1
            }
        };
        self.handle_address_error_fc(fault_addr, is_read, function_code, bus);
    }

    #[inline]
    pub(crate) fn write_d_reg(&mut self, reg: usize, val: u32, size: Size) {
        let current = self.state.d[reg];
        self.state.d[reg] = match size {
            Size::Byte => (current & !0xFF) | (val & 0xFF),
            Size::Word => (current & !0xFFFF) | (val & 0xFFFF),
            Size::Long => val,
        };
    }
}
