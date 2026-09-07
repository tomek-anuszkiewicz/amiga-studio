//! JMP (Jump) instruction handler
//!
//! Unconditional program jump to an effective address using control addressing modes:
//! `(An)`, `(d16, An)`, `(d8, An, Xn)`, `(xxx).w`, `(xxx).l`, `(d16, PC)`, `(d8, PC, Xn)`.

use crate::core::{Cpu, StepResult};
use crate::instructions::ea::{
    decode_ea_index, trigger_address_error, EA_AI, EA_AL, EA_AW, EA_DI, EA_DIPC, EA_IX, EA_IXPC,
};
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

#[inline(always)]
fn prog_fc(cpu: &Cpu) -> u8 {
    if cpu.state.is_supervisor() {
        memory_bus::function_code::SUPERVISOR_PROGRAM
    } else {
        memory_bus::function_code::USER_PROGRAM
    }
}

/// Calculates target address for control addressing modes
#[inline(always)]
pub fn resolve_control_target(cpu: &Cpu, mode: u8, reg: usize, base_pc: u32) -> u32 {
    match mode {
        EA_AI => cpu.state.read_a(reg),
        EA_DI => {
            let disp = cpu.state.prefetch[0] as i16 as i32;
            cpu.state.read_a(reg).wrapping_add(disp as u32)
        }
        EA_IX => {
            let ext = cpu.state.prefetch[0];
            let base = cpu.state.read_a(reg);
            let disp = (ext & 0xFF) as i8 as i32;
            let idx_reg = ((ext >> 12) & 7) as usize;
            let is_a = (ext & 0x8000) != 0;
            let is_long = (ext & 0x0800) != 0;
            let idx_val = if is_a {
                cpu.state.read_a(idx_reg)
            } else {
                cpu.state.d_long(idx_reg)
            };
            let idx_ext = if is_long {
                idx_val as i32
            } else {
                (idx_val as i16) as i32
            };
            base.wrapping_add(disp as u32).wrapping_add(idx_ext as u32)
        }
        EA_AW => cpu.state.prefetch[0] as i16 as i32 as u32,
        EA_DIPC => {
            let disp = cpu.state.prefetch[0] as i16 as i32;
            base_pc.wrapping_add(disp as u32)
        }
        EA_IXPC => {
            let ext = cpu.state.prefetch[0];
            let base = base_pc;
            let disp = (ext & 0xFF) as i8 as i32;
            let idx_reg = ((ext >> 12) & 7) as usize;
            let is_a = (ext & 0x8000) != 0;
            let is_long = (ext & 0x0800) != 0;
            let idx_val = if is_a {
                cpu.state.read_a(idx_reg)
            } else {
                cpu.state.d_long(idx_reg)
            };
            let idx_ext = if is_long {
                idx_val as i32
            } else {
                (idx_val as i16) as i32
            };
            base.wrapping_add(disp as u32).wrapping_add(idx_ext as u32)
        }
        _ => 0,
    }
}

/// Execution handler for `JMP <ea>`
pub fn op_jmp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let reg = (cpu.state.ir & 7) as usize;
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    let base_pc = cpu.state.pc.wrapping_sub(2);
    let fc_p = prog_fc(cpu);

    if m == EA_AL {
        // JMP (xxx).L: 12 CPU clocks (4 extension read + 2 prefetch cycles)
        match cpu.state.micro.micro_step {
            0 => {
                let ext_pc = base_pc.wrapping_add(2);
                cpu.initiate_bus_cycle(BusCycle::new_read(ext_pc, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            1 => {
                let hi = (cpu.state.prefetch[0] as u32) << 16;
                let lo = cpu.state.micro.last_read as u32;
                let target = hi | lo;
                if (target & 1) != 0 {
                    return trigger_address_error(cpu, target, true, true, bus);
                }
                cpu.state.micro.scratch[0] = target;
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            2 => {
                let new_ir = cpu.state.micro.last_read;
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(
                    target.wrapping_add(2),
                    BusAccessSize::Word,
                    fc_p,
                ));
                cpu.state.micro.mark_target_refill_retire(target, new_ir);
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else if m == EA_AI {
        // JMP (An): 8 CPU clocks (2 prefetch cycles)
        let target = resolve_control_target(cpu, m, reg, base_pc);
        if (target & 1) != 0 {
            return trigger_address_error(cpu, target, true, true, bus);
        }
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.scratch[0] = target;
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            1 => {
                let new_ir = cpu.state.micro.last_read;
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(
                    target.wrapping_add(2),
                    BusAccessSize::Word,
                    fc_p,
                ));
                cpu.state.micro.mark_target_refill_retire(target, new_ir);
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    } else {
        // Modes with 1 extension word: (d16,An), (xxx).w, (d16,PC) -> 10 clocks (2 internal + 2 prefetch)
        // (d8,An,Xn), (d8,PC,Xn) -> 14 clocks (6 internal + 2 prefetch)
        let target = resolve_control_target(cpu, m, reg, base_pc);
        if (target & 1) != 0 {
            return trigger_address_error(cpu, target, true, true, bus);
        }
        let internal_clocks = if m == EA_IX || m == EA_IXPC { 6 } else { 2 };
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.scratch[0] = target;
                cpu.state.micro.internal_clocks = internal_clocks;
                StepResult::StepCompleted
            }
            1 => {
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            2 => {
                let new_ir = cpu.state.micro.last_read;
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(
                    target.wrapping_add(2),
                    BusAccessSize::Word,
                    fc_p,
                ));
                cpu.state.micro.mark_target_refill_retire(target, new_ir);
                StepResult::StepCompleted
            }
            _ => unreachable!(),
        }
    }
}

// --- Specialized Opcode Forwarders ---

pub fn op_jmp_absl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_absw(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_ai(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_an(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_disp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_dn(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_idx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_imm(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_pcdisp(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_pcidx(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_pd(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

pub fn op_jmp_pi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_jmp(cpu, bus)
}

