//! Specialized MOVE Word (16-bit) Opcode Forwarders
//!
//! Each handler forwards directly to the core execution routines
//! until flat linear opcode specializations are implemented.

use crate::core::{Cpu, StepResult};
use crate::instructions::r#move::{op_move_to_mem, op_move_to_reg};
use memory_bus::MemoryBus;

#[inline(always)]
pub fn op_move_w_absl_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absl_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absl_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absl_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absl_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absl_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absl_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absl_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absw_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absw_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absw_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absw_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absw_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absw_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absw_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_absw_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_ai_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_ai_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_ai_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_ai_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_ai_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_ai_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_ai_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_an_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_an_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_an_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_an_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_an_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_an_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_an_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_an_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_disp_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_disp_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_disp_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_disp_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_disp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_disp_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_disp_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_disp_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_idx_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_idx_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_idx_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_idx_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_idx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_idx_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_idx_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_idx_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcdisp_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcdisp_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcdisp_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcdisp_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcdisp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcdisp_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcdisp_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcdisp_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcidx_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcidx_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcidx_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcidx_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcidx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcidx_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcidx_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pcidx_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pd_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pd_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pd_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pd_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pd_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pd_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pd_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pd_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pi_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pi_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pi_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pi_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pi_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_reg(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pi_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pi_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

#[inline(always)]
pub fn op_move_w_pi_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_move_to_mem(cpu, bus)
}

