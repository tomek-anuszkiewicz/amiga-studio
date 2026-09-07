//! M68000 SUBA Instruction (`SUBA <ea>, An`)
//!
//! Subtracts an effective address operand from an address register without modifying CCR flags.
//! Sign-extends word operands to 32 bits prior to subtraction.

use crate::core::{Cpu, StepResult};
use crate::instructions::ea::*;
use memory_bus::MemoryBus;

pub fn op_suba(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = if ((ir >> 8) & 1) != 0 { SIZE_LONG } else { SIZE_WORD };
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_suba(cpu, bus, s, m)
}

#[inline(always)]
fn exec_suba(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
    if m == EA_DN || m == EA_AN {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.record_internal_clocks(4);
                StepResult::StepCompleted
            }
            1 => {
                let reg_a = ((cpu.state.ir >> 9) & 7) as usize;
                let reg_s = (cpu.state.ir & 7) as usize;
                let raw_val = if m == EA_DN {
                    cpu.state.d_long(reg_s)
                } else {
                    cpu.state.read_a(reg_s)
                };
                let s_val = if s == SIZE_WORD {
                    (raw_val as i16 as i32) as u32
                } else {
                    raw_val
                };
                let a_val = cpu.state.read_a(reg_a);
                let res = a_val.wrapping_sub(s_val);
                cpu.state.write_a(reg_a, res);
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
        let res = a_val.wrapping_sub(s_val);
        cpu.state.write_a(reg_a, res);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

// --- Specialized Opcode Forwarders ---

pub fn op_suba_l_absl_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_l_absw_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_l_ai_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_l_an_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_l_disp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_l_dn_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_l_idx_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_l_imm_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_l_pcdisp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_l_pcidx_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_l_pd_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_l_pi_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_w_absl_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_w_absw_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_w_ai_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_w_an_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_w_disp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_w_dn_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_w_idx_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_w_imm_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_w_pcdisp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_w_pcidx_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_w_pd_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

pub fn op_suba_w_pi_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_suba(cpu, bus)
}

