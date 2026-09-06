//! M68000 System and Exception Processing (NOP, TRAP, Address Error)

use crate::state::{CpuState, SR_S, SR_T};

pub const VECTOR_RESET_SSP: u32 = 0x000000;
pub const VECTOR_RESET_PC: u32 = 0x000004;
pub const VECTOR_BUS_ERROR: u32 = 0x000008;
pub const VECTOR_ADDRESS_ERROR: u32 = 0x00000C;
pub const VECTOR_ILLEGAL_INSTRUCTION: u32 = 0x000010;
pub const VECTOR_ZERO_DIVIDE: u32 = 0x000014;
pub const VECTOR_CHK: u32 = 0x000018;
pub const VECTOR_TRAPV: u32 = 0x00001C;
pub const VECTOR_PRIVILEGE_VIOLATION: u32 = 0x000020;
pub const VECTOR_TRACE: u32 = 0x000024;
pub const VECTOR_LINE_A: u32 = 0x000028;
pub const VECTOR_LINE_F: u32 = 0x00002C;
pub const VECTOR_TRAP_BASE: u32 = 0x000080;

/// Initiates standard 3-word exception processing (TRAP, Interrupts, etc.)
pub fn push_standard_exception(
    state: &mut CpuState,
    vector_addr: u32,
    return_pc: u32,
    mut write_word: impl FnMut(u32, u16),
    mut read_long: impl FnMut(u32) -> u32,
) {
    let old_sr = state.sr;
    // Switch to supervisor mode, clear trace
    state.sr |= SR_S;
    state.sr &= !SR_T;

    // Push PC (high word, low word)
    let sp = state.ssp.wrapping_sub(4);
    state.ssp = sp;
    write_word(sp, (return_pc >> 16) as u16);
    write_word(sp.wrapping_add(2), (return_pc & 0xFFFF) as u16);

    // Push SR
    let sp = state.ssp.wrapping_sub(2);
    state.ssp = sp;
    write_word(sp, old_sr);

    // Load new PC from vector
    state.pc = read_long(vector_addr) & 0x00FF_FFFF;
}

/// Initiates MC68000 Group 0/1 Address Error (Vector 3) 7-word exception processing
pub fn push_address_error_exception(
    state: &mut CpuState,
    fault_addr: u32,
    is_read: bool,
    function_code: u8,
    mut write_word: impl FnMut(u32, u16),
    mut read_long: impl FnMut(u32) -> u32,
) {
    let old_sr = state.sr;
    let old_pc = state.pc;
    let ir = state.ir;

    // Switch to supervisor mode, clear trace
    state.sr |= SR_S;
    state.sr &= !SR_T;

    // Build Internal Information Word:
    // Bit 4: R/W (1 = Read, 0 = Write)
    // Bit 3: I/N (1 = Instruction processing, 0 = Exception)
    // Bits 2-0: Function Code (FC0-FC2)
    let rw_bit = if is_read { 0x10 } else { 0x00 };
    let in_bit = 0x08;
    let info_word = rw_bit | in_bit | ((function_code as u16) & 0x07);

    // Push 7-word stack frame in reverse order (bottom to top):
    // SP + 12: Low 16 bits of PC
    // SP + 10: High 16 bits of PC
    // SP + 08: SR
    // SP + 06: IR
    // SP + 04: Low 16 bits of Access Address
    // SP + 02: High 16 bits of Access Address
    // SP + 00: Internal Information Word
    let sp = state.ssp.wrapping_sub(14);
    state.ssp = sp;

    write_word(sp, info_word);
    write_word(sp.wrapping_add(2), (fault_addr >> 16) as u16);
    write_word(sp.wrapping_add(4), (fault_addr & 0xFFFF) as u16);
    write_word(sp.wrapping_add(6), ir);
    write_word(sp.wrapping_add(8), old_sr);
    write_word(sp.wrapping_add(10), (old_pc >> 16) as u16);
    write_word(sp.wrapping_add(12), (old_pc & 0xFFFF) as u16);

    // Vector 3 ($00000C)
    state.pc = read_long(VECTOR_ADDRESS_ERROR) & 0x00FF_FFFF;
}
