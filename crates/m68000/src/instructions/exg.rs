//! M68000 EXG Instruction (`EXG Rx, Ry`)
//!
//! Exchanges the contents of two 32-bit registers:
//! - `EXG Dx, Dy` (opmode 0b01000): Data register with data register.
//! - `EXG Ax, Ay` (opmode 0b01001): Address register with address register.
//! - `EXG Dx, Ay` (opmode 0b10001): Data register with address register.
//!
//! Flags: Condition codes are completely unaffected.
//!
//! Timing: 6 CPU clocks (3 CCK phases).

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_exg_dx_dy(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let x = state.d_long(reg_dst as usize);
    let y = state.d_long(reg_src as usize);
    state.set_d_long(reg_dst as usize, y);
    state.set_d_long(reg_src as usize, x);
}

pub fn alu_exg_ax_ay(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let x = state.a_long(reg_dst as usize);
    let y = state.a_long(reg_src as usize);
    state.set_a_long(reg_dst as usize, y);
    state.set_a_long(reg_src as usize, x);
}

pub fn alu_exg_dx_ay(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let x = state.d_long(reg_dst as usize);
    let y = state.a_long(reg_src as usize);
    state.set_d_long(reg_dst as usize, y);
    state.set_a_long(reg_src as usize, x);
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_EXG_DX_DY: [MicroStep; 3] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 2,
    },
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_exg_dx_dy),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_EXG_AX_AY: [MicroStep; 3] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 2,
    },
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_exg_ax_ay),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_EXG_DX_AY: [MicroStep; 3] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 2,
    },
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_exg_dx_ay),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];
