//! Specialized MOVE Long (32-bit) Opcode Forwarders
//!
//! Each handler forwards directly to the core execution routines
//! until flat linear opcode specializations are implemented.

use crate::core::{Cpu, StepResult};
use crate::instructions::r#move::{op_move_to_mem, op_move_to_reg};
use memory_bus::MemoryBus;

#[inline(always)]
pub fn op_move_l_absl_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absl_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absl_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absl_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absl_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absl_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absl_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absl_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absw_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absw_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absw_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absw_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absw_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absw_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absw_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_absw_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_ai_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_ai_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_ai_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_ai_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_ai_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_ai_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_ai_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_an_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_an_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_an_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_an_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_an_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_an_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_an_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_an_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_disp_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_disp_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_disp_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_disp_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_disp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_disp_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_disp_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_disp_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_idx_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_idx_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_idx_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_idx_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_idx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_idx_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_idx_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_idx_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcdisp_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcdisp_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcdisp_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcdisp_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcdisp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcdisp_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcdisp_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcdisp_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcidx_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcidx_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcidx_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcidx_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcidx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcidx_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcidx_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pcidx_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pd_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pd_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pd_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pd_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pd_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pd_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pd_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pd_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pi_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pi_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pi_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pi_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pi_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pi_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pi_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_l_pi_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

