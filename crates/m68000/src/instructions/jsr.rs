//! JSR (Jump to Subroutine) instruction handler
//!
//! Pushes the long return address onto the stack and jumps to an effective address
//! using control addressing modes: `(An)`, `(d16, An)`, `(d8, An, Xn)`, `(xxx).w`, `(xxx).l`,
//! `(d16, PC)`, `(d8, PC, Xn)`.
//! Execution time: 16 to 22 CPU clocks depending on addressing mode.

use crate::core::{Cpu, StepResult};
use crate::instructions::ea::{
    decode_ea_index, trigger_address_error, EA_AI, EA_AL, EA_IX, EA_IXPC,
};
use crate::instructions::jmp::resolve_control_target;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

#[inline(always)]
fn prog_fc(cpu: &Cpu) -> u8 {
    if cpu.state.is_supervisor() {
        memory_bus::function_code::SUPERVISOR_PROGRAM
    } else {
        memory_bus::function_code::USER_PROGRAM
    }
}

#[inline(always)]
fn data_fc(cpu: &Cpu) -> u8 {
    if cpu.state.is_supervisor() {
        memory_bus::function_code::SUPERVISOR_DATA
    } else {
        memory_bus::function_code::USER_DATA
    }
}

/// Execution handler for `JSR <ea>`
pub fn op_jsr(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let reg = (cpu.state.ir & 7) as usize;
    let m = decode_ea_index(((cpu.state.ir >> 3) & 7) as u8, (cpu.state.ir & 7) as u8);
    let base_pc = cpu.state.pc.wrapping_sub(2);
    let fc_p = prog_fc(cpu);
    let fc_d = data_fc(cpu);

    let (return_pc, target, internal_clocks) = match m {
        EA_AI => (base_pc, resolve_control_target(cpu, m, reg, base_pc), 0),
        EA_AL => {
            // Evaluated in step 0 & 1
            (base_pc.wrapping_add(4), 0, 0)
        }
        EA_IX | EA_IXPC => (
            base_pc.wrapping_add(2),
            resolve_control_target(cpu, m, reg, base_pc),
            6,
        ),
        _ => (
            base_pc.wrapping_add(2),
            resolve_control_target(cpu, m, reg, base_pc),
            2,
        ),
    };

    if m == EA_AL {
        // JSR (xxx).L: 20 CPU clocks (4 extension read + 4 prefetch target + 4 SP write + 4 SP+2 write + 4 prefetch target+2)
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
                cpu.state.micro.scratch[1] = return_pc;
                let sp = cpu.state.a7().wrapping_sub(4);
                cpu.state.set_a7(sp);
                if (sp & 1) != 0 {
                    return trigger_address_error(cpu, sp, false, false, bus);
                }
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            2 => {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
                let sp = cpu.state.a7();
                let hi = ((cpu.state.micro.scratch[1] >> 16) & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp, hi, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            3 => {
                let sp_low = cpu.state.a7().wrapping_add(2);
                let lo = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp_low, lo, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            4 => {
                let target = cpu.state.micro.scratch[0];
                let new_ir = cpu.state.micro.scratch_prefetch;
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
        // JSR (An): 16 CPU clocks
        if (target & 1) != 0 {
            return trigger_address_error(cpu, target, true, true, bus);
        }
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.scratch[0] = target;
                cpu.state.micro.scratch[1] = return_pc;
                let sp = cpu.state.a7().wrapping_sub(4);
                cpu.state.set_a7(sp);
                if (sp & 1) != 0 {
                    return trigger_address_error(cpu, sp, false, false, bus);
                }
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            1 => {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
                let sp = cpu.state.a7();
                let hi = ((cpu.state.micro.scratch[1] >> 16) & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp, hi, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            2 => {
                let sp_low = cpu.state.a7().wrapping_add(2);
                let lo = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp_low, lo, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            3 => {
                let target = cpu.state.micro.scratch[0];
                let new_ir = cpu.state.micro.scratch_prefetch;
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
        // JSR with extension word: 18 or 22 CPU clocks
        if (target & 1) != 0 {
            return trigger_address_error(cpu, target, true, true, bus);
        }
        match cpu.state.micro.micro_step {
            0 => {
                cpu.state.micro.scratch[0] = target;
                cpu.state.micro.scratch[1] = return_pc;
                let sp = cpu.state.a7().wrapping_sub(4);
                cpu.state.set_a7(sp);
                if (sp & 1) != 0 {
                    return trigger_address_error(cpu, sp, false, false, bus);
                }
                cpu.state.micro.internal_clocks = internal_clocks;
                StepResult::StepCompleted
            }
            1 => {
                let target = cpu.state.micro.scratch[0];
                cpu.initiate_bus_cycle(BusCycle::new_read(target, BusAccessSize::Word, fc_p));
                StepResult::StepCompleted
            }
            2 => {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
                let sp = cpu.state.a7();
                let hi = ((cpu.state.micro.scratch[1] >> 16) & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp, hi, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            3 => {
                let sp_low = cpu.state.a7().wrapping_add(2);
                let lo = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
                cpu.initiate_bus_cycle(BusCycle::new_write(sp_low, lo, BusAccessSize::Word, fc_d));
                StepResult::StepCompleted
            }
            4 => {
                let target = cpu.state.micro.scratch[0];
                let new_ir = cpu.state.micro.scratch_prefetch;
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
