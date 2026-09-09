//! CMPM (Compare Memory) Instruction Handlers
//!
//! Compares memory operands via postincrement: `CMPM (Ay)+, (Ax)+`.
//! Evaluates ((Ax) - (Ay)) and updates N, Z, V, and C flags.
//! Neither memory location is modified. Extend (X) flag is unaffected.

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Core Leaf ALU Compare Functions (Branchless & Cycle-Exact CCR)
// ============================================================================

#[inline(always)]
pub fn cmp_b(state: &mut CpuState, s: u8, d: u8) {
    let (res, c) = d.overflowing_sub(s);
    let v = (((s ^ d) & (d ^ res)) & 0x80) != 0;
    let n = (res & 0x80) != 0;
    let z = res == 0;
    state.set_ccr_nzvc(n, z, v, c);
}

#[inline(always)]
pub fn cmp_w(state: &mut CpuState, s: u16, d: u16) {
    let (res, c) = d.overflowing_sub(s);
    let v = (((s ^ d) & (d ^ res)) & 0x8000) != 0;
    let n = (res & 0x8000) != 0;
    let z = res == 0;
    state.set_ccr_nzvc(n, z, v, c);
}

#[inline(always)]
pub fn cmp_l(state: &mut CpuState, s: u32, d: u32) {
    let (res, c) = d.overflowing_sub(s);
    let v = (((s ^ d) & (d ^ res)) & 0x8000_0000) != 0;
    let n = (res & 0x8000_0000) != 0;
    let z = res == 0;
    state.set_ccr_nzvc(n, z, v, c);
}

// ============================================================================
// Micro-Step Callbacks
// ============================================================================

/// ALU compare callback for Byte
pub fn alu_cmpm_b(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.source & 0xFF) as u8;
    let d = (state.micro.destination & 0xFF) as u8;
    cmp_b(state, s, d);
}

/// ALU compare callback for Word
pub fn alu_cmpm_w(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = (state.micro.source & 0xFFFF) as u16;
    let d = (state.micro.destination & 0xFFFF) as u16;
    cmp_w(state, s, d);
}

/// ALU compare callback for Long
pub fn alu_cmpm_l(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let s = state.micro.source;
    let d = state.micro.destination;
    cmp_l(state, s, d);
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_CMPM_B: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_addr1_byte),
        alu_fn: Some(ea::ea_calc_dual_pi_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_ADDR2_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpm_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPM_W: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_addr1_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_addr2_word),
        alu_fn: Some(ea::ea_calc_dst_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpm_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_CMPM_L: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_addr1_long_high),
        alu_fn: Some(ea::ea_calc_src_pi_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_ADDR1_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_addr2_long_high),
        alu_fn: Some(ea::ea_calc_dst_pi_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_ADDR2_LONG_LOW,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_cmpm_l),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

/// Decodes the micro-step sequence for CMPM based on size (0 = Byte, 1 = Word, 2 = Long)
pub const fn decode_cmpm_steps(size: u8) -> Option<&'static [MicroStep]> {
    match size {
        0 => Some(&STEPS_CMPM_B),
        1 => Some(&STEPS_CMPM_W),
        2 => Some(&STEPS_CMPM_L),
        _ => None,
    }
}
