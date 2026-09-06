//! 65,536-entry compile-time static direct-dispatch jump table
//!
//! Pre-evaluated at compile time and stored directly in .rodata for instant
//! execution without cascaded runtime branch mispredictions.

use crate::addressing::{AddressingMode, EaError, Size};
use crate::core::{Cpu, StepResult};
use crate::instructions::*;
use memory_bus::MemoryBus;

/// Function pointer signature for an M68000 direct opcode handler
pub type OpcodeHandler = fn(&mut Cpu, &mut MemoryBus) -> StepResult;

/// 65,536-entry direct dispatch table placed in read-only static memory (.rodata)
pub static DISPATCH_TABLE: [OpcodeHandler; 65536] = build_dispatch_table();

/// Compile-time function evaluating the opcode table at build time
pub const fn build_dispatch_table() -> [OpcodeHandler; 65536] {
    let mut table: [OpcodeHandler; 65536] = [op_unimplemented; 65536];
    let mut i = 0;
    while i < 65536 {
        let op = i as u16;
        table[i] = decode_opcode_handler(op);
        i += 1;
    }
    table
}

/// Classifies a 16-bit opcode at compile time and assigns its direct handler
const fn decode_opcode_handler(op: u16) -> OpcodeHandler {
    // 1. NOP
    if op == 0x4E71 {
        return op_nop;
    }
    // 2. RTS
    if op == 0x4E75 {
        return op_rts;
    }
    // 3. TRAP
    if (op & 0xFFF0) == 0x4E40 {
        return op_trap;
    }
    // 4. BRA and Bcc
    if (op & 0xF000) == 0x6000 {
        return op_bra_bcc;
    }
    // 5. JMP
    if (op & 0xFFC0) == 0x4EC0 {
        return op_jmp;
    }
    // 6. JSR
    if (op & 0xFFC0) == 0x4E80 {
        return op_jsr;
    }
    // 7. MOVE and MOVEA
    let top2 = (op >> 14) & 0x03;
    if top2 == 0 {
        let size_bits = (op >> 12) & 0x03;
        if size_bits != 0 {
            return op_move;
        }
    }
    // ADDX and SUBX: 1101/1001 [Rx:3] 1 [size:2] 00 [R/M:1] [Ry:3]
    if (op & 0xF130) == 0xD100 && ((op >> 6) & 0x03) != 3 {
        return op_addx;
    }
    if (op & 0xF130) == 0x9100 && ((op >> 6) & 0x03) != 3 {
        return op_subx;
    }
    // ADDQ and SUBQ
    if (op & 0xF000) == 0x5000 {
        let size_bits = (op >> 6) & 0x03;
        if size_bits != 3 {
            return op_addq_subq;
        }
    }
    // ADDI and SUBI
    if (op & 0xFF00) == 0x0600 || (op & 0xFF00) == 0x0400 {
        let size_bits = (op >> 6) & 0x03;
        if size_bits != 3 {
            return op_addi_subi;
        }
    }
    // 8. ADD and SUB
    let op_group = (op >> 12) & 0x0F;
    if op_group == 0xD {
        return op_add;
    }
    if op_group == 0x9 {
        return op_sub;
    }
    // 9. AND and OR
    if op_group == 0xC {
        return op_and;
    }
    if op_group == 0x8 {
        return op_or;
    }
    // 10. Shifts and Rotates
    if op_group == 0xE {
        return op_shift;
    }
    // 11. Bit Manipulation (BTST, BSET, BCLR, BCHG)
    if (op & 0xF100) == 0x0100 || (op & 0xFF00) == 0x0800 {
        return op_bit;
    }

    op_unimplemented
}

// --- Specialized Direct Handlers ---

pub fn op_nop(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    cpu.retire_instruction(bus);
    StepResult::InstructionCompleted
}

pub fn op_rts(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let sp = cpu.state.a7();
    if (sp & 1) != 0 {
        cpu.handle_address_error(sp, true, bus);
        return StepResult::InstructionCompleted;
    }
    let hi = bus.read_word_debug(sp);
    let lo = bus.read_word_debug(sp.wrapping_add(2));
    cpu.state.set_a7(sp.wrapping_add(4));
    let target = ((hi as u32) << 16) | (lo as u32);
    if (target & 1) != 0 {
        let fc = if cpu.state.is_supervisor() { 6 } else { 2 };
        cpu.handle_address_error_fc(target, true, fc, bus);
        return StepResult::InstructionCompleted;
    }
    cpu.reload_pc_and_prefetch(target, bus);
    StepResult::InstructionCompleted
}

pub fn op_trap(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let vector_num = (opcode & 0x000F) as u32;
    let vector_addr = system::VECTOR_TRAP_BASE + (vector_num * 4);
    let return_pc = cpu.state.pc;
    system::push_standard_exception(
        &mut cpu.state,
        vector_addr,
        return_pc,
        bus,
    );
    cpu.reload_pc_and_prefetch(cpu.state.pc, bus);
    StepResult::InstructionCompleted
}

pub fn op_bra_bcc(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let cond = ((opcode >> 8) & 0x0F) as u8;
    let d8 = (opcode & 0x00FF) as i8;
    let base_pc = cpu.state.pc.wrapping_sub(2);

    let displacement = if d8 == 0 {
        cpu.consume_extension_word(bus) as i16 as i32
    } else {
        d8 as i32
    };

    if let Some(target) = control::evaluate_bcc(&cpu.state, cond, base_pc, displacement) {
        cpu.reload_pc_and_prefetch(target, bus);
    } else {
        cpu.retire_instruction(bus);
    }
    StepResult::InstructionCompleted
}

pub fn op_jmp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let mode = ((opcode >> 3) & 0x07) as u8;
    let reg = (opcode & 0x07) as u8;
    let pc = cpu.state.pc.wrapping_sub(2);
    let mut ext_reader = || cpu.consume_extension_word(bus);
    match AddressingMode::decode(mode, reg, Size::Long, pc, &mut ext_reader) {
        Ok(ea) => match ea.resolve_address(&mut cpu.state, Size::Long) {
            Ok(target) => {
                if (target & 1) != 0 {
                    let fc = if cpu.state.is_supervisor() { 6 } else { 2 };
                    cpu.handle_address_error_fc(target, true, fc, bus);
                    return StepResult::InstructionCompleted;
                }
                cpu.reload_pc_and_prefetch(target, bus);
                StepResult::InstructionCompleted
            }
            Err(EaError::AddressError { addr, is_read }) => {
                let fc = if cpu.state.is_supervisor() { 6 } else { 2 };
                cpu.handle_address_error_fc(addr, is_read, fc, bus);
                StepResult::InstructionCompleted
            }
            Err(_) => {
                cpu.state.halted = true;
                StepResult::Halted
            }
        },
        Err(_) => {
            cpu.state.halted = true;
            StepResult::Halted
        }
    }
}

pub fn op_jsr(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let mode = ((opcode >> 3) & 0x07) as u8;
    let reg = (opcode & 0x07) as u8;
    let pc = cpu.state.pc.wrapping_sub(2);
    let mut ext_reader = || cpu.consume_extension_word(bus);
    match AddressingMode::decode(mode, reg, Size::Long, pc, &mut ext_reader) {
        Ok(ea) => match ea.resolve_address(&mut cpu.state, Size::Long) {
            Ok(target) => {
                if (target & 1) != 0 {
                    let fc = if cpu.state.is_supervisor() { 6 } else { 2 };
                    cpu.handle_address_error_fc(target, true, fc, bus);
                    return StepResult::InstructionCompleted;
                }
                let return_pc = cpu.state.pc;
                let sp = cpu.state.a7().wrapping_sub(4);
                cpu.state.set_a7(sp);
                bus.write_word_debug(sp, (return_pc >> 16) as u16);
                bus.write_word_debug(sp.wrapping_add(2), (return_pc & 0xFFFF) as u16);
                cpu.reload_pc_and_prefetch(target, bus);
                StepResult::InstructionCompleted
            }
            Err(EaError::AddressError { addr, is_read }) => {
                let fc = if cpu.state.is_supervisor() { 6 } else { 2 };
                cpu.handle_address_error_fc(addr, is_read, fc, bus);
                StepResult::InstructionCompleted
            }
            Err(_) => {
                cpu.state.halted = true;
                StepResult::Halted
            }
        },
        Err(_) => {
            cpu.state.halted = true;
            StepResult::Halted
        }
    }
}

pub fn op_move(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let size_bits = (opcode >> 12) & 0x03;
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

    let pc = cpu.state.pc;
    let mut ext_reader = || cpu.consume_extension_word(bus);
    let src_ea = match AddressingMode::decode(src_mode, src_reg, size, pc, &mut ext_reader)
    {
        Ok(ea) => ea,
        Err(_) => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };

    let src_val = match cpu.read_ea_value(&src_ea, size, bus) {
        Ok(val) => val,
        Err(EaError::AddressError { addr, is_read }) => {
            cpu.handle_address_error(addr, is_read, bus);
            return StepResult::InstructionCompleted;
        }
        Err(_) => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };

    // MOVEA (dst_mode == 1)
    if dst_mode == 1 {
        let final_val = if size == Size::Word {
            move_ops::sign_extend_word(src_val as u16)
        } else {
            src_val
        };
        cpu.state.write_a(dst_reg as usize, final_val);
        cpu.retire_instruction(bus);
        return StepResult::InstructionCompleted;
    }

    let pc = cpu.state.pc;
    let mut ext_reader2 = || cpu.consume_extension_word(bus);
    let dst_ea =
        match AddressingMode::decode(dst_mode, dst_reg, size, pc, &mut ext_reader2) {
            Ok(ea) => ea,
            Err(_) => {
                cpu.state.halted = true;
                return StepResult::Halted;
            }
        };

    match cpu.write_ea_value(&dst_ea, src_val, size, bus) {
        Ok(()) => {
            move_ops::update_ccr_move(&mut cpu.state, src_val, size);
            cpu.retire_instruction(bus);
            StepResult::InstructionCompleted
        }
        Err(EaError::AddressError { addr, is_read }) => {
            cpu.handle_address_error(addr, is_read, bus);
            StepResult::InstructionCompleted
        }
        Err(_) => {
            cpu.state.halted = true;
            StepResult::Halted
        }
    }
}

pub fn op_add(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    execute_arithmetic_group(cpu, bus, false)
}

pub fn op_sub(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    execute_arithmetic_group(cpu, bus, true)
}

fn execute_arithmetic_group(cpu: &mut Cpu, bus: &mut MemoryBus, is_sub: bool) -> StepResult {
    let opcode = cpu.state.ir;
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

    let pc = cpu.state.pc.wrapping_sub(2);
    let mut ext_reader = || cpu.consume_extension_word(bus);
    let ea = match AddressingMode::decode(ea_mode, ea_reg, size, pc, &mut ext_reader) {
        Ok(ea) => ea,
        Err(_) => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };

    if is_adda {
        let src_val = match cpu.read_ea_value(&ea, size, bus) {
            Ok(val) => {
                if size == Size::Word {
                    move_ops::sign_extend_word(val as u16)
                } else {
                    val
                }
            }
            Err(EaError::AddressError { addr, is_read }) => {
                cpu.handle_address_error_for_ea(&ea, addr, is_read, bus);
                return StepResult::InstructionCompleted;
            }
            Err(_) => {
                cpu.state.halted = true;
                return StepResult::Halted;
            }
        };
        let dst_val = cpu.state.read_a(reg_d);
        let res = if is_sub {
            dst_val.wrapping_sub(src_val)
        } else {
            dst_val.wrapping_add(src_val)
        };
        cpu.state.write_a(reg_d, res);
        cpu.retire_instruction(bus);
        return StepResult::InstructionCompleted;
    }

    if ea_is_source {
        let src_val = match cpu.read_ea_value(&ea, size, bus) {
            Ok(val) => val,
            Err(EaError::AddressError { addr, is_read }) => {
                cpu.handle_address_error_for_ea(&ea, addr, is_read, bus);
                return StepResult::InstructionCompleted;
            }
            Err(_) => {
                cpu.state.halted = true;
                return StepResult::Halted;
            }
        };
        let dst_val = cpu.state.d[reg_d];
        let res = if is_sub {
            arithmetic::execute_sub(&mut cpu.state, src_val, dst_val, size, true)
        } else {
            arithmetic::execute_add(&mut cpu.state, src_val, dst_val, size, true)
        };
        cpu.write_d_reg(reg_d, res, size);
        cpu.retire_instruction(bus);
        StepResult::InstructionCompleted
    } else {
        let src_val = cpu.state.d[reg_d];
        let (dst_val, addr_opt) = match cpu.read_ea_modify(&ea, size, bus) {
            Ok(res) => res,
            Err(EaError::AddressError { addr, is_read }) => {
                cpu.handle_address_error_for_ea(&ea, addr, is_read, bus);
                return StepResult::InstructionCompleted;
            }
            Err(_) => {
                cpu.state.halted = true;
                return StepResult::Halted;
            }
        };
        let res = if is_sub {
            arithmetic::execute_sub(&mut cpu.state, src_val, dst_val, size, true)
        } else {
            arithmetic::execute_add(&mut cpu.state, src_val, dst_val, size, true)
        };
        match cpu.write_ea_modify(&ea, addr_opt, res, size, bus) {
            Ok(()) => {
                cpu.retire_instruction(bus);
                StepResult::InstructionCompleted
            }
            Err(EaError::AddressError { addr, is_read }) => {
                cpu.handle_address_error_for_ea(&ea, addr, is_read, bus);
                StepResult::InstructionCompleted
            }
            Err(_) => {
                cpu.state.halted = true;
                StepResult::Halted
            }
        }
    }
}

pub fn op_and(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    execute_logic_group(cpu, bus, true)
}

pub fn op_or(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    execute_logic_group(cpu, bus, false)
}

fn execute_logic_group(cpu: &mut Cpu, bus: &mut MemoryBus, is_and: bool) -> StepResult {
    let opcode = cpu.state.ir;
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
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };

    let pc = cpu.state.pc;
    let mut ext_reader = || cpu.consume_extension_word(bus);
    let ea = match AddressingMode::decode(ea_mode, ea_reg, size, pc, &mut ext_reader) {
        Ok(ea) => ea,
        Err(_) => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };

    if ea_is_source {
        let src_val = match cpu.read_ea_value(&ea, size, bus) {
            Ok(val) => val,
            Err(EaError::AddressError { addr, is_read }) => {
                cpu.handle_address_error(addr, is_read, bus);
                return StepResult::InstructionCompleted;
            }
            Err(_) => {
                cpu.state.halted = true;
                return StepResult::Halted;
            }
        };
        let dst_val = cpu.state.d[reg_d];
        let res = if is_and {
            logic::execute_and(&mut cpu.state, src_val, dst_val, size)
        } else {
            logic::execute_or(&mut cpu.state, src_val, dst_val, size)
        };
        cpu.write_d_reg(reg_d, res, size);
        cpu.retire_instruction(bus);
        StepResult::InstructionCompleted
    } else {
        let src_val = cpu.state.d[reg_d];
        let (dst_val, addr_opt) = match cpu.read_ea_modify(&ea, size, bus) {
            Ok(res) => res,
            Err(EaError::AddressError { addr, is_read }) => {
                cpu.handle_address_error(addr, is_read, bus);
                return StepResult::InstructionCompleted;
            }
            Err(_) => {
                cpu.state.halted = true;
                return StepResult::Halted;
            }
        };
        let res = if is_and {
            logic::execute_and(&mut cpu.state, src_val, dst_val, size)
        } else {
            logic::execute_or(&mut cpu.state, src_val, dst_val, size)
        };
        match cpu.write_ea_modify(&ea, addr_opt, res, size, bus) {
            Ok(()) => {
                cpu.retire_instruction(bus);
                StepResult::InstructionCompleted
            }
            Err(EaError::AddressError { addr, is_read }) => {
                cpu.handle_address_error(addr, is_read, bus);
                StepResult::InstructionCompleted
            }
            Err(_) => {
                cpu.state.halted = true;
                StepResult::Halted
            }
        }
    }
}

pub fn op_shift(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
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
            cpu.state.d[reg_cnt]
        } else {
            let imm = ((opcode >> 9) & 0x07) as u32;
            if imm == 0 {
                8
            } else {
                imm
            }
        };

        let val = cpu.state.d[reg_dst];
        let res = match (shift_type, is_left) {
            (0, true) => shifts::execute_asl(&mut cpu.state, count, val, size),
            (0, false) => shifts::execute_asr(&mut cpu.state, count, val, size),
            (1, true) => shifts::execute_lsl(&mut cpu.state, count, val, size),
            (1, false) => shifts::execute_lsr(&mut cpu.state, count, val, size),
            _ => {
                cpu.state.halted = true;
                return StepResult::Halted;
            }
        };
        cpu.write_d_reg(reg_dst, res, size);
        cpu.retire_instruction(bus);
        StepResult::InstructionCompleted
    } else {
        cpu.state.halted = true;
        StepResult::Halted
    }
}

pub fn op_bit(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let is_static = (opcode & 0xFF00) == 0x0800;
    let op_type = (opcode >> 6) & 0x03;
    let ea_mode = ((opcode >> 3) & 0x07) as u8;
    let ea_reg = (opcode & 0x07) as u8;

    let bit_num = if is_static {
        cpu.consume_extension_word(bus) as u32
    } else {
        let reg_num = ((opcode >> 9) & 0x07) as usize;
        cpu.state.d[reg_num]
    };

    let is_reg = ea_mode == 0;
    let size = if is_reg { Size::Long } else { Size::Byte };

    let pc = cpu.state.pc;
    let mut ext_reader = || cpu.consume_extension_word(bus);
    let ea = match AddressingMode::decode(ea_mode, ea_reg, size, pc, &mut ext_reader) {
        Ok(ea) => ea,
        Err(_) => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };

    let (val, addr_opt) = match cpu.read_ea_modify(&ea, size, bus) {
        Ok(res) => res,
        Err(EaError::AddressError { addr, is_read }) => {
            cpu.handle_address_error(addr, is_read, bus);
            return StepResult::InstructionCompleted;
        }
        Err(_) => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };

    let new_val = match op_type {
        0 => {
            bits::execute_btst(&mut cpu.state, bit_num, val, is_reg);
            None
        }
        1 => Some(bits::execute_bchg(&mut cpu.state, bit_num, val, is_reg)),
        2 => Some(bits::execute_bclr(&mut cpu.state, bit_num, val, is_reg)),
        3 => Some(bits::execute_bset(&mut cpu.state, bit_num, val, is_reg)),
        _ => unreachable!(),
    };

    if let Some(res) = new_val {
        match cpu.write_ea_modify(&ea, addr_opt, res, size, bus) {
            Ok(()) => {
                cpu.retire_instruction(bus);
                StepResult::InstructionCompleted
            }
            Err(EaError::AddressError { addr, is_read }) => {
                cpu.handle_address_error(addr, is_read, bus);
                StepResult::InstructionCompleted
            }
            Err(_) => {
                cpu.state.halted = true;
                StepResult::Halted
            }
        }
    } else {
        cpu.retire_instruction(bus);
        StepResult::InstructionCompleted
    }
}

pub fn op_addx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    execute_addx_subx(cpu, bus, false)
}

pub fn op_subx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    execute_addx_subx(cpu, bus, true)
}

fn execute_addx_subx(cpu: &mut Cpu, bus: &mut MemoryBus, is_sub: bool) -> StepResult {
    let opcode = cpu.state.ir;
    let rx = ((opcode >> 9) & 0x07) as usize;
    let ry = (opcode & 0x07) as usize;
    let size_bits = (opcode >> 6) & 0x03;
    let size = match size_bits {
        0 => Size::Byte,
        1 => Size::Word,
        2 => Size::Long,
        _ => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };
    let rm = (opcode & 0x0008) != 0;

    if !rm {
        let src_val = cpu.state.d[ry];
        let dst_val = cpu.state.d[rx];
        let res = if is_sub {
            arithmetic::execute_subx(&mut cpu.state, src_val, dst_val, size)
        } else {
            arithmetic::execute_addx(&mut cpu.state, src_val, dst_val, size)
        };
        cpu.write_d_reg(rx, res, size);
        cpu.retire_instruction(bus);
        StepResult::InstructionCompleted
    } else {
        let ea_src = AddressingMode::Predecrement(ry as u8);
        let ea_dst = AddressingMode::Predecrement(rx as u8);

        let src_val = match cpu.read_ea_value(&ea_src, size, bus) {
            Ok(val) => val,
            Err(EaError::AddressError { addr, is_read }) => {
                cpu.handle_address_error(addr, is_read, bus);
                return StepResult::InstructionCompleted;
            }
            Err(_) => {
                cpu.state.halted = true;
                return StepResult::Halted;
            }
        };

        let (dst_val, addr_opt) = match cpu.read_ea_modify(&ea_dst, size, bus) {
            Ok(res) => res,
            Err(EaError::AddressError { addr, is_read }) => {
                cpu.handle_address_error(addr, is_read, bus);
                return StepResult::InstructionCompleted;
            }
            Err(_) => {
                cpu.state.halted = true;
                return StepResult::Halted;
            }
        };

        let res = if is_sub {
            arithmetic::execute_subx(&mut cpu.state, src_val, dst_val, size)
        } else {
            arithmetic::execute_addx(&mut cpu.state, src_val, dst_val, size)
        };

        match cpu.write_ea_modify(&ea_dst, addr_opt, res, size, bus) {
            Ok(()) => {
                cpu.retire_instruction(bus);
                StepResult::InstructionCompleted
            }
            Err(EaError::AddressError { addr, is_read }) => {
                cpu.handle_address_error(addr, is_read, bus);
                StepResult::InstructionCompleted
            }
            Err(_) => {
                cpu.state.halted = true;
                StepResult::Halted
            }
        }
    }
}

pub fn op_addq_subq(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let data = (opcode >> 9) & 0x07;
    let imm: u32 = if data == 0 { 8 } else { data as u32 };
    let is_sub = (opcode & 0x0100) != 0;
    let size_bits = (opcode >> 6) & 0x03;
    let size = match size_bits {
        0 => Size::Byte,
        1 => Size::Word,
        2 => Size::Long,
        _ => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };
    let ea_mode = ((opcode >> 3) & 0x07) as u8;
    let ea_reg = (opcode & 0x07) as u8;

    let pc = cpu.state.pc;
    let mut ext_reader = || cpu.consume_extension_word(bus);
    let ea = match AddressingMode::decode(ea_mode, ea_reg, size, pc, &mut ext_reader) {
        Ok(ea) => ea,
        Err(_) => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };

    if ea_mode == 1 {
        let val = cpu.state.read_a(ea_reg as usize);
        let res = if is_sub {
            val.wrapping_sub(imm)
        } else {
            val.wrapping_add(imm)
        };
        cpu.state.write_a(ea_reg as usize, res);
        cpu.retire_instruction(bus);
        return StepResult::InstructionCompleted;
    }

    let (dst_val, addr_opt) = match cpu.read_ea_modify(&ea, size, bus) {
        Ok(res) => res,
        Err(EaError::AddressError { addr, is_read }) => {
            cpu.handle_address_error(addr, is_read, bus);
            return StepResult::InstructionCompleted;
        }
        Err(_) => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };

    let res = if is_sub {
        arithmetic::execute_sub(&mut cpu.state, imm, dst_val, size, true)
    } else {
        arithmetic::execute_add(&mut cpu.state, imm, dst_val, size, true)
    };

    match cpu.write_ea_modify(&ea, addr_opt, res, size, bus) {
        Ok(()) => {
            cpu.retire_instruction(bus);
            StepResult::InstructionCompleted
        }
        Err(EaError::AddressError { addr, is_read }) => {
            cpu.handle_address_error(addr, is_read, bus);
            StepResult::InstructionCompleted
        }
        Err(_) => {
            cpu.state.halted = true;
            StepResult::Halted
        }
    }
}

pub fn op_addi_subi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let is_sub = (opcode & 0x0200) == 0;
    let size_bits = (opcode >> 6) & 0x03;
    let size = match size_bits {
        0 => Size::Byte,
        1 => Size::Word,
        2 => Size::Long,
        _ => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };
    let imm = match size {
        Size::Byte => (cpu.consume_extension_word(bus) & 0xFF) as u32,
        Size::Word => cpu.consume_extension_word(bus) as u32,
        Size::Long => {
            let hi = cpu.consume_extension_word(bus) as u32;
            let lo = cpu.consume_extension_word(bus) as u32;
            (hi << 16) | lo
        }
    };
    let ea_mode = ((opcode >> 3) & 0x07) as u8;
    let ea_reg = (opcode & 0x07) as u8;

    let pc = cpu.state.pc;
    let mut ext_reader = || cpu.consume_extension_word(bus);
    let ea = match AddressingMode::decode(ea_mode, ea_reg, size, pc, &mut ext_reader) {
        Ok(ea) => ea,
        Err(_) => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };

    let (dst_val, addr_opt) = match cpu.read_ea_modify(&ea, size, bus) {
        Ok(res) => res,
        Err(EaError::AddressError { addr, is_read }) => {
            cpu.handle_address_error(addr, is_read, bus);
            return StepResult::InstructionCompleted;
        }
        Err(_) => {
            cpu.state.halted = true;
            return StepResult::Halted;
        }
    };

    let res = if is_sub {
        arithmetic::execute_sub(&mut cpu.state, imm, dst_val, size, true)
    } else {
        arithmetic::execute_add(&mut cpu.state, imm, dst_val, size, true)
    };

    match cpu.write_ea_modify(&ea, addr_opt, res, size, bus) {
        Ok(()) => {
            cpu.retire_instruction(bus);
            StepResult::InstructionCompleted
        }
        Err(EaError::AddressError { addr, is_read }) => {
            cpu.handle_address_error(addr, is_read, bus);
            StepResult::InstructionCompleted
        }
        Err(_) => {
            cpu.state.halted = true;
            StepResult::Halted
        }
    }
}

pub fn op_unimplemented(cpu: &mut Cpu, _bus: &mut MemoryBus) -> StepResult {
    cpu.state.halted = true;
    StepResult::Halted
}
