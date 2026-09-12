//! M68000 LEA Instruction (`LEA <ea>, An`)
//!
//! Calculates the effective address using control addressing modes and loads it into An:
//! `(An)`, `(d16, An)`, `(d8, An, Xn)`, `(xxx).w`, `(xxx).l`, `(d16, PC)`, `(d8, PC, Xn)`.
//!
//! Note: Memory at the calculated address is never accessed; odd addresses do not trigger
//! an Address Error exception.
//!
//! Flags: Condition codes are completely unaffected.
//!
//! Execution time: 4 to 12 CPU clocks depending on addressing mode.

use crate::core::Cpu;
use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

pub fn alu_lea_ai(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let ea = state.read_a(reg_src as usize);
    state.set_a_long(reg_dst as usize, ea);
}

pub fn alu_lea_finish(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let ea = state.micro.destination;
    state.set_a_long(reg_dst as usize, ea);
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

/// LEA (An): 4 CPU clocks / 2 CCKs
pub static STEPS_LEA_AI: [MicroStep; 2] = [
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_lea_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

/// LEA (d16, An): 8 CPU clocks / 4 CCKs
pub static STEPS_LEA_D16_AN: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_pea_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_lea_finish),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

/// LEA (d8, An, Xn): 12 CPU clocks / 6 CCKs
pub static STEPS_LEA_IDX_AN: [MicroStep; 5] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_pea_idx_an),
        base_clocks: 4,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_lea_finish),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

/// LEA (xxx).W: 8 CPU clocks / 4 CCKs
pub static STEPS_LEA_ABSW: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_pea_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_lea_finish),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

/// LEA (xxx).L: 12 CPU clocks / 6 CCKs
pub static STEPS_LEA_ABSL: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_pea_absl_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_lea_finish),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

/// LEA (d16, PC): 8 CPU clocks / 4 CCKs
pub static STEPS_LEA_D16_PC: [MicroStep; 4] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_pea_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_lea_finish),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

/// LEA (d8, PC, Xn): 12 CPU clocks / 6 CCKs
pub static STEPS_LEA_IDX_PC: [MicroStep; 5] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_pea_idx_pc),
        base_clocks: 4,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_lea_finish),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
];

/// Compile-time opcode decoder for LEA ($41C0..=$4FC0)
pub const fn decode_lea_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        2 => Some(&STEPS_LEA_AI),
        5 => Some(&STEPS_LEA_D16_AN),
        6 => Some(&STEPS_LEA_IDX_AN),
        7 => match reg {
            0 => Some(&STEPS_LEA_ABSW),
            1 => Some(&STEPS_LEA_ABSL),
            2 => Some(&STEPS_LEA_D16_PC),
            3 => Some(&STEPS_LEA_IDX_PC),
            _ => None,
        },
        _ => None,
    }
}
