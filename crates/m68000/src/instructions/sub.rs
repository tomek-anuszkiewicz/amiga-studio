//! M68000 SUB Instruction (`SUB <ea>, Dn` and `SUB Dn, <ea>`)
//!
//! Provides cycle-exact Color Clock execution handlers and branchless
//! size-specialized ALU arithmetic functions (`sub_b`, `sub_w`, `sub_l`, `execute_sub`).

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::ea::*;
use crate::state::CpuState;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

// ============================================================================
// Leaf ALU Subtraction Functions (Branchless & Cycle-Exact CCR)
// ============================================================================

#[inline(always)]
pub fn sub_b(state: &mut CpuState, s: u8, d: u8, update_ccr: bool) -> u8 {
    let (res, c) = d.overflowing_sub(s);
    if update_ccr {
        let v = (((s ^ d) & (d ^ res)) & 0x80) != 0;
        let n = (res & 0x80) != 0;
        let z = res == 0;
        state.set_x(c);
        state.set_c(c);
        state.set_v(v);
        state.set_n(n);
        state.set_z(z);
    }
    res
}

#[inline(always)]
pub fn sub_w(state: &mut CpuState, s: u16, d: u16, update_ccr: bool) -> u16 {
    let (res, c) = d.overflowing_sub(s);
    if update_ccr {
        let v = (((s ^ d) & (d ^ res)) & 0x8000) != 0;
        let n = (res & 0x8000) != 0;
        let z = res == 0;
        state.set_x(c);
        state.set_c(c);
        state.set_v(v);
        state.set_n(n);
        state.set_z(z);
    }
    res
}

#[inline(always)]
pub fn sub_l(state: &mut CpuState, s: u32, d: u32, update_ccr: bool) -> u32 {
    let (res, c) = d.overflowing_sub(s);
    if update_ccr {
        let v = (((s ^ d) & (d ^ res)) & 0x8000_0000) != 0;
        let n = (res & 0x8000_0000) != 0;
        let z = res == 0;
        state.set_x(c);
        state.set_c(c);
        state.set_v(v);
        state.set_n(n);
        state.set_z(z);
    }
    res
}

pub fn execute_sub(state: &mut CpuState, src: u32, dst: u32, size: Size, update_ccr: bool) -> u32 {
    match size {
        Size::Byte => {
            let res = sub_b(state, (src & 0xFF) as u8, (dst & 0xFF) as u8, update_ccr);
            (dst & !0xFF) | (res as u32)
        }
        Size::Word => {
            let res = sub_w(
                state,
                (src & 0xFFFF) as u16,
                (dst & 0xFFFF) as u16,
                update_ccr,
            );
            (dst & !0xFFFF) | (res as u32)
        }
        Size::Long => sub_l(state, src, dst, update_ccr),
    }
}

// ============================================================================
// SUB: <ea>, Dn
// ============================================================================

pub fn op_sub_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_sub_ea_to_dn(cpu, bus, s, m)
}

#[inline(always)]
fn exec_sub_ea_to_dn(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
    let size = size_from_const(s);
    if s == SIZE_LONG {
        if m == EA_DN || m == EA_AN {
            match cpu.state.micro.micro_step {
                0 => {
                    cpu.record_internal_clocks(4);
                    StepResult::StepCompleted
                }
                1 => {
                    let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
                    let reg_s = (cpu.state.ir & 7) as usize;
                    let s_val = if m == EA_DN {
                        cpu.state.d[reg_s]
                    } else {
                        cpu.state.read_a(reg_s)
                    };
                    let d_val = cpu.state.d[reg_d];
                    let res = execute_sub(&mut cpu.state, s_val, d_val, Size::Long, true);
                    cpu.write_d_reg(reg_d, res, Size::Long);
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
            let d_val = cpu.state.d[reg_d];
            let res = execute_sub(&mut cpu.state, s_val, d_val, Size::Long, true);
            cpu.write_d_reg(reg_d, res, Size::Long);
            cpu.initiate_prefetch();
            cpu.state.micro.mark_standard_prefetch_retire();
            StepResult::StepCompleted
        }
    } else {
        let s_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
            Ok(v) => v,
            Err(res) => return res,
        };
        let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
        let d_val = cpu.state.d[reg_d];
        let res = execute_sub(&mut cpu.state, s_val, d_val, size, true);
        cpu.write_d_reg(reg_d, res, size);
        cpu.initiate_prefetch();
        cpu.state.micro.mark_standard_prefetch_retire();
        StepResult::StepCompleted
    }
}

// ============================================================================
// SUB: Dn, <ea> (Class 0 Read-Modify-Write)
// ============================================================================

pub fn op_sub_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    exec_sub_dn_to_ea(cpu, bus, s, m)
}

#[inline(always)]
fn exec_sub_dn_to_ea(cpu: &mut Cpu, bus: &mut MemoryBus, s: u8, m: u8) -> StepResult {
    let size = size_from_const(s);
    let bus_size = bus_size_from_const(s);
    let fc = data_fc(cpu);

    let phase = cpu.state.micro.scratch[2];
    if phase == 0 {
        let mem_val = match read_ea_operand(cpu, bus, cpu.state.micro.micro_step, s, m) {
            Ok(v) => v,
            Err(res) => return res,
        };
        let reg_d = ((cpu.state.ir >> 9) & 7) as usize;
        let d_val = cpu.state.d[reg_d];
        let res = execute_sub(&mut cpu.state, d_val, mem_val, size, true);
        cpu.state.micro.scratch[1] = res;
        cpu.initiate_prefetch();
        cpu.state.micro.scratch[2] = 1;
        return StepResult::StepCompleted;
    }

    if s == SIZE_LONG {
        if phase == 1 {
            cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
            let addr2 = cpu.state.micro.scratch[0].wrapping_add(2);
            let lo = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
            cpu.initiate_bus_cycle(BusCycle::new_write(addr2, lo, BusAccessSize::Word, fc));
            cpu.state.micro.scratch[2] = 2;
            StepResult::StepCompleted
        } else {
            let addr = cpu.state.micro.scratch[0];
            let hi = ((cpu.state.micro.scratch[1] >> 16) & 0xFFFF) as u16;
            cpu.initiate_bus_cycle(BusCycle::new_write(addr, hi, BusAccessSize::Word, fc));
            cpu.state.micro.mark_scratch_prefetch_retire();
            StepResult::StepCompleted
        }
    } else {
        cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
        let addr = cpu.state.micro.scratch[0];
        let val = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
        cpu.initiate_bus_cycle(BusCycle::new_write(addr, val, bus_size, fc));
        cpu.state.micro.mark_scratch_prefetch_retire();
        StepResult::StepCompleted
    }
}

// --- Specialized Opcode Forwarders ---

#[inline(always)]
pub fn op_sub_b_absl_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_absw_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_disp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_idx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_pcdisp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_pcidx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_pd_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_b_pi_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_absl_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_absw_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_an_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_disp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_idx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_pcdisp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_pcidx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_pd_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_l_pi_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_absl_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_absw_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_ai_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_an_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_disp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_dn_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_dn_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_dn_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_dn_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_dn_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_dn_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_dn_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_dn_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_dn_to_ea(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_idx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_pcdisp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_pcidx_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_pd_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

#[inline(always)]
pub fn op_sub_w_pi_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_sub_ea_to_dn(cpu, bus)
}

