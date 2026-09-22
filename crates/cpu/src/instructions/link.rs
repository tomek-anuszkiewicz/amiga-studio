//! M68000 LINK Instruction (`LINK An, #d16`)
//!
//! Allocates a stack frame:
//! 1. Pushes current `An` onto stack at `SP - 4` (high word then low word).
//! 2. Sets `An = SP - 4`.
//! 3. Sets `SP = SP + d16` (sign-extended displacement).
//! Timing: 16 CPU clocks (8 CCK phases).
//!
//! Flags: Condition codes are completely unaffected.

use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Micro-Step ALU Callbacks: LINK
// ============================================================================

pub fn alu_link_setup(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let an = reg_dst;
    let sp = state.a_long(7).wrapping_sub(4);
    let val = if an == 7 {
        sp
    } else {
        state.a_long(an as usize)
    };
    let disp = (state.prefetch as i16) as i32 as u32;
    state.micro.source = disp;
    state.micro.destination = val;
    state.micro.ea_addr = sp;
}

pub fn alu_link_finish(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let an = reg_dst;
    let sp = state.micro.ea_addr;
    let disp = state.micro.source;
    state.set_a_long(an as usize, sp);
    state.set_a_long(7, sp.wrapping_add(disp));
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_LINK: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(alu_link_setup),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_HIGH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_LONG_LOW,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_link_finish),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];
