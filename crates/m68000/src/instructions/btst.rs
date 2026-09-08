//! BTST (Bit Test) instruction handler and CCR update
//!
//! Tests a single bit in a Data Register (modulo 32) or memory operand (modulo 8).
//! Updates Z flag (set if bit is 0, cleared if bit is 1). Other condition codes unaffected.

use crate::core::{Cpu, StepResult};
use crate::instructions::ea::{
    decode_ea_index, read_ea_operand, EA_DN, SIZE_BYTE,
};
use crate::state::CpuState;
use memory_bus::MemoryBus;

/// Evaluates CCR updates for BTST: Z is set if tested bit is 0, cleared if 1
#[inline]
pub fn execute_btst(state: &mut CpuState, bit_num: u32, val: u32, is_register: bool) {
    let bit_idx = if is_register {
        bit_num % 32
    } else {
        bit_num % 8
    };
    let bit_val = (val & (1 << bit_idx)) != 0;
    state.set_ccr_z_only(!bit_val);
}

/// Dynamic bit test: `BTST Dn, <ea>`
pub fn op_btst_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    let reg_bit = ((cpu.state.ir >> 9) & 7) as usize;
    let bit_num = cpu.state.d_long(reg_bit);

    if m == EA_DN {
        let reg_d = (cpu.state.ir & 7) as usize;
        match cpu.state.micro.micro_step {
            0 => {
                let val = cpu.state.d_long(reg_d);
                execute_btst(&mut cpu.state, bit_num, val, true);
                cpu.initiate_prefetch();
                StepResult::StepCompleted
            }
            1 => {
                cpu.record_internal_clocks(2);
                StepResult::StepCompleted
            }
            2 => {
                cpu.state.ir = cpu.state.prefetch[0];
                cpu.state.prefetch[0] = cpu.state.micro.last_read;
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                cpu.state.micro.reset();
                StepResult::InstructionCompleted
            }
            _ => unreachable!(),
        }
    } else {
        let val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, SIZE_BYTE, m) {
            Ok(v) => v,
            Err(res) => return res,
        };
        execute_btst(&mut cpu.state, bit_num, val, false);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

/// Static bit test: `BTST #<data>, <ea>`
pub fn op_btst_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);

    if m == EA_DN {
        let reg_d = (cpu.state.ir & 7) as usize;
        match cpu.state.micro.micro_step {
            0 => {
                let bit_num = (cpu.state.prefetch[0] & 0xFF) as u32;
                cpu.state.micro.scratch[3] = bit_num;
                cpu.initiate_prefetch();
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                StepResult::StepCompleted
            }
            1 => {
                cpu.state.prefetch[0] = cpu.state.micro.last_read;
                let bit_num = cpu.state.micro.scratch[3];
                let val = cpu.state.d_long(reg_d);
                execute_btst(&mut cpu.state, bit_num, val, true);
                cpu.initiate_prefetch();
                StepResult::StepCompleted
            }
            2 => {
                cpu.record_internal_clocks(2);
                StepResult::StepCompleted
            }
            3 => {
                cpu.state.ir = cpu.state.prefetch[0];
                cpu.state.prefetch[0] = cpu.state.micro.last_read;
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                cpu.state.micro.reset();
                StepResult::InstructionCompleted
            }
            _ => unreachable!(),
        }
    } else {
        let step = cpu.state.micro.micro_step;
        if step == 0 {
            let bit_num = (cpu.state.prefetch[0] & 0xFF) as u32;
            cpu.state.micro.scratch[3] = bit_num;
            cpu.initiate_prefetch();
            cpu.state.pc = cpu.state.pc.wrapping_add(2);
            return StepResult::StepCompleted;
        }

        let ea_step = step - 1;
        if ea_step == 0 {
            cpu.state.prefetch[0] = cpu.state.micro.last_read;
        }
        let val = match read_ea_operand(cpu, bus, ea_step, SIZE_BYTE, m) {
            Ok(v) => v,
            Err(res) => return res,
        };

        let bit_num = cpu.state.micro.scratch[3];
        execute_btst(&mut cpu.state, bit_num, val, false);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

// --- Specialized Opcode Forwarders ---

pub fn op_btst_b_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_dn(cpu, bus)
}

pub fn op_btst_b_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_dn(cpu, bus)
}

pub fn op_btst_b_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_dn(cpu, bus)
}

pub fn op_btst_b_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_dn(cpu, bus)
}

pub fn op_btst_b_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_dn(cpu, bus)
}

pub fn op_btst_b_dn_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_dn(cpu, bus)
}

pub fn op_btst_b_dn_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_dn(cpu, bus)
}

pub fn op_btst_b_dn_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_dn(cpu, bus)
}

pub fn op_btst_b_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_dn(cpu, bus)
}

pub fn op_btst_b_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_dn(cpu, bus)
}

pub fn op_btst_b_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_imm(cpu, bus)
}

pub fn op_btst_b_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_imm(cpu, bus)
}

pub fn op_btst_b_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_imm(cpu, bus)
}

pub fn op_btst_b_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_imm(cpu, bus)
}

pub fn op_btst_b_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_imm(cpu, bus)
}

pub fn op_btst_b_imm_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_imm(cpu, bus)
}

pub fn op_btst_b_imm_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_imm(cpu, bus)
}

pub fn op_btst_b_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_imm(cpu, bus)
}

pub fn op_btst_b_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_imm(cpu, bus)
}

pub fn op_btst_l_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_dn(cpu, bus)
}

pub fn op_btst_l_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_btst_imm(cpu, bus)
}

