//! M68000 Logical Operations (AND, OR, EOR)

use crate::addressing::Size;
use crate::state::CpuState;

pub fn execute_and(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    let res = dst & src;
    update_ccr_logic(state, res, size);
    match size {
        Size::Byte => (dst & !0xFF) | (res & 0xFF),
        Size::Word => (dst & !0xFFFF) | (res & 0xFFFF),
        Size::Long => res,
    }
}

pub fn execute_or(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    let res = dst | src;
    update_ccr_logic(state, res, size);
    match size {
        Size::Byte => (dst & !0xFF) | (res & 0xFF),
        Size::Word => (dst & !0xFFFF) | (res & 0xFFFF),
        Size::Long => res,
    }
}

pub fn execute_eor(state: &mut CpuState, src: u32, dst: u32, size: Size) -> u32 {
    let res = dst ^ src;
    update_ccr_logic(state, res, size);
    match size {
        Size::Byte => (dst & !0xFF) | (res & 0xFF),
        Size::Word => (dst & !0xFFFF) | (res & 0xFFFF),
        Size::Long => res,
    }
}

#[inline]
fn update_ccr_logic(state: &mut CpuState, res: u32, size: Size) {
    let (n, z) = match size {
        Size::Byte => (((res as u8) & 0x80) != 0, (res as u8) == 0),
        Size::Word => (((res as u16) & 0x8000) != 0, (res as u16) == 0),
        Size::Long => ((res & 0x8000_0000) != 0, res == 0),
    };
    state.set_n(n);
    state.set_z(z);
    state.set_v(false);
    state.set_c(false);
    // X is unaffected by logical operations
}
