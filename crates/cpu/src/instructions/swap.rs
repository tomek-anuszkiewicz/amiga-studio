//! M68000 SWAP Instruction (`SWAP Dn`)
//!
//! Exchanges the 16-bit halves of a data register:
//! `bits 31..16 <-> bits 15..0`.
//!
//! Flags:
//! - N = MSB of 32-bit result (bit 31)
//! - Z = result == 0
//! - V = 0
//! - C = 0
//! - X = unaffected
//!
//! Timing: 4 CPU clocks (2 CCK phases).

use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_swap(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let val = state.d_long(reg_dst as usize);
    let swapped = (val >> 16) | (val << 16);
    state.set_d_long(reg_dst as usize, swapped);
    state.set_ccr_nz_clear_vc((swapped as i32) < 0, swapped == 0);
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_SWAP: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_swap),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];
