//! M68000 Control Flow Operations (BRA, Bcc, JMP, JSR, RTS)

use crate::state::CpuState;

/// Evaluates Bcc branch condition and returns the target PC if taken
pub fn evaluate_bcc(state: &CpuState, cond: u8, base_pc: u32, displacement: i32) -> Option<u32> {
    if state.eval_condition(cond) {
        let target = base_pc.wrapping_add(displacement as u32) & 0x00FF_FFFF;
        Some(target)
    } else {
        None
    }
}
