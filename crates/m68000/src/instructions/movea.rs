//! MOVEA (Move Address) instruction handlers
//!
//! Handles MOVEA.W and MOVEA.L instructions which load an effective address operand
//! into an Address Register (An) with word sign-extension, leaving CCR untouched.

use crate::core::{Cpu, StepResult};
use crate::instructions::ea::{decode_ea_index, read_ea_operand, SIZE_LONG, SIZE_WORD};
use memory_bus::MemoryBus;

/// Sign-extends a word to 32 bits for MOVEA.W
#[inline]
pub fn sign_extend_word(val: u16) -> u32 {
    (val as i16 as i32) as u32
}

/// Specialized execution handler for MOVEA <ea>, An
pub fn op_movea(
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
) -> StepResult {
    let ir = cpu.state.ir;
    let s = match (ir >> 12) & 3 {
        3 => SIZE_WORD,
        _ => SIZE_LONG,
    };
    let src_m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);

    let val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, src_m) {
        Ok(v) => v,
        Err(res) => return res,
    };

    let dst_reg = ((ir >> 9) & 7) as usize;

    // MOVEA <ea>, An: Sign-extend Word, Long written directly; CCR untouched
    let final_val = if s == SIZE_WORD {
        val as i16 as i32 as u32
    } else {
        val
    };
    cpu.state.write_a(dst_reg, final_val);

    cpu.initiate_prefetch();
    cpu.state.micro.mark_standard_prefetch_retire();
    StepResult::StepCompleted
}

// --- Specialized Opcode Forwarders ---

#[inline(always)]
pub fn op_movea_l_absl_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_l_absw_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_l_ai_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_l_an_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_l_disp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_l_dn_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_l_idx_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_l_imm_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_l_pcdisp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_l_pcidx_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_l_pd_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_l_pi_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_w_absl_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_w_absw_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_w_ai_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_w_an_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_w_disp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_w_dn_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_w_idx_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_w_imm_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_w_pcdisp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_w_pcidx_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_w_pd_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

#[inline(always)]
pub fn op_movea_w_pi_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_movea(cpu, bus)
}

