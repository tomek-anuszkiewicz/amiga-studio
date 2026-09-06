//! M68000 Integer Arithmetic (ADD, SUB, ADDQ, SUBQ) and cycle-exact CCR calculation

use crate::addressing::Size;
use crate::state::CpuState;

/// Performs integer addition with cycle-exact CCR flag evaluation (X, N, Z, V, C)
pub fn execute_add(state: &mut CpuState, src: u32, dst: u32, size: Size, update_ccr: bool) -> u32 {
    match size {
        Size::Byte => {
            let s = (src & 0xFF) as u8;
            let d = (dst & 0xFF) as u8;
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
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let s = (src & 0xFFFF) as u16;
            let d = (dst & 0xFFFF) as u16;
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
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => {
            let (res, c) = dst.overflowing_add(src);
            if update_ccr {
                let v = ((!(src ^ dst) & (dst ^ res)) & 0x8000_0000) != 0;
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
    }
}

/// Performs integer subtraction (Dst - Src) with cycle-exact CCR flag evaluation (X, N, Z, V, C)
pub fn execute_sub(state: &mut CpuState, src: u32, dst: u32, size: Size, update_ccr: bool) -> u32 {
    match size {
        Size::Byte => {
            let s = (src & 0xFF) as u8;
            let d = (dst & 0xFF) as u8;
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
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let s = (src & 0xFFFF) as u16;
            let d = (dst & 0xFFFF) as u16;
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
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => {
            let (res, c) = dst.overflowing_sub(src);
            if update_ccr {
                let v = (((src ^ dst) & (dst ^ res)) & 0x8000_0000) != 0;
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
    }
}
