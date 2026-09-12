//! Stepping primitives and Headless Debugger Engine

use crate::breakpoints::BreakpointManager;
use crate::disassembler::{disassemble, Disassembly};
use crate::temporal::TemporalHistory;
use crate::trace::TraceRingBuffer;
use m68000::Cpu;
use memory_bus::MemoryBus;

/// Fine-grained execution stepping modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    /// Step forward exactly 1 Color Clock (CCK, ~280 ns)
    StepCck,
    /// Step forward until the current M68000 instruction completes
    StepInstruction,
    /// Continue execution until a breakpoint is hit or CPU halts
    Continue,
}

/// Headless Debugger Backend
#[derive(Debug, Clone, Default)]
pub struct Debugger {
    pub breakpoints: BreakpointManager,
    pub trace: TraceRingBuffer,
    pub current_cck: u64,
}

impl Debugger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Disassembles one instruction at the specified address from memory without side effects
    pub fn disassemble_at(&self, addr: u32, bus: &MemoryBus) -> (Disassembly, u32) {
        disassemble(addr, |a| bus.read_word_debug(a))
    }

    /// Steps exactly one M68000 instruction, recording to trace history, and returns instruction clocks
    pub fn step_instruction(&mut self, cpu: &mut Cpu, bus: &mut MemoryBus) -> u32 {
        let pc = cpu.state.instruction_pc; // Address of opcode currently in IR
        let (disasm, _) = self.disassemble_at(pc, bus);

        // Record trace entry before stepping
        self.trace.record(
            self.current_cck,
            pc,
            cpu.state.ir,
            disasm.format_line(),
            cpu.state.clone(),
        );

        let clocks = cpu.step_instruction(bus);
        self.current_cck = self.current_cck.wrapping_add((clocks as u64) / 2);
        clocks
    }

    /// Free-runs execution until a breakpoint is hit, CPU halts/stops, or max_instructions is reached.
    /// Returns the number of instructions executed.
    pub fn run_until_breakpoint(
        &mut self,
        cpu: &mut Cpu,
        bus: &mut MemoryBus,
        max_instructions: usize,
    ) -> usize {
        for steps in 0..max_instructions {
            let next_pc = cpu.state.instruction_pc;
            if self.breakpoints.check_pc_with_state(next_pc, &cpu.state) {
                return steps;
            }
            if cpu.state.halted || cpu.state.stopped {
                return steps;
            }
            self.step_instruction(cpu, bus);
            if cpu.state.halted || cpu.state.stopped {
                return steps + 1;
            }
        }
        max_instructions
    }

    /// Free-runs execution while optionally recording states into the high-capacity temporal history buffer
    pub fn run_until_breakpoint_with_temporal(
        &mut self,
        cpu: &mut Cpu,
        bus: &mut MemoryBus,
        temporal: &mut TemporalHistory,
        max_instructions: usize,
    ) -> usize {
        for steps in 0..max_instructions {
            let next_pc = cpu.state.instruction_pc;
            if self.breakpoints.check_pc_with_state(next_pc, &cpu.state) {
                return steps;
            }
            if cpu.state.halted || cpu.state.stopped {
                return steps;
            }
            if temporal.is_recording() {
                temporal.record(self.current_cck, next_pc, cpu.state.ir, cpu.state.clone());
            }
            self.step_instruction(cpu, bus);
            if cpu.state.halted || cpu.state.stopped {
                return steps + 1;
            }
        }
        max_instructions
    }
}
