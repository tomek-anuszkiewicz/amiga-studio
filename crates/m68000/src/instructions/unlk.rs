//! M68000 UNLK Instruction (`UNLK An`)
//!
//! Deallocates a stack frame:
//! 1. Sets `SP = An`.
//! 2. Pops 32-bit long from `(SP)` into `An` (high word then low word).
//! 3. Increments `SP += 4`.
//! Timing: 12 CPU clocks (6 CCK phases).
//!
//! Flags: Condition codes are completely unaffected.

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Micro-Step ALU Callbacks: UNLK
// ============================================================================

pub fn alu_unlk_setup(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let an = reg_dst;
    let sp = state.a_long(an as usize);
    state.micro.ea_addr = sp;
    if (sp & 1) == 0 {
        state.set_a_long(7, sp);
    }
}

pub fn alu_unlk_finish(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let an = reg_dst;
    let sp = state.micro.ea_addr;
    let data = state.micro.source;
    state.set_a_long(7, sp.wrapping_add(4));
    state.set_a_long(an as usize, data);
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_UNLK: [MicroStep; 5] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_long_high),
        alu_fn: Some(alu_unlk_setup),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::READ_SRC_LONG_LOW,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_unlk_finish),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];
