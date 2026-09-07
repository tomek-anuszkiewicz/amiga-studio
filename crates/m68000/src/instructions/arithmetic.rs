//! M68000 Integer Arithmetic (ADD, SUB, ADDQ, SUBQ) and cycle-exact CCR calculation
//!
//! Provides both generic size handlers and branchless size-specialized leaf functions (add_b, add_w, add_l).

use crate::addressing::Size;
use crate::state::CpuState;

// --- Direct Size-Specialized Addition Functions (Branchless) ---

#[inline(always)]
pub fn add_b(state: &mut CpuState, s: u8, d: u8, update_ccr: bool) -> u8 {
    let (res, c) = d.overflowing_add(s);
    if update_ccr {
        let v = ((!(s ^ d) & (d ^ res)) & 0x80) != 0;
        let n = (res & 0x80) != 0;
        let z = res == 0;
        state.set_x(c);
        state.set_c(c);
        state.set_v(v);
        state.set_n(n);
        state.set_z(z);
    }
    res
}

#[inline(always)]
pub fn add_w(state: &mut CpuState, s: u16, d: u16, update_ccr: bool) -> u16 {
    let (res, c) = d.overflowing_add(s);
    if update_ccr {
        let v = ((!(s ^ d) & (d ^ res)) & 0x8000) != 0;
        let n = (res & 0x8000) != 0;
        let z = res == 0;
        state.set_x(c);
        state.set_c(c);
        state.set_v(v);
        state.set_n(n);
        state.set_z(z);
    }
    res
}

#[inline(always)]
pub fn add_l(state: &mut CpuState, s: u32, d: u32, update_ccr: bool) -> u32 {
    let (res, c) = d.overflowing_add(s);
    if update_ccr {
        let v = ((!(s ^ d) & (d ^ res)) & 0x8000_0000) != 0;
        let n = (res & 0x8000_0000) != 0;
        let z = res == 0;
        state.set_x(c);
        state.set_c(c);
        state.set_v(v);
        state.set_n(n);
        state.set_z(z);
    }
    res
}

// --- Direct Size-Specialized Subtraction Functions (Branchless) ---

#[inline(always)]
pub fn sub_b(state: &mut CpuState, s: u8, d: u8, update_ccr: bool) -> u8 {
    let (res, c) = d.overflowing_sub(s);
    if update_ccr {
        let v = (((s ^ d) & (d ^ res)) & 0x80) != 0;
        let n = (res & 0x80) != 0;
        let z = res == 0;
        state.set_x(c);
        state.set_c(c);
        state.set_v(v);
        state.set_n(n);
        state.set_z(z);
    }
    res
}

#[inline(always)]
pub fn sub_w(state: &mut CpuState, s: u16, d: u16, update_ccr: bool) -> u16 {
    let (res, c) = d.overflowing_sub(s);
    if update_ccr {
        let v = (((s ^ d) & (d ^ res)) & 0x8000) != 0;
        let n = (res & 0x8000) != 0;
        let z = res == 0;
        state.set_x(c);
        state.set_c(c);
        state.set_v(v);
        state.set_n(n);
        state.set_z(z);
    }
    res
}

#[inline(always)]
pub fn sub_l(state: &mut CpuState, s: u32, d: u32, update_ccr: bool) -> u32 {
    let (res, c) = d.overflowing_sub(s);
    if update_ccr {
        let v = (((s ^ d) & (d ^ res)) & 0x8000_0000) != 0;
        let n = (res & 0x8000_0000) != 0;
        let z = res == 0;
        state.set_x(c);
        state.set_c(c);
        state.set_v(v);
        state.set_n(n);
        state.set_z(z);
    }
    res
}

// --- Sized Generic Wrappers ---

pub fn execute_add(state: &mut CpuState, src: u32, dst: u32, size: Size, update_ccr: bool) -> u32 {
    match size {
        Size::Byte => {
            let res = add_b(state, (src & 0xFF) as u8, (dst & 0xFF) as u8, update_ccr);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let res = add_w(
                state,
                (src & 0xFFFF) as u16,
                (dst & 0xFFFF) as u16,
                update_ccr,
            );
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => add_l(state, src, dst, update_ccr),
    }
}

pub fn execute_sub(state: &mut CpuState, src: u32, dst: u32, size: Size, update_ccr: bool) -> u32 {
    match size {
        Size::Byte => {
            let res = sub_b(state, (src & 0xFF) as u8, (dst & 0xFF) as u8, update_ccr);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let res = sub_w(
                state,
                (src & 0xFFFF) as u16,
                (dst & 0xFFFF) as u16,
                update_ccr,
            );
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => sub_l(state, src, dst, update_ccr),
    }
}

pub fn execute_addx(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    let x = if state.get_x() { 1 } else { 0 };
    match size {
        Size::Byte => {
            let s = (src & 0xFF) as u8;
            let d = (dst & 0xFF) as u8;
            let (res1, c1) = d.overflowing_add(s);
            let (res, c2) = res1.overflowing_add(x as u8);
            let c = c1 || c2;
            let v = ((!(s ^ d) & (d ^ res)) & 0x80) != 0;
            let n = (res & 0x80) != 0;
            if res != 0 {
                state.set_z(false);
            }
            state.set_x(c);
            state.set_c(c);
            state.set_v(v);
            state.set_n(n);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let s = (src & 0xFFFF) as u16;
            let d = (dst & 0xFFFF) as u16;
            let (res1, c1) = d.overflowing_add(s);
            let (res, c2) = res1.overflowing_add(x as u16);
            let c = c1 || c2;
            let v = ((!(s ^ d) & (d ^ res)) & 0x8000) != 0;
            let n = (res & 0x8000) != 0;
            if res != 0 {
                state.set_z(false);
            }
            state.set_x(c);
            state.set_c(c);
            state.set_v(v);
            state.set_n(n);
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => {
            let s = src;
            let d = dst;
            let (res1, c1) = d.overflowing_add(s);
            let (res, c2) = res1.overflowing_add(x);
            let c = c1 || c2;
            let v = ((!(s ^ d) & (d ^ res)) & 0x8000_0000) != 0;
            let n = (res & 0x8000_0000) != 0;
            if res != 0 {
                state.set_z(false);
            }
            state.set_x(c);
            state.set_c(c);
            state.set_v(v);
            state.set_n(n);
            res
        }
    }
}

pub fn execute_subx(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    let x = if state.get_x() { 1 } else { 0 };
    match size {
        Size::Byte => {
            let s = (src & 0xFF) as u8;
            let d = (dst & 0xFF) as u8;
            let (res1, c1) = d.overflowing_sub(s);
            let (res, c2) = res1.overflowing_sub(x as u8);
            let c = c1 || c2;
            let v = (((s ^ d) & (d ^ res)) & 0x80) != 0;
            let n = (res & 0x80) != 0;
            if res != 0 {
                state.set_z(false);
            }
            state.set_x(c);
            state.set_c(c);
            state.set_v(v);
            state.set_n(n);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let s = (src & 0xFFFF) as u16;
            let d = (dst & 0xFFFF) as u16;
            let (res1, c1) = d.overflowing_sub(s);
            let (res, c2) = res1.overflowing_sub(x as u16);
            let c = c1 || c2;
            let v = (((s ^ d) & (d ^ res)) & 0x8000) != 0;
            let n = (res & 0x8000) != 0;
            if res != 0 {
                state.set_z(false);
            }
            state.set_x(c);
            state.set_c(c);
            state.set_v(v);
            state.set_n(n);
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => {
            let s = src;
            let d = dst;
            let (res1, c1) = d.overflowing_sub(s);
            let (res, c2) = res1.overflowing_sub(x);
            let c = c1 || c2;
            let v = (((s ^ d) & (d ^ res)) & 0x8000_0000) != 0;
            let n = (res & 0x8000_0000) != 0;
            if res != 0 {
                state.set_z(false);
            }
            state.set_x(c);
            state.set_c(c);
            state.set_v(v);
            state.set_n(n);
            res
        }
    }
}

/// Executes CMP: evaluates (dst - src) and updates N, Z, V, C flags.
/// X flag is NOT affected.
#[inline]
pub fn execute_cmp(state: &mut CpuState, src: u32, dst: u32, size: Size) {
    match size {
        Size::Byte => {
            let s = (src & 0xFF) as u8;
            let d = (dst & 0xFF) as u8;
            let (res, c) = d.overflowing_sub(s);
            let v = (((s ^ d) & (d ^ res)) & 0x80) != 0;
            let n = (res & 0x80) != 0;
            let z = res == 0;
            state.set_c(c);
            state.set_v(v);
            state.set_n(n);
            state.set_z(z);
        }
        Size::Word => {
            let s = (src & 0xFFFF) as u16;
            let d = (dst & 0xFFFF) as u16;
            let (res, c) = d.overflowing_sub(s);
            let v = (((s ^ d) & (d ^ res)) & 0x8000) != 0;
            let n = (res & 0x8000) != 0;
            let z = res == 0;
            state.set_c(c);
            state.set_v(v);
            state.set_n(n);
            state.set_z(z);
        }
        Size::Long => {
            let (res, c) = dst.overflowing_sub(src);
            let v = (((src ^ dst) & (dst ^ res)) & 0x8000_0000) != 0;
            let n = (res & 0x8000_0000) != 0;
            let z = res == 0;
            state.set_c(c);
            state.set_v(v);
            state.set_n(n);
            state.set_z(z);
        }
    }
}

/// Executes TST: evaluates val against zero, updating N and Z flags, clearing V and C.
/// X flag is NOT affected.
#[inline]
pub fn execute_tst(state: &mut CpuState, val: u32, size: Size) {
    let (n, z) = match size {
        Size::Byte => {
            let b = (val & 0xFF) as u8;
            ((b & 0x80) != 0, b == 0)
        }
        Size::Word => {
            let w = (val & 0xFFFF) as u16;
            ((w & 0x8000) != 0, w == 0)
        }
        Size::Long => ((val & 0x8000_0000) != 0, val == 0),
    };
    state.set_n(n);
    state.set_z(z);
    state.set_v(false);
    state.set_c(false);
}
