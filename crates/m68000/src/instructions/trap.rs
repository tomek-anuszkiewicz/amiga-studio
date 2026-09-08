//! TRAP instruction handler
//!
//! Initiates exception processing for TRAP #0..15 (vectors 32..47).
//! Pushes return PC and SR to the supervisor stack and jumps through vector.
//! Execution time: 34 CPU clocks (17 CCKs).

use crate::core::{Cpu, StepResult};
use crate::micro::types::MicroStep;
use memory_bus::{CckPhase, MemoryBus};

pub static STEPS_TRAP: [MicroStep; 9] = [
    MicroStep {
        step_fn: op_trap,
        alu_fn: None,
        base_clocks: 0,
    };
    9
];

/// Execution handler for `TRAP #<vector>` (34 CPU clocks / 17 CCKs)
pub fn op_trap(cpu: &mut Cpu, bus: &mut MemoryBus) -> Option<StepResult> {
    let opcode = cpu.state.ir;
    let vec = (opcode & 0x000F) as u32;
    let vector_addr = 0x0000_0080 + vec * 4;
    let base_pc = cpu.state.pc.wrapping_sub(2);

    match cpu.state.micro.micro_step {
        0 => {
            // 4 internal clocks: 2 consumed now, 2 remaining
            cpu.state.micro.internal_clocks = 2;
            let return_pc = base_pc;
            let old_sr = cpu.state.sr;
            // Switch to supervisor mode (S=1, T=0)
            cpu.state.set_supervisor(true);
            cpu.state.sr &= !0x8000;
            cpu.state.micro.scratch[0] = return_pc;
            cpu.state.micro.scratch[1] = old_sr as u32;
            cpu.state.micro.scratch[2] = vector_addr;
            cpu.instruction_clocks = cpu.instruction_clocks.wrapping_add(2);
            Some(StepResult::StepCompleted)
        }
        1 => {
            // Write return PC low word to SP - 2
            let sp = cpu.state.read_a(7);
            let lo = (cpu.state.micro.scratch[0] & 0xFFFF) as u16;
            let res = cpu.step_write_word_at(bus, sp.wrapping_sub(2), lo);
            if res == StepResult::StepCompleted && cpu.state.micro.phase == CckPhase::Cck1 {
                cpu.state.micro.micro_step = 2;
            }
            Some(res)
        }
        2 => {
            // Write old SR to SP - 6
            let sp = cpu.state.read_a(7);
            let sr = (cpu.state.micro.scratch[1] & 0xFFFF) as u16;
            let res = cpu.step_write_word_at(bus, sp.wrapping_sub(6), sr);
            if res == StepResult::StepCompleted && cpu.state.micro.phase == CckPhase::Cck1 {
                cpu.state.micro.micro_step = 3;
            }
            Some(res)
        }
        3 => {
            // Write return PC high word to SP - 4 and update SP on completion
            let sp = cpu.state.read_a(7);
            let hi = ((cpu.state.micro.scratch[0] >> 16) & 0xFFFF) as u16;
            let res = cpu.step_write_word_at(bus, sp.wrapping_sub(4), hi);
            if res == StepResult::StepCompleted && cpu.state.micro.phase == CckPhase::Cck1 {
                cpu.state.write_a(7, sp.wrapping_sub(6));
                cpu.state.micro.micro_step = 4;
            }
            Some(res)
        }
        4 => {
            // Read vector high word from vector_addr
            let vec_addr = cpu.state.micro.scratch[2];
            let res = cpu.step_read_word_at(bus, vec_addr);
            if res == StepResult::StepCompleted && cpu.state.micro.phase == CckPhase::Cck1 {
                cpu.state.micro.scratch[0] = (cpu.state.micro.last_read as u32) << 16;
                cpu.state.micro.micro_step = 5;
            }
            Some(res)
        }
        5 => {
            // Read vector low word from vector_addr + 2
            let vec_addr = cpu.state.micro.scratch[2];
            let res = cpu.step_read_word_at(bus, vec_addr.wrapping_add(2));
            if res == StepResult::StepCompleted && cpu.state.micro.phase == CckPhase::Cck1 {
                let lo = cpu.state.micro.last_read as u32;
                let target = (cpu.state.micro.scratch[0] | lo) & 0x00FF_FFFF;
                if (target & 1) != 0 {
                    return Some(cpu.trigger_address_error_step(target, true, true, bus));
                }
                cpu.state.micro.scratch[0] = target;
                cpu.state.micro.micro_step = 6;
            }
            Some(res)
        }
        6 => {
            // Read target opcode
            let target = cpu.state.micro.scratch[0];
            let res = cpu.step_read_prog_word_at(bus, target);
            if res == StepResult::StepCompleted && cpu.state.micro.phase == CckPhase::Cck1 {
                cpu.state.micro.scratch_prefetch = cpu.state.micro.last_read;
                cpu.state.micro.internal_clocks = 2;
                cpu.state.micro.micro_step = 7;
            }
            Some(res)
        }
        7 => {
            // 2 internal clocks: handled by core step_cck when internal_clocks > 0
            cpu.state.micro.micro_step = 8;
            Some(StepResult::StepCompleted)
        }
        8 => {
            // Read target + 2 word and refill prefetch pipeline
            let target = cpu.state.micro.scratch[0];
            let new_ir = cpu.state.micro.scratch_prefetch;
            let res = cpu.step_read_prog_word_at(bus, target.wrapping_add(2));
            if res == StepResult::StepCompleted && cpu.state.micro.phase == CckPhase::Cck1 {
                let target_prefetch = cpu.state.micro.last_read;
                return Some(cpu.retire_target_refill(target, target_prefetch, new_ir));
            }
            Some(res)
        }
        _ => unreachable!(),
    }
}
