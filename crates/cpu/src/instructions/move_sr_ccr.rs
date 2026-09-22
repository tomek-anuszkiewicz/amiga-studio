//! MOVE to/from SR/CCR Handlers
//!
//! Implements cycle-exact handlers for Status Register (SR) and Condition Code Register (CCR) operations:
//! - `MOVE <ea>, CCR` (12..24 clocks)
//! - `MOVE <ea>, SR` (Privileged, 12..24 clocks or 34 clocks on privilege violation)
//! - `MOVE SR, <ea>` (6 clocks for Dn, 12..22 clocks for memory)

use crate::micro::common;
use crate::micro::ea;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;

// ============================================================================
// Privilege and Setup ALU Callbacks
// ============================================================================

pub fn alu_check_privilege(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    if !state.is_supervisor() {
        state.micro.current_steps = &common::STEPS_PRIVILEGE_VIOLATION;
        state.micro.micro_step = 0;
        state.micro.clocks_remaining = 0;
    }
}

pub static ALU_CHECK_PRIVILEGE: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_check_privilege),
    base_clocks: 0,
};

pub fn alu_latch_src_dn_word(state: &mut CpuState, reg_src: u8, _reg_dst: u8) {
    state.micro.source = state.d_long(reg_src as usize) & 0xFFFF;
}

pub static ALU_LATCH_SRC_DN_WORD: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_latch_src_dn_word),
    base_clocks: 0,
};

pub fn alu_set_ccr(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.set_ccr((state.micro.source & 0xFF) as u8);
}

pub static ALU_SET_CCR_4CLK: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_set_ccr),
    base_clocks: 4,
};

pub fn alu_set_sr(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.set_sr(state.micro.source as u16);
}

pub static ALU_SET_SR_4CLK: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_set_sr),
    base_clocks: 4,
};

// ============================================================================
// MOVE <ea>, CCR Static Step Sequences
// ============================================================================

pub static STEPS_MOVE_TO_CCR_DN: [MicroStep; 6] = [
    ALU_LATCH_SRC_DN_WORD,
    ALU_SET_CCR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_CCR_AI: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    ALU_SET_CCR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_CCR_PI: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    ALU_SET_CCR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_CCR_PD: [MicroStep; 8] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_CCR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_CCR_D16: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_CCR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_CCR_IDX: [MicroStep; 10] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_CCR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_CCR_AW: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_CCR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_CCR_AL: [MicroStep; 11] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_CCR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_CCR_PC_D16: [MicroStep; 9] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_CCR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_CCR_PC_IDX: [MicroStep; 10] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_CCR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_CCR_IMM: [MicroStep; 7] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    ALU_SET_CCR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// MOVE <ea>, SR Static Step Sequences (Privileged)
// ============================================================================

pub static STEPS_MOVE_TO_SR_DN: [MicroStep; 7] = [
    ALU_CHECK_PRIVILEGE,
    ALU_LATCH_SRC_DN_WORD,
    ALU_SET_SR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_SR_AI: [MicroStep; 8] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    ALU_SET_SR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_SR_PI: [MicroStep; 8] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_src_word),
        alu_fn: Some(ea::ea_calc_src_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    ALU_SET_SR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_SR_PD: [MicroStep; 9] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_pd_w),
        base_clocks: 2,
    },
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_SR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_SR_D16: [MicroStep; 10] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_src_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_SR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_SR_IDX: [MicroStep; 11] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_src_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_SR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_SR_AW: [MicroStep; 10] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_SR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_SR_AL: [MicroStep; 12] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_SR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_SR_PC_D16: [MicroStep; 10] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_d16_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_SR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_SR_PC_IDX: [MicroStep; 11] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_idx_pc),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_SRC_WORD,
    common::BUS_READ_IDLE,
    ALU_SET_SR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

pub static STEPS_MOVE_TO_SR_IMM: [MicroStep; 8] = [
    ALU_CHECK_PRIVILEGE,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_imm_w),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    ALU_SET_SR_4CLK,
    common::REFILL_FIRST_READ,
    common::BUS_READ_IDLE,
    common::REFILL_SECOND_READ,
    common::BUS_READ_IDLE,
];

// ============================================================================
// MOVE SR, <ea> Static Step Sequences (Unprivileged on 68000)
// ============================================================================

pub fn alu_move_from_sr_dn(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let orig = state.d_long(reg_dst as usize);
    state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (state.sr() as u32));
}

pub static ALU_MOVE_FROM_SR_DN: MicroStep = MicroStep {
    bus_fn: None,
    alu_fn: Some(alu_move_from_sr_dn),
    base_clocks: 2,
};

pub static STEPS_MOVE_FROM_SR_DN: [MicroStep; 3] = [
    common::PREFETCH_NEXT_READ,
    common::BUS_READ_IDLE,
    ALU_MOVE_FROM_SR_DN,
];

pub fn alu_move_from_sr_init(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    state.micro.destination = state.sr() as u32;
}

pub static STEPS_MOVE_FROM_SR_AI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_ai),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_from_sr_init),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_MOVE_FROM_SR_PI: [MicroStep; 6] = [
    MicroStep {
        bus_fn: Some(Cpu::step_bus_read_dst_word),
        alu_fn: Some(ea::ea_calc_dst_pi_w),
        base_clocks: 2,
    },
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_from_sr_init),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_MOVE_FROM_SR_PD: [MicroStep; 7] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_pd_w),
        base_clocks: 2,
    },
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_from_sr_init),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_MOVE_FROM_SR_D16: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_dst_d16_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_from_sr_init),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_MOVE_FROM_SR_IDX: [MicroStep; 9] = [
    MicroStep {
        bus_fn: None,
        alu_fn: Some(ea::ea_calc_dst_idx_an),
        base_clocks: 2,
    },
    common::FETCH_EXT_READ,
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_from_sr_init),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_MOVE_FROM_SR_AW: [MicroStep; 8] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absw),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_from_sr_init),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

pub static STEPS_MOVE_FROM_SR_AL: [MicroStep; 10] = [
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_hi),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    MicroStep {
        bus_fn: Some(Cpu::step_fetch_extension_read),
        alu_fn: Some(ea::ea_calc_absl_lo),
        base_clocks: 2,
    },
    common::FETCH_EXT_FINISH,
    common::READ_DST_WORD,
    common::BUS_READ_IDLE,
    MicroStep {
        bus_fn: Some(Cpu::step_prefetch_next_read),
        alu_fn: Some(alu_move_from_sr_init),
        base_clocks: 2,
    },
    common::PREFETCH_IRC_FINISH,
    common::BUS_WRITE_IDLE,
    common::WRITE_DST_WORD,
];

// ============================================================================
// Opcode Decoding Helpers
// ============================================================================

pub const fn decode_move_to_ccr_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(&STEPS_MOVE_TO_CCR_DN),
        2 => Some(&STEPS_MOVE_TO_CCR_AI),
        3 => Some(&STEPS_MOVE_TO_CCR_PI),
        4 => Some(&STEPS_MOVE_TO_CCR_PD),
        5 => Some(&STEPS_MOVE_TO_CCR_D16),
        6 => Some(&STEPS_MOVE_TO_CCR_IDX),
        7 => match reg {
            0 => Some(&STEPS_MOVE_TO_CCR_AW),
            1 => Some(&STEPS_MOVE_TO_CCR_AL),
            2 => Some(&STEPS_MOVE_TO_CCR_PC_D16),
            3 => Some(&STEPS_MOVE_TO_CCR_PC_IDX),
            4 => Some(&STEPS_MOVE_TO_CCR_IMM),
            _ => None,
        },
        _ => None,
    }
}

pub const fn decode_move_to_sr_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(&STEPS_MOVE_TO_SR_DN),
        2 => Some(&STEPS_MOVE_TO_SR_AI),
        3 => Some(&STEPS_MOVE_TO_SR_PI),
        4 => Some(&STEPS_MOVE_TO_SR_PD),
        5 => Some(&STEPS_MOVE_TO_SR_D16),
        6 => Some(&STEPS_MOVE_TO_SR_IDX),
        7 => match reg {
            0 => Some(&STEPS_MOVE_TO_SR_AW),
            1 => Some(&STEPS_MOVE_TO_SR_AL),
            2 => Some(&STEPS_MOVE_TO_SR_PC_D16),
            3 => Some(&STEPS_MOVE_TO_SR_PC_IDX),
            4 => Some(&STEPS_MOVE_TO_SR_IMM),
            _ => None,
        },
        _ => None,
    }
}

pub const fn decode_move_from_sr_steps(mode: u8, reg: u8) -> Option<&'static [MicroStep]> {
    match mode {
        0 => Some(&STEPS_MOVE_FROM_SR_DN),
        2 => Some(&STEPS_MOVE_FROM_SR_AI),
        3 => Some(&STEPS_MOVE_FROM_SR_PI),
        4 => Some(&STEPS_MOVE_FROM_SR_PD),
        5 => Some(&STEPS_MOVE_FROM_SR_D16),
        6 => Some(&STEPS_MOVE_FROM_SR_IDX),
        7 => match reg {
            0 => Some(&STEPS_MOVE_FROM_SR_AW),
            1 => Some(&STEPS_MOVE_FROM_SR_AL),
            _ => None,
        },
        _ => None,
    }
}
