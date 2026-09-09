//! MOVEQ (Move Quick) instruction handler
//!
//! Moves an 8-bit sign-extended immediate value into a data register.
//! Execution time: 4 CPU clocks (2 CCKs / 1 prefetch cycle).

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

#[inline(always)]
pub fn alu_moveq(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let d8 = (state.ir & 0xFF) as i8 as i32 as u32;
    state.set_d_long(reg_dst as usize, d8);
    let n = (d8 as i32) < 0;
    let z = d8 == 0;
    state.set_ccr_nz_clear_vc(n, z);
}

/// MOVEQ #<data>, Dn (4 CPU clocks / 2 CCKs)
pub static STEPS_MOVEQ: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_moveq),
        base_clocks: 2,
    },
    common::PREFETCH_NEXT_RETIRE,
];

/// Compile-time opcode decoder for MOVEQ ($7000..=$7FFF with bit 8 == 0)
pub const fn decode_moveq_steps(ir: u16) -> Option<&'static [MicroStep]> {
    if (ir & 0x0100) == 0 {
        Some(&STEPS_MOVEQ)
    } else {
        None
    }
}
