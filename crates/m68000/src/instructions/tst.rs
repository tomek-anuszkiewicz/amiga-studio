//! TST (Test an Operand) instruction handlers and CCR updates
//!
//! Evaluates an effective address operand against zero.
//! Updates N and Z flags according to the operand value; clears V and C flags.
//! Extend (X) flag is unaffected. Operand is not modified.

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::ea::{decode_ea_index, read_ea_operand, size_from_const};
use crate::state::CpuState;
use memory_bus::MemoryBus;

/// Executes TST: evaluates val against zero, updating N and Z flags, clearing V and C.
/// Extend (X) flag is unaffected.
#[inline]
pub fn execute_tst(state: &mut CpuState, val: u32, size: Size) {
    let (n, z) = match size {
        Size::Byte => {
            let b = (val & 0xFF) as u8;
            ((b & 0x80) != 0, b == 0)
        }
        Size::Word => {
            let w = (val & 0xFFFF) as u16;
            ((w & 0x8000) != 0, w == 0)
        }
        Size::Long => ((val & 0x8000_0000) != 0, val == 0),
    };
    state.set_n(n);
    state.set_z(z);
    state.set_v(false);
    state.set_c(false);
}

/// Execution handler for `TST <ea>`
pub fn op_tst(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    let size = size_from_const(s);
    let val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
        Ok(v) => v,
        Err(res) => return res,
    };
    execute_tst(&mut cpu.state, val, size);
    cpu.initiate_prefetch();
    cpu.state.micro.mark_standard_prefetch_retire();
    StepResult::StepCompleted
}

// --- Specialized Opcode Forwarders ---

#[inline(always)]
pub fn op_tst_b_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_b_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_b_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_b_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_b_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_b_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_b_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_b_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_b_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_b_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_l_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_l_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_l_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_l_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_l_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_l_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_l_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_l_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_l_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_l_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_w_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_w_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_w_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_w_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_w_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_w_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_w_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_w_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_w_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

#[inline(always)]
pub fn op_tst_w_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_tst(cpu, bus)
}

