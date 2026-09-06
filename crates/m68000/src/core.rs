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

/// Motorola 68000 CPU Core
#[derive(Debug, Clone)]
pub struct Cpu {
    pub state: CpuState,
    /// Number of wait-state cycles currently accumulated
    pub wait_cycles: u32,
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
        }
    }

    /// Reset CPU according to Cold/Warm reset specification
    pub fn reset(&mut self, bus: &mut MemoryBus) {
        self.state.sr = 0x2700;
        self.state.stopped = false;
        self.state.halted = false;
        self.state.step = 0;

        // Fetch initial SSP from $000000
        let ssp_hi = bus.read_word_debug(0x000000);
        let ssp_lo = bus.read_word_debug(0x000002);
        self.state.ssp = ((ssp_hi as u32) << 16) | (ssp_lo as u32);

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

        // For now, execute one instruction step and retire
        self.step_instruction(bus)
    }

    /// Executes exactly one full M68000 instruction via direct table dispatch
    #[inline(always)]
    pub fn step_instruction(&mut self, bus: &mut MemoryBus) -> StepResult {
        if self.state.halted {
            return StepResult::Halted;
        }
        if self.state.stopped {
            return StepResult::Stopped;
        }

        // Direct table dispatch: index directly into static 65,536-entry jump table in .rodata
        let handler = crate::dispatch_table::DISPATCH_TABLE[self.state.ir as usize];
        handler(self, bus)
    }

    /// Retire current instruction and refill prefetch pipeline for sequential instructions
    #[inline]
    pub fn retire_instruction(&mut self, bus: &mut MemoryBus) {
        self.state.ir = self.state.prefetch[0];
        self.state.prefetch[0] = bus.read_word_debug(self.state.pc);
        self.state.pc = self.state.pc.wrapping_add(2);
    }

    /// Consumes the next extension word from the prefetch pipeline
    #[inline]
    pub fn consume_extension_word(&mut self, bus: &mut MemoryBus) -> u16 {
        let ext = self.state.prefetch[0];
        self.state.prefetch[0] = bus.read_word_debug(self.state.pc);
        self.state.pc = self.state.pc.wrapping_add(2);
        ext
    }

    /// Reloads PC and prefetches the next two instruction words (after branch or jump)
    pub fn reload_pc_and_prefetch(&mut self, target_pc: u32, bus: &mut MemoryBus) {
        self.state.pc = target_pc & 0x00FF_FFFF;
        self.state.ir = bus.read_word_debug(self.state.pc);
        self.state.pc = self.state.pc.wrapping_add(2);
        self.state.prefetch[0] = bus.read_word_debug(self.state.pc);
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
                    move_ops::sign_extend_word(val as u16)
                } else {
                    val
                };
                self.state.write_a(reg as usize, final_val);
                Ok(())
            }
            AddressingMode::Immediate(_) => Err(EaError::IllegalAddressingMode),
            _ => {
                let addr = ea.resolve_address(&mut self.state, size)?;
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

    /// Handles address error exception by pushing 7-word stack frame
    pub(crate) fn handle_address_error(&mut self, fault_addr: u32, is_read: bool, bus: &mut MemoryBus) {
        system::push_address_error_exception(
            &mut self.state,
            fault_addr,
            is_read,
            if self.state.is_supervisor() { 5 } else { 1 },
            |addr, val| bus.write_word_debug(addr, val),
            |addr| {
                let hi = bus.read_word_debug(addr);
                let lo = bus.read_word_debug(addr.wrapping_add(2));
                ((hi as u32) << 16) | (lo as u32)
            },
        );
        self.reload_pc_and_prefetch(self.state.pc, bus);
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
