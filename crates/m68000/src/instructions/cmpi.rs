//! CMPI (Compare Immediate) instruction handlers
//!
//! Compares an immediate operand with an effective address operand: `CMPI #<data>, <ea>`.
//! Evaluates (<ea> - immediate) and updates N, Z, V, and C flags.
//! Destination operand is not modified. Extend (X) flag is unaffected.

use crate::addressing::Size;
use crate::core::{Cpu, StepResult};
use crate::instructions::cmp::execute_cmp;
use crate::instructions::ea::{
    decode_ea_index, read_ea_operand, size_from_const, EA_DN, SIZE_BYTE,
    SIZE_LONG,
};
use memory_bus::MemoryBus;

/// Execution handler for `CMPI #<data>, <ea>`
pub fn op_cmpi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let ir = cpu.state.ir;
    let s = ((ir >> 6) & 3) as u8;
    let m = decode_ea_index(((ir >> 3) & 7) as u8, (ir & 7) as u8);
    let size = size_from_const(s);

    if m == EA_DN {
        if s == SIZE_LONG {
            // Long immediate CMPI: 14 clocks (3 prefetch cycles + 2 internal clocks)
            match cpu.state.micro.micro_step {
                0 => {
                    let hi = cpu.state.prefetch[0];
                    cpu.state.micro.scratch[0] = (hi as u32) << 16;
                    cpu.initiate_prefetch();
                    cpu.state.pc = cpu.state.pc.wrapping_add(2);
                    StepResult::StepCompleted
                }
                1 => {
                    let lo = cpu.state.micro.last_read;
                    cpu.state.micro.scratch[0] |= lo as u32;
                    cpu.initiate_prefetch();
                    cpu.state.pc = cpu.state.pc.wrapping_add(2);
                    StepResult::StepCompleted
                }
                2 => {
                    cpu.state.prefetch[0] = cpu.state.micro.last_read;
                    cpu.record_internal_clocks(2);
                    StepResult::StepCompleted
                }
                3 => {
                    let imm = cpu.state.micro.scratch[0];
                    let reg_d = (cpu.state.ir & 7) as usize;
                    let d = cpu.state.d_long(reg_d);
                    execute_cmp(&mut cpu.state, imm, d, Size::Long);
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        } else {
            // Byte / Word immediate CMPI: 8 clocks (2 prefetch cycles)
            match cpu.state.micro.micro_step {
                0 => {
                    let imm = match s {
                        SIZE_BYTE => (cpu.state.prefetch[0] & 0xFF) as u32,
                        _ => cpu.state.prefetch[0] as u32,
                    };
                    cpu.state.micro.scratch[0] = imm;
                    cpu.initiate_prefetch();
                    cpu.state.pc = cpu.state.pc.wrapping_add(2);
                    StepResult::StepCompleted
                }
                1 => {
                    cpu.state.prefetch[0] = cpu.state.micro.last_read;
                    let imm = cpu.state.micro.scratch[0];
                    let reg_d = (cpu.state.ir & 7) as usize;
                    let d = cpu.state.d_long(reg_d);
                    execute_cmp(&mut cpu.state, imm, d, size);
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        }
    } else {
        // Memory destination CMPI: Strictly read-only, zero writeback cycles!
        if s == SIZE_LONG {
            let step = cpu.state.micro.micro_step;
            if step == 0 {
                let hi = cpu.state.prefetch[0];
                cpu.state.micro.scratch[3] = (hi as u32) << 16;
                cpu.initiate_prefetch();
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                return StepResult::StepCompleted;
            } else if step == 1 {
                let lo = cpu.state.micro.last_read;
                cpu.state.micro.scratch[3] |= lo as u32;
                cpu.initiate_prefetch();
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                return StepResult::StepCompleted;
            }

            let ea_step = step - 2;
            if ea_step == 0 {
                cpu.state.prefetch[0] = cpu.state.micro.last_read;
            }
            let mem_val = match read_ea_operand(cpu, bus, ea_step, s, m) {
                Ok(v) => v,
                Err(res) => return res,
            };

            let imm = cpu.state.micro.scratch[3];
            execute_cmp(&mut cpu.state, imm, mem_val, Size::Long);
            cpu.initiate_prefetch();
            cpu.state.micro.mark_standard_prefetch_retire();
            StepResult::StepCompleted
        } else {
            let step = cpu.state.micro.micro_step;
            if step == 0 {
                let imm = match s {
                    SIZE_BYTE => (cpu.state.prefetch[0] & 0xFF) as u32,
                    _ => cpu.state.prefetch[0] as u32,
                };
                cpu.state.micro.scratch[3] = imm;
                cpu.initiate_prefetch();
                cpu.state.pc = cpu.state.pc.wrapping_add(2);
                return StepResult::StepCompleted;
            }

            let ea_step = step - 1;
            if ea_step == 0 {
                cpu.state.prefetch[0] = cpu.state.micro.last_read;
            }
            let mem_val = match read_ea_operand(cpu, bus, ea_step, s, m) {
                Ok(v) => v,
                Err(res) => return res,
            };

            let imm = cpu.state.micro.scratch[3];
            execute_cmp(&mut cpu.state, imm, mem_val, size);
            cpu.initiate_prefetch();
            cpu.state.micro.mark_standard_prefetch_retire();
            StepResult::StepCompleted
        }
    }
}

// --- Specialized Opcode Forwarders ---

pub fn op_cmpi_b_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_b_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_b_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_b_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_b_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_b_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_b_imm_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_b_imm_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_b_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_b_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_l_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_l_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_l_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_l_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_l_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_l_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_l_imm_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_l_imm_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_l_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_l_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_w_imm_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_w_imm_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_w_imm_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_w_imm_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_w_imm_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_w_imm_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_w_imm_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_w_imm_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_w_imm_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

pub fn op_cmpi_w_imm_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_cmpi(cpu, bus)
}

