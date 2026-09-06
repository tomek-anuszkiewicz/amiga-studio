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

    /// Executes exactly one full M68000 instruction
    pub fn step_instruction(&mut self, bus: &mut MemoryBus) -> StepResult {
        if self.state.halted {
            return StepResult::Halted;
        }
        if self.state.stopped {
            return StepResult::Stopped;
        }

        let opcode = self.state.ir;

        // Dispatch instruction by opcode bit patterns
        let res = self.execute_opcode(opcode, bus);

        // Retire instruction and refill prefetch pipeline unless branch/jump already updated prefetch
        if res == StepResult::InstructionCompleted {
            self.retire_instruction(bus);
        }

        res
    }

    /// Prefetch retirement: Transfers IRC -> IR, reads next word into IRC, increments PC
    pub fn retire_instruction(&mut self, bus: &mut MemoryBus) {
        self.state.ir = self.state.prefetch[0];
        let next_word = bus.read_word_debug(self.state.pc);
        self.state.prefetch[0] = next_word;
        self.state.pc = self.state.pc.wrapping_add(2);
    }

    /// Helper to consume an extension word from the prefetch queue
    pub fn consume_extension_word(&mut self, bus: &mut MemoryBus) -> u16 {
        let ext = self.state.prefetch[0];
        let next_word = bus.read_word_debug(self.state.pc);
        self.state.prefetch[0] = next_word;
        self.state.pc = self.state.pc.wrapping_add(2);
        ext
    }

    /// Dispatches and executes the opcode in IR
    fn execute_opcode(&mut self, opcode: u16, bus: &mut MemoryBus) -> StepResult {
        // 1. NOP ($4E71)
        if opcode == 0x4E71 {
            return StepResult::InstructionCompleted;
        }

        // 2. RTS ($4E75)
        if opcode == 0x4E75 {
            let sp = self.state.a7();
            let hi = bus.read_word_debug(sp);
            let lo = bus.read_word_debug(sp.wrapping_add(2));
            self.state.set_a7(sp.wrapping_add(4));
            let target = (((hi as u32) << 16) | (lo as u32)) & 0x00FF_FFFF;
            self.reload_pc_and_prefetch(target, bus);
            return StepResult::InstructionCompleted;
        }

        // 3. TRAP ($4E40..$4E4F)
        if (opcode & 0xFFF0) == 0x4E40 {
            let vector_num = (opcode & 0x000F) as u32;
            let vector_addr = system::VECTOR_TRAP_BASE + (vector_num * 4);
            let return_pc = self.state.pc;
            system::push_standard_exception(
                &mut self.state,
                vector_addr,
                return_pc,
                |addr, val| bus.write_word_debug(addr, val),
                |addr| {
                    let hi = bus.read_word_debug(addr);
                    let lo = bus.read_word_debug(addr.wrapping_add(2));
                    ((hi as u32) << 16) | (lo as u32)
                },
            );
            self.reload_pc_and_prefetch(self.state.pc, bus);
            return StepResult::InstructionCompleted;
        }

        // 4. BRA and Bcc ($6000..$6FFF)
        if (opcode & 0xF000) == 0x6000 {
            let cond = ((opcode >> 8) & 0x0F) as u8;
            let d8 = (opcode & 0x00FF) as i8;
            let base_pc = self.state.pc.wrapping_sub(2);

            let displacement = if d8 == 0 {
                self.consume_extension_word(bus) as i16 as i32
            } else {
                d8 as i32
            };

            if let Some(target) = control::evaluate_bcc(&self.state, cond, base_pc, displacement) {
                self.reload_pc_and_prefetch(target, bus);
            }
            return StepResult::InstructionCompleted;
        }

        // 5. JMP ($4EC0..$4EFF)
        if (opcode & 0xFFC0) == 0x4EC0 {
            let mode = ((opcode >> 3) & 0x07) as u8;
            let reg = (opcode & 0x07) as u8;
            let mut ext_reader = || self.consume_extension_word(bus);
            match AddressingMode::decode(mode, reg, Size::Long, self.state.pc, &mut ext_reader) {
                Ok(ea) => match ea.resolve_address(&mut self.state, Size::Long) {
                    Ok(target) => {
                        self.reload_pc_and_prefetch(target, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(EaError::AddressError { addr, is_read }) => {
                        self.handle_address_error(addr, is_read, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                },
                Err(_) => {
                    self.state.halted = true;
                    return StepResult::Halted;
                }
            }
        }

        // 6. JSR ($4E80..$4EBF)
        if (opcode & 0xFFC0) == 0x4E80 {
            let mode = ((opcode >> 3) & 0x07) as u8;
            let reg = (opcode & 0x07) as u8;
            let mut ext_reader = || self.consume_extension_word(bus);
            match AddressingMode::decode(mode, reg, Size::Long, self.state.pc, &mut ext_reader) {
                Ok(ea) => match ea.resolve_address(&mut self.state, Size::Long) {
                    Ok(target) => {
                        let return_pc = self.state.pc;
                        let sp = self.state.a7().wrapping_sub(4);
                        self.state.set_a7(sp);
                        bus.write_word_debug(sp, (return_pc >> 16) as u16);
                        bus.write_word_debug(sp.wrapping_add(2), (return_pc & 0xFFFF) as u16);
                        self.reload_pc_and_prefetch(target, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(EaError::AddressError { addr, is_read }) => {
                        self.handle_address_error(addr, is_read, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                },
                Err(_) => {
                    self.state.halted = true;
                    return StepResult::Halted;
                }
            }
        }

        // 7. MOVE and MOVEA (00ss_dddd_ddmm_mrrr)
        let top2 = (opcode >> 14) & 0x03;
        if top2 == 0 {
            let size_bits = (opcode >> 12) & 0x03;
            if size_bits != 0 {
                let size = match size_bits {
                    1 => Size::Byte,
                    3 => Size::Word,
                    2 => Size::Long,
                    _ => unreachable!(),
                };

                let dst_reg = ((opcode >> 9) & 0x07) as u8;
                let dst_mode = ((opcode >> 6) & 0x07) as u8;
                let src_mode = ((opcode >> 3) & 0x07) as u8;
                let src_reg = (opcode & 0x07) as u8;

                // Decode source EA
                let mut ext_reader = || self.consume_extension_word(bus);
                let src_ea = match AddressingMode::decode(
                    src_mode,
                    src_reg,
                    size,
                    self.state.pc,
                    &mut ext_reader,
                ) {
                    Ok(ea) => ea,
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                };

                let src_val = match self.read_ea_value(&src_ea, size, bus) {
                    Ok(val) => val,
                    Err(EaError::AddressError { addr, is_read }) => {
                        self.handle_address_error(addr, is_read, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                };

                // Check if MOVEA (dst_mode == 1)
                if dst_mode == 1 {
                    let final_val = if size == Size::Word {
                        move_ops::sign_extend_word(src_val as u16)
                    } else {
                        src_val
                    };
                    self.state.write_a(dst_reg as usize, final_val);
                    return StepResult::InstructionCompleted;
                }

                // Standard MOVE: decode destination EA
                let mut ext_reader2 = || self.consume_extension_word(bus);
                let dst_ea = match AddressingMode::decode(
                    dst_mode,
                    dst_reg,
                    size,
                    self.state.pc,
                    &mut ext_reader2,
                ) {
                    Ok(ea) => ea,
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                };

                match self.write_ea_value(&dst_ea, src_val, size, bus) {
                    Ok(()) => {
                        move_ops::update_ccr_move(&mut self.state, src_val, size);
                        return StepResult::InstructionCompleted;
                    }
                    Err(EaError::AddressError { addr, is_read }) => {
                        self.handle_address_error(addr, is_read, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                }
            }
        }

        // 8. ADD and SUB ($D000 / $9000)
        let op_group = (opcode >> 12) & 0x0F;
        if op_group == 0xD || op_group == 0x9 {
            let is_sub = op_group == 0x9;
            let reg_d = ((opcode >> 9) & 0x07) as usize;
            let opmode = ((opcode >> 6) & 0x07) as u8;
            let ea_mode = ((opcode >> 3) & 0x07) as u8;
            let ea_reg = (opcode & 0x07) as u8;

            let (size, ea_is_source, is_adda) = match opmode {
                0 => (Size::Byte, true, false),
                1 => (Size::Word, true, false),
                2 => (Size::Long, true, false),
                3 => (Size::Word, true, true), // ADDA/SUBA.W
                4 => (Size::Byte, false, false),
                5 => (Size::Word, false, false),
                6 => (Size::Long, false, false),
                7 => (Size::Long, true, true), // ADDA/SUBA.L
                _ => unreachable!(),
            };

            let mut ext_reader = || self.consume_extension_word(bus);
            let ea = match AddressingMode::decode(
                ea_mode,
                ea_reg,
                size,
                self.state.pc,
                &mut ext_reader,
            ) {
                Ok(ea) => ea,
                Err(_) => {
                    self.state.halted = true;
                    return StepResult::Halted;
                }
            };

            if is_adda {
                let src_val = match self.read_ea_value(&ea, size, bus) {
                    Ok(val) => {
                        if size == Size::Word {
                            move_ops::sign_extend_word(val as u16)
                        } else {
                            val
                        }
                    }
                    Err(EaError::AddressError { addr, is_read }) => {
                        self.handle_address_error(addr, is_read, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                };
                let dst_val = self.state.read_a(reg_d);
                let res = if is_sub {
                    dst_val.wrapping_sub(src_val)
                } else {
                    dst_val.wrapping_add(src_val)
                };
                self.state.write_a(reg_d, res);
                return StepResult::InstructionCompleted;
            }

            if ea_is_source {
                let src_val = match self.read_ea_value(&ea, size, bus) {
                    Ok(val) => val,
                    Err(EaError::AddressError { addr, is_read }) => {
                        self.handle_address_error(addr, is_read, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                };
                let dst_val = self.state.d[reg_d];
                let res = if is_sub {
                    arithmetic::execute_sub(&mut self.state, src_val, dst_val, size, true)
                } else {
                    arithmetic::execute_add(&mut self.state, src_val, dst_val, size, true)
                };
                self.write_d_reg(reg_d, res, size);
                return StepResult::InstructionCompleted;
            } else {
                let src_val = self.state.d[reg_d];
                let dst_val = match self.read_ea_value(&ea, size, bus) {
                    Ok(val) => val,
                    Err(EaError::AddressError { addr, is_read }) => {
                        self.handle_address_error(addr, is_read, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                };
                let res = if is_sub {
                    arithmetic::execute_sub(&mut self.state, src_val, dst_val, size, true)
                } else {
                    arithmetic::execute_add(&mut self.state, src_val, dst_val, size, true)
                };
                match self.write_ea_value(&ea, res, size, bus) {
                    Ok(()) => return StepResult::InstructionCompleted,
                    Err(EaError::AddressError { addr, is_read }) => {
                        self.handle_address_error(addr, is_read, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                }
            }
        }

        // 9. AND and OR ($C000 / $8000)
        if op_group == 0xC || op_group == 0x8 {
            let is_and = op_group == 0xC;
            let reg_d = ((opcode >> 9) & 0x07) as usize;
            let opmode = ((opcode >> 6) & 0x07) as u8;
            let ea_mode = ((opcode >> 3) & 0x07) as u8;
            let ea_reg = (opcode & 0x07) as u8;

            let (size, ea_is_source) = match opmode {
                0 => (Size::Byte, true),
                1 => (Size::Word, true),
                2 => (Size::Long, true),
                4 => (Size::Byte, false),
                5 => (Size::Word, false),
                6 => (Size::Long, false),
                _ => {
                    self.state.halted = true;
                    return StepResult::Halted;
                }
            };

            let mut ext_reader = || self.consume_extension_word(bus);
            let ea = match AddressingMode::decode(
                ea_mode,
                ea_reg,
                size,
                self.state.pc,
                &mut ext_reader,
            ) {
                Ok(ea) => ea,
                Err(_) => {
                    self.state.halted = true;
                    return StepResult::Halted;
                }
            };

            if ea_is_source {
                let src_val = match self.read_ea_value(&ea, size, bus) {
                    Ok(val) => val,
                    Err(EaError::AddressError { addr, is_read }) => {
                        self.handle_address_error(addr, is_read, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                };
                let dst_val = self.state.d[reg_d];
                let res = if is_and {
                    logic::execute_and(&mut self.state, src_val, dst_val, size)
                } else {
                    logic::execute_or(&mut self.state, src_val, dst_val, size)
                };
                self.write_d_reg(reg_d, res, size);
                return StepResult::InstructionCompleted;
            } else {
                let src_val = self.state.d[reg_d];
                let dst_val = match self.read_ea_value(&ea, size, bus) {
                    Ok(val) => val,
                    Err(EaError::AddressError { addr, is_read }) => {
                        self.handle_address_error(addr, is_read, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                };
                let res = if is_and {
                    logic::execute_and(&mut self.state, src_val, dst_val, size)
                } else {
                    logic::execute_or(&mut self.state, src_val, dst_val, size)
                };
                match self.write_ea_value(&ea, res, size, bus) {
                    Ok(()) => return StepResult::InstructionCompleted,
                    Err(EaError::AddressError { addr, is_read }) => {
                        self.handle_address_error(addr, is_read, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                }
            }
        }

        // 10. Shifts and Rotates ($E000..$EFFF)
        if op_group == 0xE {
            let is_left = (opcode & 0x0100) != 0;
            let size_bits = (opcode >> 6) & 0x03;
            let is_reg_count = (opcode & 0x0020) != 0;
            let shift_type = (opcode >> 3) & 0x03;
            let reg_dst = (opcode & 0x07) as usize;

            if size_bits != 3 {
                let size = match size_bits {
                    0 => Size::Byte,
                    1 => Size::Word,
                    2 => Size::Long,
                    _ => unreachable!(),
                };

                let count = if is_reg_count {
                    let reg_cnt = ((opcode >> 9) & 0x07) as usize;
                    self.state.d[reg_cnt]
                } else {
                    let imm = ((opcode >> 9) & 0x07) as u32;
                    if imm == 0 {
                        8
                    } else {
                        imm
                    }
                };

                let val = self.state.d[reg_dst];
                let res = match (shift_type, is_left) {
                    (0, true) => shifts::execute_asl(&mut self.state, count, val, size),
                    (0, false) => shifts::execute_asr(&mut self.state, count, val, size),
                    (1, true) => shifts::execute_lsl(&mut self.state, count, val, size),
                    (1, false) => shifts::execute_lsr(&mut self.state, count, val, size),
                    _ => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                };
                self.write_d_reg(reg_dst, res, size);
                return StepResult::InstructionCompleted;
            }
        }

        // 11. BTST, BSET, BCLR, BCHG ($0800..$08FF or dynamic $0100..$01FF)
        if (opcode & 0xF100) == 0x0100 || (opcode & 0xFF00) == 0x0800 {
            let is_static = (opcode & 0xFF00) == 0x0800;
            let op_type = (opcode >> 6) & 0x03;
            let ea_mode = ((opcode >> 3) & 0x07) as u8;
            let ea_reg = (opcode & 0x07) as u8;

            let bit_num = if is_static {
                self.consume_extension_word(bus) as u32
            } else {
                let reg_num = ((opcode >> 9) & 0x07) as usize;
                self.state.d[reg_num]
            };

            let is_reg = ea_mode == 0;
            let size = if is_reg { Size::Long } else { Size::Byte };

            let mut ext_reader = || self.consume_extension_word(bus);
            let ea = match AddressingMode::decode(
                ea_mode,
                ea_reg,
                size,
                self.state.pc,
                &mut ext_reader,
            ) {
                Ok(ea) => ea,
                Err(_) => {
                    self.state.halted = true;
                    return StepResult::Halted;
                }
            };

            let val = match self.read_ea_value(&ea, size, bus) {
                Ok(val) => val,
                Err(EaError::AddressError { addr, is_read }) => {
                    self.handle_address_error(addr, is_read, bus);
                    return StepResult::InstructionCompleted;
                }
                Err(_) => {
                    self.state.halted = true;
                    return StepResult::Halted;
                }
            };

            let new_val = match op_type {
                0 => {
                    bits::execute_btst(&mut self.state, bit_num, val, is_reg);
                    None
                }
                1 => Some(bits::execute_bchg(&mut self.state, bit_num, val, is_reg)),
                2 => Some(bits::execute_bclr(&mut self.state, bit_num, val, is_reg)),
                3 => Some(bits::execute_bset(&mut self.state, bit_num, val, is_reg)),
                _ => unreachable!(),
            };

            if let Some(res) = new_val {
                match self.write_ea_value(&ea, res, size, bus) {
                    Ok(()) => return StepResult::InstructionCompleted,
                    Err(EaError::AddressError { addr, is_read }) => {
                        self.handle_address_error(addr, is_read, bus);
                        return StepResult::InstructionCompleted;
                    }
                    Err(_) => {
                        self.state.halted = true;
                        return StepResult::Halted;
                    }
                }
            } else {
                return StepResult::InstructionCompleted;
            }
        }

        // Unimplemented opcode in Step 3a -> halt
        self.state.halted = true;
        StepResult::Halted
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
    fn handle_address_error(&mut self, fault_addr: u32, is_read: bool, bus: &mut MemoryBus) {
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
    fn write_d_reg(&mut self, reg: usize, val: u32, size: Size) {
        let current = self.state.d[reg];
        self.state.d[reg] = match size {
            Size::Byte => (current & !0xFF) | (val & 0xFF),
            Size::Word => (current & !0xFFFF) | (val & 0xFFFF),
            Size::Long => val,
        };
    }
}
