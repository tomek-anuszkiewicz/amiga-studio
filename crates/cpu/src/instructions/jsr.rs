//! JSR (Jump to Subroutine) instruction handler
//!
//! Pushes the long return address onto the stack and jumps to an effective address
//! using control addressing modes: `(An)`, `(d16, An)`, `(d8, An, Xn)`, `(xxx).w`, `(xxx).l`,
//! `(d16, PC)`, `(d8, PC, Xn)`.
//! Execution time: 16 to 22 CPU clocks depending on addressing mode.

use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

pub fn alu_jsr_ai(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    state.micro.ea_addr = state.a_long(reg_src as usize);
    let ret = state.pc.wrapping_sub(2);
    state.micro.destination = ret;
}

pub fn alu_jsr_d16_an(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let disp = (state.prefetch as i16) as i32;
    state.micro.ea_addr = state.a_long(reg_src as usize).wrapping_add(disp as u32);
    state.micro.destination = state.pc;
}

pub fn alu_jsr_idx_an(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    let ext = state.prefetch;
    let disp8 = (ext & 0xFF) as i8 as i32;
    let xn = crate::micro::ea::read_index_reg(state, ext);
    let an = state.a_long(reg_src as usize);
    state.micro.ea_addr = an.wrapping_add(xn).wrapping_add(disp8 as u32);
    state.micro.destination = state.pc;
}

pub fn alu_jsr_absw(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.ea_addr = state.prefetch as i16 as i32 as u32;
    state.micro.destination = state.pc;
}

pub fn alu_jsr_absl_lo(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.ea_addr = state.micro.ea_high | (state.prefetch as u32);
    state.micro.destination = state.pc;
}

pub fn alu_jsr_d16_pc(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let disp = (state.prefetch as i16) as i32;
    let base_pc = state.pc.wrapping_sub(2);
    state.micro.ea_addr = base_pc.wrapping_add(disp as u32);
    state.micro.destination = state.pc;
}

pub fn alu_jsr_idx_pc(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let ext = state.prefetch;
    let disp8 = (ext & 0xFF) as i8 as i32;
    let xn = crate::micro::ea::read_index_reg(state, ext);
    let base_pc = state.pc.wrapping_sub(2);
    state.micro.ea_addr = base_pc.wrapping_add(xn).wrapping_add(disp8 as u32);
    state.micro.destination = state.pc;
}

/// JSR (An): 16 CPU clocks / 8 CCKs (2 reads, 2 writes)
pub static STEPS_JSR_AI: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_target_opcode_read),
        alu_fn: Some(alu_jsr_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::PUSH_STACK_HIGH_IDLE,
    common::PUSH_STACK_HIGH_WRITE,
    common::BUS_WRITE_IDLE,
    common::PUSH_STACK_LOW_WRITE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// JSR (d16, An): 18 CPU clocks / 9 CCKs (2 reads, 2 writes)
pub static STEPS_JSR_D16_AN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_jsr_d16_an),
        base_clocks: 2,
    },
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PUSH_STACK_HIGH_IDLE,
    common::PUSH_STACK_HIGH_WRITE,
    common::BUS_WRITE_IDLE,
    common::PUSH_STACK_LOW_WRITE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// JSR (d8, An, Xn): 22 CPU clocks / 11 CCKs (2 reads, 2 writes)
pub static STEPS_JSR_IDX_AN: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_jsr_idx_an),
        base_clocks: 6,
    },
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PUSH_STACK_HIGH_IDLE,
    common::PUSH_STACK_HIGH_WRITE,
    common::BUS_WRITE_IDLE,
    common::PUSH_STACK_LOW_WRITE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// JSR (xxx).W: 18 CPU clocks / 9 CCKs (2 reads, 2 writes)
pub static STEPS_JSR_ABSW: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_jsr_absw),
        base_clocks: 2,
    },
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PUSH_STACK_HIGH_IDLE,
    common::PUSH_STACK_HIGH_WRITE,
    common::BUS_WRITE_IDLE,
    common::PUSH_STACK_LOW_WRITE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// JSR (xxx).L: 20 CPU clocks / 10 CCKs (3 reads, 2 writes)
pub static STEPS_JSR_ABSL: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(crate::micro::ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_target_opcode_read),
        alu_fn: Some(alu_jsr_absl_lo),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    common::PUSH_STACK_HIGH_IDLE,
    common::PUSH_STACK_HIGH_WRITE,
    common::BUS_WRITE_IDLE,
    common::PUSH_STACK_LOW_WRITE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// JSR (d16, PC): 18 CPU clocks / 9 CCKs (2 reads, 2 writes)
pub static STEPS_JSR_D16_PC: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_jsr_d16_pc),
        base_clocks: 2,
    },
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PUSH_STACK_HIGH_IDLE,
    common::PUSH_STACK_HIGH_WRITE,
    common::BUS_WRITE_IDLE,
    common::PUSH_STACK_LOW_WRITE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// JSR (d8, PC, Xn): 22 CPU clocks / 11 CCKs (2 reads, 2 writes)
pub static STEPS_JSR_IDX_PC: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(alu_jsr_idx_pc),
        base_clocks: 6,
    },
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PUSH_STACK_HIGH_IDLE,
    common::PUSH_STACK_HIGH_WRITE,
    common::BUS_WRITE_IDLE,
    common::PUSH_STACK_LOW_WRITE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// Compile-time opcode decoder for JSR ($4E90..=$4EBF)
pub const fn decode_jsr_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        2 => Some(&STEPS_JSR_AI),
        5 => Some(&STEPS_JSR_D16_AN),
        6 => Some(&STEPS_JSR_IDX_AN),
        7 => match reg {
            0 => Some(&STEPS_JSR_ABSW),
            1 => Some(&STEPS_JSR_ABSL),
            2 => Some(&STEPS_JSR_D16_PC),
            3 => Some(&STEPS_JSR_IDX_PC),
            _ => None,
        },
        _ => None,
    }
}
