//! M68000 CHK Instruction (`CHK <ea>, Dn`)
//!
//! Checks data register against upper bound:
//! - If `(Dn.w as i16) < 0` or `(Dn.w as i16) > (<ea>.w as i16)`:
//!   Triggers CHK trap (Vector 6, vector address `$0018`).
//!
//! Flags:
//! - N = `(Dn.w as i16) < 0`
//! - Z = 0
//! - V = 0
//! - C = 0
//! - X = unaffected
//!
//! Timing:
//! - If no trap: 10 clocks for Dn, 14 for (An)/(An)+/#imm, 16 for -(An), 18 for d16/abs.w, 22 for idx/abs.l.
//! - If trap taken: adds 8 internal idle clocks + 30-clock exception sequence.

use crate::core::Cpu;
use crate::micro::common::{
    ALU_IDLE, BUS_READ_IDLE, EXCEPTION_PUSH_PCHI_IDLE, EXCEPTION_PUSH_PCHI_WRITE,
    EXCEPTION_PUSH_PCLO_IDLE, EXCEPTION_PUSH_PCLO_WRITE, EXCEPTION_PUSH_SR_IDLE,
    EXCEPTION_PUSH_SR_WRITE, FETCH_EXT_FINISH, FETCH_EXT_READ, PREFETCH_IRC_FINISH,
    PREFETCH_IRC_READ, PREFETCH_TARGET_FINISH, PREFETCH_TARGET_READ, READ_SRC_WORD,
    READ_TARGET_OPCODE_READ, READ_VECTOR_HIGH_READ, READ_VECTOR_LOW_FINISH, READ_VECTOR_LOW_READ,
};
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

// ============================================================================
// CHK Exception Processing Pipeline (Vector 6, Address $0018)
// ============================================================================

pub static STEPS_CHK_TRAP: [MicroStep; 16] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 8,
    },
    EXCEPTION_PUSH_PCLO_IDLE,
    EXCEPTION_PUSH_PCLO_WRITE,
    EXCEPTION_PUSH_SR_IDLE,
    EXCEPTION_PUSH_SR_WRITE,
    EXCEPTION_PUSH_PCHI_IDLE,
    EXCEPTION_PUSH_PCHI_WRITE,
    READ_VECTOR_HIGH_READ,
    BUS_READ_IDLE,
    READ_VECTOR_LOW_READ,
    READ_VECTOR_LOW_FINISH,
    READ_TARGET_OPCODE_READ,
    BUS_READ_IDLE,
    ALU_IDLE,
    PREFETCH_TARGET_READ,
    PREFETCH_TARGET_FINISH,
];

#[inline(never)]
pub fn trigger_chk_trap(state: &mut CpuState) {
    let old_sr = state.sr;
    state.set_supervisor(true);
    state.sr &= !0x8000;

    let return_pc = state.pc.wrapping_sub(2);
    state.micro.source = return_pc;
    state.micro.destination = old_sr as u32;
    state.micro.ea_addr = 0x0000_0018; // Vector 6 (offset 24)
    state.micro.current_steps = &STEPS_CHK_TRAP;
    state.micro.micro_step = 0;
    state.micro.clocks_remaining = 0;
}

// ============================================================================
// Micro-Step ALU Callbacks
// ============================================================================

#[inline(always)]
fn execute_chk(state: &mut CpuState, bound: i16, reg_dst: u8) {
    let val = state.d_word(reg_dst as usize) as i16;
    if val < 0 {
        state.set_ccr_nz_clear_vc(true, false);
        trigger_chk_trap(state);
    } else if val > bound {
        state.set_ccr_nz_clear_vc(false, false);
        trigger_chk_trap(state);
    } else {
        // No trap: Motorola PRM defines N as undefined. On 68000 silicon,
        // N preserves its prior value, while Z, V, and C are cleared.
        let prior_n = state.get_n();
        state.set_ccr_nz_clear_vc(prior_n, false);
    }
}

pub fn alu_chk_dn(state: &mut CpuState, reg_src: u8, reg_dst: u8) {
    let bound = state.d_word(reg_src as usize) as i16;
    execute_chk(state, bound, reg_dst);
}

pub fn alu_chk_mem(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let bound = state.micro.source as u16 as i16;
    execute_chk(state, bound, reg_dst);
}

pub fn latch_imm_chk(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.source = state.prefetch[0] as u32;
}

// ============================================================================
// Static Micro-Step Slices
// ============================================================================

/// CHK Dn, Dm: 10 CPU clocks / 5 CCKs (or 38 clocks if trap)
pub static STEPS_CHK_DN: [MicroStep; 3] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(alu_chk_dn),
        base_clocks: 6,
    },
    PREFETCH_IRC_READ,
    PREFETCH_IRC_FINISH,
];

/// CHK (An), Dn: 14 CPU clocks / 7 CCKs (or 42 clocks if trap)
pub static STEPS_CHK_AI: [MicroStep; 5] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    MicroStep {
        step_fn: None,
        alu_fn: Some(alu_chk_mem),
        base_clocks: 6,
    },
    PREFETCH_IRC_READ,
    PREFETCH_IRC_FINISH,
];

/// CHK (An)+, Dn: 14 CPU clocks / 7 CCKs (or 42 clocks if trap)
pub static STEPS_CHK_PI: [MicroStep; 5] = [
    MicroStep {
        step_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: 2,
    },
    BUS_READ_IDLE,
    MicroStep {
        step_fn: None,
        alu_fn: Some(alu_chk_mem),
        base_clocks: 6,
    },
    PREFETCH_IRC_READ,
    PREFETCH_IRC_FINISH,
];

/// CHK -(An), Dn: 16 CPU clocks / 8 CCKs (or 44 clocks if trap)
pub static STEPS_CHK_PD: [MicroStep; 6] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep {
        step_fn: None,
        alu_fn: Some(alu_chk_mem),
        base_clocks: 6,
    },
    PREFETCH_IRC_READ,
    PREFETCH_IRC_FINISH,
];

/// CHK (d16, An), Dn: 18 CPU clocks / 9 CCKs (or 46 clocks if trap)
pub static STEPS_CHK_D16_AN: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep {
        step_fn: None,
        alu_fn: Some(alu_chk_mem),
        base_clocks: 6,
    },
    PREFETCH_IRC_READ,
    PREFETCH_IRC_FINISH,
];

/// CHK (d8, An, Xn), Dn: 22 CPU clocks / 11 CCKs (or 50 clocks if trap)
pub static STEPS_CHK_IDX_AN: [MicroStep; 8] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 4,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep {
        step_fn: None,
        alu_fn: Some(alu_chk_mem),
        base_clocks: 6,
    },
    PREFETCH_IRC_READ,
    PREFETCH_IRC_FINISH,
];

/// CHK (xxx).W, Dn: 18 CPU clocks / 9 CCKs (or 46 clocks if trap)
pub static STEPS_CHK_ABSW: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep {
        step_fn: None,
        alu_fn: Some(alu_chk_mem),
        base_clocks: 6,
    },
    PREFETCH_IRC_READ,
    PREFETCH_IRC_FINISH,
];

/// CHK (xxx).L, Dn: 22 CPU clocks / 11 CCKs (or 50 clocks if trap)
pub static STEPS_CHK_ABSL: [MicroStep; 9] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep {
        step_fn: None,
        alu_fn: Some(alu_chk_mem),
        base_clocks: 6,
    },
    PREFETCH_IRC_READ,
    PREFETCH_IRC_FINISH,
];

/// CHK (d16, PC), Dn: 18 CPU clocks / 9 CCKs (or 46 clocks if trap)
pub static STEPS_CHK_D16_PC: [MicroStep; 7] = [
    MicroStep {
        step_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep {
        step_fn: None,
        alu_fn: Some(alu_chk_mem),
        base_clocks: 6,
    },
    PREFETCH_IRC_READ,
    PREFETCH_IRC_FINISH,
];

/// CHK (d8, PC, Xn), Dn: 22 CPU clocks / 11 CCKs (or 50 clocks if trap)
pub static STEPS_CHK_IDX_PC: [MicroStep; 8] = [
    MicroStep {
        step_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 4,
    },
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    READ_SRC_WORD,
    BUS_READ_IDLE,
    MicroStep {
        step_fn: None,
        alu_fn: Some(alu_chk_mem),
        base_clocks: 6,
    },
    PREFETCH_IRC_READ,
    PREFETCH_IRC_FINISH,
];

/// CHK #<data>, Dn: 14 CPU clocks / 7 CCKs (or 42 clocks if trap)
pub static STEPS_CHK_IMM: [MicroStep; 6] = [
    MicroStep::alu(latch_imm_chk),
    FETCH_EXT_READ,
    FETCH_EXT_FINISH,
    MicroStep {
        step_fn: None,
        alu_fn: Some(alu_chk_mem),
        base_clocks: 6,
    },
    PREFETCH_IRC_READ,
    PREFETCH_IRC_FINISH,
];

/// Compile-time opcode decoder for CHK ($4180..=$4F80)
pub const fn decode_chk_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(&STEPS_CHK_DN),
        2 => Some(&STEPS_CHK_AI),
        3 => Some(&STEPS_CHK_PI),
        4 => Some(&STEPS_CHK_PD),
        5 => Some(&STEPS_CHK_D16_AN),
        6 => Some(&STEPS_CHK_IDX_AN),
        7 => match reg {
            0 => Some(&STEPS_CHK_ABSW),
            1 => Some(&STEPS_CHK_ABSL),
            2 => Some(&STEPS_CHK_D16_PC),
            3 => Some(&STEPS_CHK_IDX_PC),
            4 => Some(&STEPS_CHK_IMM),
            _ => None,
        },
        _ => None,
    }
}
