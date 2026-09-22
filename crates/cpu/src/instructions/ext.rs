//! M68000 EXT Instruction (`EXT.W Dn` and `EXT.L Dn`)
//!
//! Sign-extends a data register:
//! - `EXT.W`: Extends bit 7 into bits 8..15 (lower byte to word). Bits 16..31 unaffected.
//! - `EXT.L`: Extends bit 15 into bits 16..31 (word to 32-bit long).
//!
//! Flags:
//! - N = MSB of result (bit 15 for EXT.W, bit 31 for EXT.L)
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

pub fn alu_ext_w(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let b = (state.d_long(reg_dst as usize) & 0xFF) as u8 as i8 as i16 as u16;
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (b as u32));
    state.set_ccr_nz_clear_vc((b as i16) < 0, b == 0);
}

pub fn alu_ext_l(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let w = (state.d_long(reg_dst as usize) & 0xFFFF) as u16 as i16 as i32 as u32;
    state.set_d_long(reg_dst as usize, w);
    state.set_ccr_nz_clear_vc((w as i32) < 0, w == 0);
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_EXT_W: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ext_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_EXT_L: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_ext_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];
