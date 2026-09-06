//! MOVE and MOVEA instruction implementations

use crate::addressing::Size;
use crate::state::CpuState;

/// Evaluates CCR updates for MOVE instruction (N and Z updated, V and C cleared, X unchanged)
pub fn update_ccr_move(state: &mut CpuState, val: u32, size: Size) {
    let (is_negative, is_zero) = match size {
        Size::Byte => ((val as u8 & 0x80) != 0, (val as u8) == 0),
        Size::Word => ((val as u16 & 0x8000) != 0, (val as u16) == 0),
        Size::Long => ((val & 0x8000_0000) != 0, val == 0),
    };
    state.set_n(is_negative);
    state.set_z(is_zero);
    state.set_v(false);
    state.set_c(false);
}

/// Sign-extends a word to 32 bits for MOVEA.W
#[inline]
pub fn sign_extend_word(val: u16) -> u32 {
    (val as i16 as i32) as u32
}
