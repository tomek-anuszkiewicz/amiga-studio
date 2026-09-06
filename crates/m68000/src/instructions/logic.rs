//! M68000 Logical Operations (AND, OR, EOR, NOT)
//!
//! Provides direct branchless size-specialized functions (and_b, and_w, and_l, etc.)
//! and notes on endianness bypass (bitwise operations commute with byte reversal).

use crate::addressing::Size;
use crate::state::CpuState;

// --- Direct Size-Specialized AND (Branchless & Endian-Neutral) ---

#[inline(always)]
pub fn and_b(state: &mut CpuState, s: u8, d: u8) -> u8 {
    let res = d & s;
    state.set_n((res & 0x80) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

#[inline(always)]
pub fn and_w(state: &mut CpuState, s: u16, d: u16) -> u16 {
    let res = d & s;
    state.set_n((res & 0x8000) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

#[inline(always)]
pub fn and_l(state: &mut CpuState, s: u32, d: u32) -> u32 {
    let res = d & s;
    state.set_n((res & 0x8000_0000) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

// --- Direct Size-Specialized OR (Branchless & Endian-Neutral) ---

#[inline(always)]
pub fn or_b(state: &mut CpuState, s: u8, d: u8) -> u8 {
    let res = d | s;
    state.set_n((res & 0x80) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

#[inline(always)]
pub fn or_w(state: &mut CpuState, s: u16, d: u16) -> u16 {
    let res = d | s;
    state.set_n((res & 0x8000) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

#[inline(always)]
pub fn or_l(state: &mut CpuState, s: u32, d: u32) -> u32 {
    let res = d | s;
    state.set_n((res & 0x8000_0000) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

// --- Direct Size-Specialized EOR (Branchless & Endian-Neutral) ---

#[inline(always)]
pub fn eor_b(state: &mut CpuState, s: u8, d: u8) -> u8 {
    let res = d ^ s;
    state.set_n((res & 0x80) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

#[inline(always)]
pub fn eor_w(state: &mut CpuState, s: u16, d: u16) -> u16 {
    let res = d ^ s;
    state.set_n((res & 0x8000) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

#[inline(always)]
pub fn eor_l(state: &mut CpuState, s: u32, d: u32) -> u32 {
    let res = d ^ s;
    state.set_n((res & 0x8000_0000) != 0);
    state.set_z(res == 0);
    state.set_v(false);
    state.set_c(false);
    res
}

// --- Sized Generic Wrappers ---

pub fn execute_and(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    match size {
        Size::Byte => {
            let res = and_b(state, (src & 0xFF) as u8, (dst & 0xFF) as u8);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let res = and_w(state, (src & 0xFFFF) as u16, (dst & 0xFFFF) as u16);
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => and_l(state, src, dst),
    }
}

pub fn execute_or(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    match size {
        Size::Byte => {
            let res = or_b(state, (src & 0xFF) as u8, (dst & 0xFF) as u8);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let res = or_w(state, (src & 0xFFFF) as u16, (dst & 0xFFFF) as u16);
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => or_l(state, src, dst),
    }
}

pub fn execute_eor(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    match size {
        Size::Byte => {
            let res = eor_b(state, (src & 0xFF) as u8, (dst & 0xFF) as u8);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let res = eor_w(state, (src & 0xFFFF) as u16, (dst & 0xFFFF) as u16);
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => eor_l(state, src, dst),
    }
}
