//! Specialized MOVE Byte (8-bit) Opcode Forwarders
//!
//! Each handler forwards directly to the core execution routines
//! until flat linear opcode specializations are implemented.

use crate::core::{Cpu, StepResult};
use crate::instructions::r#move::{op_move_to_mem, op_move_to_reg};
use memory_bus::MemoryBus;

#[inline(always)]
pub fn op_move_b_absl_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absl_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absl_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absl_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absl_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absl_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absl_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absl_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absw_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absw_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absw_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absw_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absw_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absw_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absw_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_absw_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_ai_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_ai_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_ai_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_ai_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_ai_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_ai_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_ai_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_disp_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_disp_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_disp_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_disp_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_disp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_disp_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_disp_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_disp_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_idx_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_idx_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_idx_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_idx_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_idx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_idx_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_idx_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_idx_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcdisp_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcdisp_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcdisp_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcdisp_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcdisp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcdisp_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcdisp_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcdisp_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcidx_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcidx_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcidx_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcidx_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcidx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcidx_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcidx_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pcidx_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pd_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pd_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pd_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pd_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pd_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pd_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pd_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pd_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pi_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pi_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pi_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pi_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pi_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pi_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pi_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_b_pi_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

