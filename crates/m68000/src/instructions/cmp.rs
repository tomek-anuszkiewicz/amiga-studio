//! CMP (Compare) instruction handlers and CCR updates
//!
//! Evaluates (dst - src) and updates N, Z, V, and C condition code flags.
//! Extend (X) flag is unaffected. Destination operand is not modified.

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::ea::{
    decode_ea_index, read_ea_operand, size_from_const, EA_AN, EA_DN, SIZE_LONG,
};
use crate::state::CpuState;
use memory_bus::MemoryBus;

/// Executes CMP: evaluates (dst - src) and updates N, Z, V, C flags.
/// Extend (X) flag is unaffected.
#[inline]
pub fn execute_cmp(state: &mut CpuState, src: u32, dst: u32, size: Size) {
    match size {
        Size::Byte => {
            let s = (src & 0xFF) as u8;
            let d = (dst & 0xFF) as u8;
            let (res, c) = d.overflowing_sub(s);
            let v = (((s ^ d) & (d ^ res)) & 0x80) != 0;
            let n = (res & 0x80) != 0;
            let z = res == 0;
            state.set_c(c);
            state.set_v(v);
            state.set_n(n);
            state.set_z(z);
        }
        Size::Word => {
            let s = (src & 0xFFFF) as u16;
            let d = (dst & 0xFFFF) as u16;
            let (res, c) = d.overflowing_sub(s);
            let v = (((s ^ d) & (d ^ res)) & 0x8000) != 0;
            let n = (res & 0x8000) != 0;
            let z = res == 0;
            state.set_c(c);
            state.set_v(v);
            state.set_n(n);
            state.set_z(z);
        }
        Size::Long => {
            let (res, c) = dst.overflowing_sub(src);
            let v = (((src ^ dst) & (dst ^ res)) & 0x8000_0000) != 0;
            let n = (res & 0x8000_0000) != 0;
            let z = res == 0;
            state.set_c(c);
            state.set_v(v);
            state.set_n(n);
            state.set_z(z);
        }
    }
}

/// Execution handler for `CMP <ea>, Dn`
pub fn op_cmp_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    let size = size_from_const(s);

    if s == SIZE_LONG && (m == EA_DN || m == EA_AN) {
        match cpu.state.micro.micro_step {
            0 => {
                cpu.record_internal_clocks(2);
                StepResult::StepCompleted
            }
            1 => {
                let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
                let reg_s = (cpu.state.ir & 7) as usize;
                let s_val = if m == EA_DN {
                    cpu.state.d_long(reg_s)
                } else {
                    cpu.state.read_a(reg_s)
                };
                let d_val = cpu.state.d_long(reg_d);
                execute_cmp(&mut cpu.state, s_val, d_val, Size::Long);
                cpu.initiate_prefetch();
                cpu.state.micro.mark_standard_prefetch_retire();
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        let s_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
            Ok(v) => v,
            Err(res) => return res,
        };
        let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
        let d_val = cpu.state.d_long(reg_d);
        execute_cmp(&mut cpu.state, s_val, d_val, size);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

// --- Specialized Opcode Forwarders ---

pub fn op_cmp_b_absl_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_b_absw_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_b_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_b_disp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_b_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_b_idx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_b_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_b_pcdisp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_b_pcidx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_b_pd_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_b_pi_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_l_absl_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_l_absw_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_l_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_l_an_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_l_disp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_l_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_l_idx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_l_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_l_pcdisp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_l_pcidx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_l_pd_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_l_pi_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_w_absl_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_w_absw_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_w_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_w_an_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_w_disp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_w_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_w_idx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_w_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_w_pcdisp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_w_pcidx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_w_pd_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

pub fn op_cmp_w_pi_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmp_ea_to_dn(cpu, bus)
}

