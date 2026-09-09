//! M68000 DBcc Instruction (`DBcc Dn, <label>`)
//!
//! Decrement and Branch Conditionally.
//! - If condition is FALSE, decrements lower 16 bits of Dn:
//!   - If Dn.w != -1, branches to PC + d16 (10 CPU clocks / 5 CCKs).
//!   - If Dn.w == -1, loop expires and falls through to next instruction (12 CPU clocks / 6 CCKs).
//! - If condition is TRUE, loop terminates and falls through without modifying Dn (12 CPU clocks / 6 CCKs).
//!
//! Condition Codes: Strictly unaffected.

use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;

/// Taken branch execution steps (10 CPU clocks / 5 CCKs)
pub static STEPS_DBCC_BRANCH_TAKEN: [MicroStep; 5] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 2,
    },
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// Untaken branch / condition met execution steps (12 CPU clocks / 6 CCKs)
pub static STEPS_DBCC_COND_TRUE: [MicroStep; 6] = [
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 2,
    },
    MicroStep {
        step_fn: None,
        alu_fn: None,
        base_clocks: 2,
    },
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

pub fn alu_dbcc(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let cond = ((state.ir >> 8) & 0x0F) as u8;
    if state.eval_condition(cond) {
        state.micro.ea_addr = state.pc;
        state.micro.current_steps = &STEPS_DBCC_COND_TRUE;
    } else {
        let val = state.d_long(reg_dst as usize) as u16;
        let new_val = val.wrapping_sub(1);
        let orig = state.d_long(reg_dst as usize);
        if new_val != 0xFFFF {
            let disp = state.prefetch[0] as i16 as i32;
            let base_pc = state.pc.wrapping_sub(2);
            let target_pc = base_pc.wrapping_add(disp as u32);
            state.micro.ea_addr = target_pc;
            state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (new_val as u32));
            state.micro.current_steps = &STEPS_DBCC_BRANCH_TAKEN;
        } else {
            state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (new_val as u32));
            state.micro.ea_addr = state.pc;
            state.micro.current_steps = &STEPS_DBCC_COND_TRUE;
        }
    }
    state.micro.micro_step = 0;
}

pub static STEPS_DBCC: [MicroStep; 1] = [MicroStep::alu(alu_dbcc)];
