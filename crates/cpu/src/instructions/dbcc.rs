//! M68000 DBcc Instruction (`DBcc Dn, <label>`)
//!
//! Decrement and Branch Conditionally.
//! - If condition is FALSE, decrements lower 16 bits of Dn:
//!   - If Dn.w != -1, branches to PC + d16 (10 CPU clocks / 5 CCKs).
//!   - If Dn.w == -1, loop expires and falls through to next instruction (14 CPU clocks / 7 CCKs).
//! - If condition is TRUE, loop terminates and falls through without modifying Dn (12 CPU clocks / 6 CCKs).
//!
//! Condition Codes: Strictly unaffected.

use crate::micro::common;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use crate::Cpu;
use physical_memory::{AddressBus, BusResult};

/// Taken branch execution steps (10 CPU clocks / 5 CCKs)
pub static STEPS_DBCC_BRANCH_TAKEN: [MicroStep; 5] = [
    common::ALU_IDLE,
    common::READ_CONTROL_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// Condition met execution steps (12 CPU clocks / 6 CCKs)
pub static STEPS_DBCC_COND_TRUE: [MicroStep; 6] = [
    common::ALU_IDLE,
    common::ALU_IDLE,
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

/// Counter expired: read the branch target, then refill from the fallthrough address.
/// MC68000 manual Table 8-9 gives 14 clocks and three reads; WinUAE's DBcc
/// generator identifies the first read as a speculative branch-target prefetch.
/// Supplied DBcc SingleStep vectors omit this path; test_dbcc covers it separately.
pub static STEPS_DBCC_COUNTER_EXPIRED: [MicroStep; 7] = [
    common::ALU_IDLE,
    common::READ_CONTROL_TARGET_OPCODE_READ,
    MicroStep::cck(Cpu::step_dbcc_expired_target_finish_idle),
    common::READ_TARGET_OPCODE_READ,
    common::BUS_READ_IDLE,
    common::PREFETCH_TARGET_READ,
    common::PREFETCH_TARGET_FINISH,
];

impl Cpu {
    /// Discard the target word and redirect the next prefetch to the fallthrough.
    fn step_dbcc_expired_target_finish_idle(&mut self, _bus: &mut dyn AddressBus) -> BusResult<()> {
        self.state.micro_mut().ea_addr = self.state.pc();
        BusResult::Ready(())
    }
}

pub fn alu_dbcc(state: &mut CpuState, _reg_src: u8, reg_dst: u8) {
    let cond = ((state.ir() >> 8) & 0x0F) as u8;
    if state.eval_condition(cond) {
        state.micro_mut().ea_addr = state.pc();
        state.micro_mut().current_steps = &STEPS_DBCC_COND_TRUE;
    } else {
        let val = state.d_long(reg_dst as usize) as u16;
        let new_val = val.wrapping_sub(1);
        let orig = state.d_long(reg_dst as usize);
        let disp = state.prefetch() as i16 as i32;
        let target_pc = state.pc().wrapping_sub(2).wrapping_add(disp as u32);
        state.micro_mut().ea_addr = target_pc;
        state.set_d_long(reg_dst as usize, (orig & !0xFFFF) | (new_val as u32));
        if new_val != 0xFFFF {
            state.micro_mut().current_steps = &STEPS_DBCC_BRANCH_TAKEN;
        } else {
            state.micro_mut().current_steps = &STEPS_DBCC_COUNTER_EXPIRED;
        }
    }
    state.micro_mut().micro_step = 0;
}

pub static STEPS_DBCC: [MicroStep; 1] = [MicroStep::alu(alu_dbcc)];
