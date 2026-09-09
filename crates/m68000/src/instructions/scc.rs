//! M68000 Scc Instruction (`Scc <ea>`)
//!
//! Set According to Condition:
//! - If Condition is TRUE: byte at destination is set to $FF.
//! - If Condition is FALSE: byte at destination is set to $00.
//!
//! Valid EA modes: Data alterable (Dn, (An), (An)+, -(An), (d16,An), (d8,An,Xn), (xxx).W, (xxx).L).
//! Timing:
//! - Dn: 4 CPU clocks if condition is false, 6 CPU clocks if condition is true.
//! - Memory: identical to CLR.B (dummy read before write):
//!   (An): 12 clocks, (An)+: 12 clocks, -(An): 14 clocks,
//!   (d16,An): 16 clocks, (d8,An,Xn): 18 clocks, (xxx).W: 16 clocks, (xxx).L: 20 clocks.
//!
//! Condition Codes: Strictly unaffected.

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub static STEPS_SCC_DN_FALSE: [MicroStep; 2] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

pub static STEPS_SCC_DN_TRUE: [MicroStep; 3] = [
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: None,
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::ALU_IDLE,
];

pub fn alu_scc_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let cond = ((state.ir >> 8) & 0x0F) as u8;
    let orig = state.d_long(reg_dst as usize);
    if state.eval_condition(cond) {
        state.set_d_long(reg_dst as usize, (orig & !0xFF) | 0xFF);
        state.micro.current_steps = &STEPS_SCC_DN_TRUE;
    } else {
        state.set_d_long(reg_dst as usize, orig & !0xFF);
        state.micro.current_steps = &STEPS_SCC_DN_FALSE;
    }
    state.micro.micro_step = 0;
}

pub fn alu_scc_mem(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let cond = ((state.ir >> 8) & 0x0F) as u8;
    let byte_val = if state.eval_condition(cond) {
        0xFF
    } else {
        0x00
    };
    state.micro.destination = (state.micro.destination & !0xFF) | byte_val;
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

pub static STEPS_SCC_DN: [MicroStep; 1] = [MicroStep::alu(alu_scc_dn)];

pub static STEPS_SCC_AI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_scc_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_SCC_PI: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pi_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_scc_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_SCC_PD: [MicroStep; 6] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_dst_byte),
        alu_fn: Some(ea::ea_calc_dst_pd_b),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_scc_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_SCC_D16: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_scc_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_SCC_IDX: [MicroStep; 9] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_scc_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_SCC_ABSW: [MicroStep; 8] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_scc_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

pub static STEPS_SCC_ABSL: [MicroStep; 10] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_BYTE,
    common::BUS_READ_IDLE,
    MicroStep {
        step_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_scc_mem),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_BYTE,
];

// ============================================================================
// Static Lookup Table & Decode Function
// ============================================================================

static SCC_LOOKUP: [&[MicroStep]; 8] = [
    &STEPS_SCC_DN,
    &STEPS_SCC_AI,
    &STEPS_SCC_PI,
    &STEPS_SCC_PD,
    &STEPS_SCC_D16,
    &STEPS_SCC_IDX,
    &STEPS_SCC_ABSW,
    &STEPS_SCC_ABSL,
];

pub const fn decode_scc_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(SCC_LOOKUP[0]),
        2 => Some(SCC_LOOKUP[1]),
        3 => Some(SCC_LOOKUP[2]),
        4 => Some(SCC_LOOKUP[3]),
        5 => Some(SCC_LOOKUP[4]),
        6 => Some(SCC_LOOKUP[5]),
        7 => match reg {
            0 => Some(SCC_LOOKUP[6]),
            1 => Some(SCC_LOOKUP[7]),
            _ => None,
        },
        _ => None,
    }
}
