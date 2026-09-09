//! TRAP instruction handler
//!
//! Initiates exception processing for TRAP #0..15 (vectors 32..47).
//! Pushes return PC and SR to the supervisor stack and jumps through vector.
//! Execution time: 34 CPU clocks (17 CCKs).

use crate::core::Cpu;
use crate::micro::types::MicroStep;
use crate::state::CpuState;
use memory_bus::{BusResult, CckPhase, MemoryBus};

/// Initial setup for TRAP exception: saves old SR, switches to supervisor, records 4 internal clocks
pub fn alu_trap_init(state: &mut CpuState, _reg_src: u8, _reg_dst: u8) {
    let opcode = state.ir;
    let vec = (opcode & 0x000F) as u32;
    let vector_addr = 0x0000_0080 + vec * 4;
    let base_pc = state.pc.wrapping_sub(2);
    let return_pc = base_pc;
    let old_sr = state.sr;
    // Switch to supervisor mode (S=1, T=0)
    state.set_supervisor(true);
    state.sr &= !0x8000;
    state.micro.source = return_pc;
    state.micro.destination = old_sr as u32;
    state.micro.ea_addr = vector_addr;
    state.micro.record_internal_clocks(4);
}

pub static STEPS_TRAP: [MicroStep; 9] = [
    MicroStep {
        step_fn: Cpu::step_alu,
        alu_fn: Some(alu_trap_init),
        base_clocks: 0,
    },
    MicroStep {
        step_fn: op_trap,
        alu_fn: None,
        base_clocks: 4,
    },
    MicroStep {
        step_fn: op_trap,
        alu_fn: None,
        base_clocks: 4,
    },
    MicroStep {
        step_fn: op_trap,
        alu_fn: None,
        base_clocks: 4,
    },
    MicroStep {
        step_fn: op_trap,
        alu_fn: None,
        base_clocks: 4,
    },
    MicroStep {
        step_fn: op_trap,
        alu_fn: None,
        base_clocks: 4,
    },
    MicroStep {
        step_fn: op_trap,
        alu_fn: None,
        base_clocks: 4,
    },
    MicroStep {
        step_fn: Cpu::step_alu,
        alu_fn: None,
        base_clocks: 2,
    },
    MicroStep {
        step_fn: op_trap,
        alu_fn: None,
        base_clocks: 4,
    },
];

/// Execution handler for `TRAP #<vector>` (34 CPU clocks / 17 CCKs)
pub fn op_trap(cpu: &mut Cpu, bus: &mut MemoryBus) -> BusResult<()> {
    match cpu.state.micro.micro_step {
        1 => {
            // Write return PC low word to SP - 2
            let sp = cpu.state.read_a(7);
            let lo = (cpu.state.micro.source & 0xFFFF) as u16;
            cpu.step_write_word_at(bus, sp.wrapping_sub(2), lo)
        }
        2 => {
            // Write old SR to SP - 6
            let sp = cpu.state.read_a(7);
            let sr = (cpu.state.micro.destination & 0xFFFF) as u16;
            cpu.step_write_word_at(bus, sp.wrapping_sub(6), sr)
        }
        3 => {
            // Write return PC high word to SP - 4 and update SP on completion
            let sp = cpu.state.read_a(7);
            let hi = ((cpu.state.micro.source >> 16) & 0xFFFF) as u16;
            let res = cpu.step_write_word_at(bus, sp.wrapping_sub(4), hi);
            if res == BusResult::Ready(()) && cpu.state.micro.phase == CckPhase::Cck1 {
                cpu.state.write_a(7, sp.wrapping_sub(6));
            }
            res
        }
        4 => {
            // Read vector high word from vector_addr
            let vec_addr = cpu.state.micro.ea_addr;
            let res = cpu.step_read_word_at(bus, vec_addr);
            if res == BusResult::Ready(()) && cpu.state.micro.phase == CckPhase::Cck1 {
                cpu.state.micro.ea_high = (cpu.state.micro.source & 0xFFFF) << 16;
            }
            res
        }
        5 => {
            // Read vector low word from vector_addr + 2
            let vec_addr = cpu.state.micro.ea_addr;
            let res = cpu.step_read_word_at(bus, vec_addr.wrapping_add(2));
            if res == BusResult::Ready(()) && cpu.state.micro.phase == CckPhase::Cck1 {
                let lo = cpu.state.micro.source & 0xFFFF;
                let target = (cpu.state.micro.ea_high | lo) & 0x00FF_FFFF;
                if (target & 1) != 0 {
                    cpu.trigger_address_error_step(target, true, true, bus);
                    return BusResult::Ready(());
                }
                cpu.state.micro.ea_addr = target;
            }
            res
        }
        6 => {
            // Read target opcode
            let target = cpu.state.micro.ea_addr;
            let res = cpu.step_read_prog_word_at(bus, target);
            if res == BusResult::Ready(()) && cpu.state.micro.phase == CckPhase::Cck1 {
                cpu.state.micro.irc = cpu.state.micro.source as u16;
            }
            res
        }
        8 => {
            // Read target + 2 word and refill prefetch pipeline
            let target = cpu.state.micro.ea_addr;
            let res = cpu.step_read_prog_word_at(bus, target.wrapping_add(2));
            if res == BusResult::Ready(()) && cpu.state.micro.phase == CckPhase::Cck1 {
                cpu.state.prefetch[0] = cpu.state.micro.source as u16;
                cpu.state.micro.target_refill = true;
            }
            res
        }
        _ => unreachable!(),
    }
}
