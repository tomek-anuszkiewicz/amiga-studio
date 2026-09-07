//! CMPA (Compare Address) instruction handlers
//!
//! Compares an effective address operand with an Address Register (An).
//! Word operands are sign-extended to 32 bits and compared as 32-bit (Long).
//! Condition codes (N, Z, V, C) are updated; X flag is unaffected.

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::cmp::execute_cmp;
use crate::instructions::ea::{decode_ea_index, read_ea_operand, EA_AN, EA_DN, SIZE_LONG, SIZE_WORD};
use memory_bus::MemoryBus;

/// Execution handler for `CMPA <ea>, An`
pub fn op_cmpa(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = if ((ir >> 8) & 1) != 0 { SIZE_LONG } else { SIZE_WORD };
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);

    if m == EA_DN || m == EA_AN {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.record_internal_clocks(2);
                StepResult::StepCompleted
            }
            1 => {
                let reg_a = ((cpu.state.ir >> 9) & 7) as usize;
                let reg_s = (cpu.state.ir & 7) as usize;
                let raw_val = if m == EA_DN {
                    cpu.state.d[reg_s]
                } else {
                    cpu.state.read_a(reg_s)
                };
                let s_val = if s == SIZE_WORD {
                    (raw_val as i16 as i32) as u32
                } else {
                    raw_val
                };
                let a_val = cpu.state.read_a(reg_a);
                execute_cmp(&mut cpu.state, s_val, a_val, Size::Long);
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        let raw_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
            Ok(v) => v,
            Err(res) => return res,
        };
        let reg_a = ((cpu.state.ir >> 9) & 7) as usize;
        let s_val = if s == SIZE_WORD {
            (raw_val as i16 as i32) as u32
        } else {
            raw_val
        };
        let a_val = cpu.state.read_a(reg_a);
        execute_cmp(&mut cpu.state, s_val, a_val, Size::Long);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

// --- Specialized Opcode Forwarders ---

#[inline(always)]
pub fn op_cmpa_l_absl_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_l_absw_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_l_ai_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_l_an_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_l_disp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_l_dn_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_l_idx_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_l_imm_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_l_pcdisp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_l_pcidx_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_l_pd_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_l_pi_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_w_absl_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_w_absw_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_w_ai_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_w_an_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_w_disp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_w_dn_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_w_idx_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_w_imm_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_w_pcdisp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_w_pcidx_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_w_pd_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

#[inline(always)]
pub fn op_cmpa_w_pi_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpa(cpu, bus)
}

