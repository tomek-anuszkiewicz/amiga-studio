//! Stepping primitives and Headless Debugger Engine

use crate::breakpoints::BreakpointManager;
use crate::disassembler::{disassemble, Disassembly};
use crate::trace::TraceRingBuffer;
use m68000::{Cpu, StepResult};
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

    /// Steps exactly one M68000 instruction, recording to trace history
    pub fn step_instruction(&mut self, cpu: &mut Cpu, bus: &mut MemoryBus) -> StepResult {
        let pc = cpu.state.pc.wrapping_sub(4); // Address of opcode currently in IR
        let (disasm, _) = self.disassemble_at(pc, bus);

        // Record trace entry before stepping
        self.trace.record(
            self.current_cck,
            pc,
            cpu.state.ir,
            disasm.format_line(),
            cpu.state.clone(),
        );

        let res = cpu.step_instruction(bus);
        self.current_cck = self.current_cck.wrapping_add(8); // Approximation: 8 CCK per instruction
        res
    }

    /// Free-runs execution until a breakpoint is hit, CPU halts, or max_instructions is reached
    pub fn run_until_breakpoint(
        &mut self,
        cpu: &mut Cpu,
        bus: &mut MemoryBus,
        max_instructions: usize,
    ) -> StepResult {
        for _ in 0..max_instructions {
            let next_pc = cpu.state.pc.wrapping_sub(4);
            if self.breakpoints.check_pc(next_pc) {
                return StepResult::StepCompleted;
            }
            let res = self.step_instruction(cpu, bus);
            if res == StepResult::Halted || res == StepResult::Stopped {
                return res;
            }
        }
        StepResult::StepCompleted
    }
}
