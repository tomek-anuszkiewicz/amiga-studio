//! M68000 System and Exception Processing (NOP, TRAP, Address Error)

use crate::core::{Cpu, StepResult};
use crate::state::{CpuState, SR_T};
use memory_bus::MemoryBus;

pub const VECTOR_ADDRESS_ERROR: u32 = 0x00000C;
pub const VECTOR_PRIVILEGE_VIOLATION: u32 = 0x000020;

/// Initiates standard 3-word exception processing (TRAP, Interrupts, etc.)
pub fn push_standard_exception(
    state: &mut CpuState,
    vector_addr: u32,
    return_pc: u32,
    bus: &mut MemoryBus,
) {
    let old_sr = state.sr;
    // Switch to supervisor mode, clear trace
    state.set_supervisor(true);
    state.sr &= !SR_T;

    // Push PC (high word, low word)
    let sp = state.read_a(7).wrapping_sub(4);
    state.write_a(7, sp);
    bus.write_word_debug(sp, (return_pc >> 16) as u16);
    bus.write_word_debug(sp.wrapping_add(2), (return_pc & 0xFFFF) as u16);

    // Push SR
    let sp = state.read_a(7).wrapping_sub(2);
    state.write_a(7, sp);
    bus.write_word_debug(sp, old_sr);

    // Load new PC from vector
    let hi = bus.read_word_debug(vector_addr);
    let lo = bus.read_word_debug(vector_addr.wrapping_add(2));
    state.pc = ((hi as u32) << 16) | (lo as u32);
}

/// Initiates MC68000 Group 0/1 Address Error (Vector 3) 7-word exception processing
pub fn push_address_error_exception(
    state: &mut CpuState,
    fault_addr: u32,
    is_read: bool,
    function_code: u8,
    bus: &mut MemoryBus,
) {
    let old_sr = state.sr;
    let old_pc = state.instruction_pc;
    let ir = state.ir;

    // Switch to supervisor mode, clear trace
    state.set_supervisor(true);
    state.sr &= !SR_T;

    // Build Internal Information Word:
    // Bits 15-5: Opcode (IR & 0xFFE0)
    // Bit 4: R/W (1 = Read, 0 = Write)
    // Bit 3: I/N (0 = Instruction processing, 1 = Exception)
    // Bits 2-0: Function Code (FC0-FC2)
    let rw_bit = if is_read { 0x10 } else { 0x00 };
    let info_word = (ir & 0xFFE0) | rw_bit | ((function_code as u16) & 0x07);

    // Push 7-word stack frame in reverse order (bottom to top):
    // SP + 12: Low 16 bits of PC
    // SP + 10: High 16 bits of PC
    // SP + 08: SR
    // SP + 06: IR
    // SP + 04: Low 16 bits of Access Address
    // SP + 02: High 16 bits of Access Address
    // SP + 00: Internal Information Word
    let sp = state.read_a(7).wrapping_sub(14);
    state.write_a(7, sp);

    bus.write_word_debug(sp, info_word);
    bus.write_word_debug(sp.wrapping_add(2), (fault_addr >> 16) as u16);
    bus.write_word_debug(sp.wrapping_add(4), (fault_addr & 0xFFFF) as u16);
    bus.write_word_debug(sp.wrapping_add(6), ir);
    bus.write_word_debug(sp.wrapping_add(8), old_sr);
    bus.write_word_debug(sp.wrapping_add(10), (old_pc >> 16) as u16);
    bus.write_word_debug(sp.wrapping_add(12), (old_pc & 0xFFFF) as u16);

    // Vector 3 ($00000C)
    let hi = bus.read_word_debug(VECTOR_ADDRESS_ERROR);
    let lo = bus.read_word_debug(VECTOR_ADDRESS_ERROR.wrapping_add(2));
    state.pc = ((hi as u32) << 16) | (lo as u32);
}

// ============================================================================
// CCR and SR Manipulation (ORI, ANDI, EORI)
// ============================================================================

pub fn op_ori_to_ccr(cpu: &mut Cpu, bus: &mut MemoryBus) -> Option<StepResult> {
    let imm = cpu.consume_extension_word(bus) & 0x1F;
    cpu.state.sr = (cpu.state.sr & !0x1F) | ((cpu.state.sr | imm) & 0x1F);
    cpu.retire_instruction(bus);
    Some(StepResult::InstructionCompleted)
}

pub fn op_ori_to_sr(cpu: &mut Cpu, bus: &mut MemoryBus) -> Option<StepResult> {
    if !cpu.state.is_supervisor() {
        let ret_pc = cpu.state.instruction_pc;
        push_standard_exception(
            &mut cpu.state,
            VECTOR_PRIVILEGE_VIOLATION,
            ret_pc,
            bus,
        );
        cpu.reload_pc_and_prefetch(cpu.state.pc, bus);
        return Some(StepResult::InstructionCompleted);
    }
    let imm = cpu.consume_extension_word(bus);
    let new_sr = cpu.state.sr | imm;
    cpu.state.set_sr(new_sr);
    cpu.retire_instruction(bus);
    Some(StepResult::InstructionCompleted)
}

pub fn op_andi_to_ccr(cpu: &mut Cpu, bus: &mut MemoryBus) -> Option<StepResult> {
    let imm = cpu.consume_extension_word(bus) & 0x1F;
    cpu.state.sr = (cpu.state.sr & !0x1F) | (cpu.state.sr & imm);
    cpu.retire_instruction(bus);
    Some(StepResult::InstructionCompleted)
}

pub fn op_andi_to_sr(cpu: &mut Cpu, bus: &mut MemoryBus) -> Option<StepResult> {
    if !cpu.state.is_supervisor() {
        let ret_pc = cpu.state.instruction_pc;
        push_standard_exception(
            &mut cpu.state,
            VECTOR_PRIVILEGE_VIOLATION,
            ret_pc,
            bus,
        );
        cpu.reload_pc_and_prefetch(cpu.state.pc, bus);
        return Some(StepResult::InstructionCompleted);
    }
    let imm = cpu.consume_extension_word(bus);
    let new_sr = cpu.state.sr & imm;
    cpu.state.set_sr(new_sr);
    cpu.retire_instruction(bus);
    Some(StepResult::InstructionCompleted)
}

pub fn op_eori_to_ccr(cpu: &mut Cpu, bus: &mut MemoryBus) -> Option<StepResult> {
    let imm = cpu.consume_extension_word(bus) & 0x1F;
    cpu.state.sr = (cpu.state.sr & !0x1F) | ((cpu.state.sr ^ imm) & 0x1F);
    cpu.retire_instruction(bus);
    Some(StepResult::InstructionCompleted)
}

pub fn op_eori_to_sr(cpu: &mut Cpu, bus: &mut MemoryBus) -> Option<StepResult> {
    if !cpu.state.is_supervisor() {
        let ret_pc = cpu.state.instruction_pc;
        push_standard_exception(
            &mut cpu.state,
            VECTOR_PRIVILEGE_VIOLATION,
            ret_pc,
            bus,
        );
        cpu.reload_pc_and_prefetch(cpu.state.pc, bus);
        return Some(StepResult::InstructionCompleted);
    }
    let imm = cpu.consume_extension_word(bus);
    let new_sr = cpu.state.sr ^ imm;
    cpu.state.set_sr(new_sr);
    cpu.retire_instruction(bus);
    Some(StepResult::InstructionCompleted)
}

