//! Bcc (Branch Conditionally) instruction handler and condition evaluation
//!
//! Evaluates the 14 conditional branches (BHI, BLS, BCC, BCS, BNE, BEQ, BVC, BVS,
//! BPL, BMI, BGE, BLT, BGT, BLE) using 8-bit short or 16-bit word displacements.
//! Execution time: 10 CPU clocks if branch is taken; 8/12 CPU clocks if branch is not taken.

use crate::core::{Cpu, StepResult};
use crate::instructions::ea::trigger_address_error;
use crate::state::CpuState;
use memory_bus::{BusAccessSize, BusCycle, MemoryBus};

#[inline(always)]
fn prog_fc(cpu: &Cpu) -> u8 {
    if cpu.state.is_supervisor() {
        memory_bus::function_code::SUPERVISOR_PROGRAM
    } else {
        memory_bus::function_code::USER_PROGRAM
    }
}

/// Evaluates Bcc branch condition and returns the target PC if taken
#[inline]
pub fn evaluate_bcc(state: &CpuState, cond: u8, base_pc: u32, displacement: i32) -> Option<u32> {
    if state.eval_condition(cond) {
        let target = base_pc.wrapping_add(displacement as u32) & 0x00FF_FFFF;
        Some(target)
    } else {
        None
    }
}

/// Execution handler for conditional branches `Bcc` (`cond >= 2`)
pub fn op_bcc(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    let opcode = cpu.state.ir;
    let cond = ((opcode >> 8) & 0x0F) as u8;
    let d8 = (opcode & 0x00FF) as i8;
    let base_pc = cpu.state.pc.wrapping_sub(2);
    let fc_p = prog_fc(cpu);

    if d8 != 0 {
        // --- 8-bit Short Displacement Branch ---
        let target = base_pc.wrapping_add(d8 as i32 as u32);
        let taken = cpu.state.eval_condition(cond);

        if taken {
            // Taken Bcc Short: 10 CPU clocks (2 internal + 2 prefetch)
            if (target & 1) != 0 {
                return trigger_address_error(cpu, target, true, true, bus);
            }
            match cpu.state.micro.micro_step {
                0 => {
                    cpu.state.micro.scratch[0] = target;
                    cpu.state.micro.internal_clocks = 2;
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
        } else {
            // Untaken Bcc Short: 8 CPU clocks (4 internal + 4 prefetch)
            match cpu.state.micro.micro_step {
                0 => {
                    cpu.state.micro.internal_clocks = 4;
                    StepResult::StepCompleted
                }
                1 => {
                    cpu.initiate_prefetch();
                    cpu.state.micro.mark_standard_prefetch_retire();
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        }
    } else {
        // --- 16-bit Word Displacement Branch (d8 == 0) ---
        let disp = cpu.state.prefetch[0] as i16 as i32;
        let target = base_pc.wrapping_add(disp as u32);
        let taken = cpu.state.eval_condition(cond);

        if taken {
            // Taken Bcc Word: 10 CPU clocks (2 internal + 2 prefetch)
            if (target & 1) != 0 {
                return trigger_address_error(cpu, target, true, true, bus);
            }
            match cpu.state.micro.micro_step {
                0 => {
                    cpu.state.micro.scratch[0] = target;
                    cpu.state.micro.internal_clocks = 2;
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
        } else {
            // Untaken Bcc Word: 12 CPU clocks (4 internal + 2 prefetch)
            match cpu.state.micro.micro_step {
                0 => {
                    cpu.state.micro.internal_clocks = 4;
                    StepResult::StepCompleted
                }
                1 => {
                    let next_pc = base_pc.wrapping_add(2);
                    cpu.initiate_bus_cycle(BusCycle::new_read(next_pc, BusAccessSize::Word, fc_p));
                    StepResult::StepCompleted
                }
                2 => {
                    let new_ir = cpu.state.micro.last_read;
                    let next_pc = base_pc.wrapping_add(2);
                    cpu.initiate_bus_cycle(BusCycle::new_read(
                        next_pc.wrapping_add(2),
                        BusAccessSize::Word,
                        fc_p,
                    ));
                    cpu.state.micro.mark_target_refill_retire(next_pc, new_ir);
                    StepResult::StepCompleted
                }
                _ => unreachable!(),
            }
        }
    }
}

// --- Specialized Opcode Forwarders ---

pub fn op_bhi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}

pub fn op_bls(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}

pub fn op_bcs(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}

pub fn op_bne(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}

pub fn op_beq(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}

pub fn op_bvc(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}

pub fn op_bvs(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}

pub fn op_bpl(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}

pub fn op_bmi(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}

pub fn op_bge(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}

pub fn op_blt(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}

pub fn op_bgt(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}

pub fn op_ble(cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
    op_bcc(cpu, bus)
}
