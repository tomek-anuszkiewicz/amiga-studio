//! Benchmark Program Execution Tracer & Audit Log Generator
//!
//! Executes a single pass of a synthesized benchmark program, tracking instruction
//! disassembly, Color Clock cycle counts, register mutations, and memory writes
//! at every step to produce deterministic, human- and LLM-readable audit traces.

use std::fs;
use std::path::Path;

use cpu::Cpu;
use disassembler::disassemble;
use physical_memory::PhysicalMemory;

use super::builder::{
    BenchmarkProgram, BenchmarkProgramBuilder, BENCH_EXIT_PC, BENCH_RAM_BUFFER_A1,
};
use super::catalog::BenchmarkSpec;

/// Individual instruction step record captured during execution tracing
#[derive(Debug, Clone)]
pub struct StepTraceRecord {
    pub step_index: usize,
    pub pc_before: u32,
    pub disassembly: String,
    pub guest_cycles: u32,
    pub reg_deltas: Vec<String>,
    pub mem_deltas: Vec<String>,
}

/// Full execution trace and audit snapshot of a synthesized benchmark program
#[derive(Debug, Clone)]
pub struct BenchmarkTraceLog {
    pub spec: BenchmarkSpec,
    pub unroll_k: usize,
    pub initial_pc: u32,
    pub initial_sr: u16,
    pub initial_d: [u32; 8],
    pub initial_a: [u32; 8],
    pub steps: Vec<StepTraceRecord>,
    pub final_d: [u32; 8],
    pub final_a: [u32; 8],
    pub final_sr: u16,
    pub final_pc: u32,
    pub total_guest_cycles: u64,
    pub terminated_cleanly: bool,
    pub timed_out: bool,
}

impl BenchmarkTraceLog {
    /// Formats the execution trace as a structured, human- and LLM-readable audit report
    pub fn format_text(&self) -> String {
        let mut out = String::new();
        out.push_str(
            "================================================================================\n",
        );
        out.push_str(&format!(
            "M68000 BENCHMARK EXECUTION AUDIT TRACE: [{}] {}\n",
            self.spec.id, self.spec.representative_syntax
        ));
        out.push_str(&format!(
            "Category: {:?} | Mode: {:?} | Expected Amiga CCK: {}\n",
            self.spec.category, self.spec.addressing_mode, self.spec.amiga_cck
        ));
        out.push_str(&format!(
            "Unroll Factor: K = {} | Entry PC: 0x{:06X} | Exit Sentinel: 0x{:06X}\n",
            self.unroll_k, self.initial_pc, BENCH_EXIT_PC
        ));
        out.push_str(
            "--------------------------------------------------------------------------------\n",
        );
        out.push_str("INITIAL CPU REGISTERS:\n");
        for i in 0..4 {
            out.push_str(&format!(
                "  D{}: 0x{:08X}   D{}: 0x{:08X}   A{}: 0x{:08X}   A{}: 0x{:08X}\n",
                i,
                self.initial_d[i],
                i + 4,
                self.initial_d[i + 4],
                i,
                self.initial_a[i],
                i + 4,
                self.initial_a[i + 4]
            ));
        }
        out.push_str(&format!(
            "  SR: 0x{:04X} [S={}, IPL={}]   SSP (A7): 0x{:08X}\n",
            self.initial_sr,
            (self.initial_sr & 0x2000) != 0,
            (self.initial_sr >> 8) & 7,
            self.initial_a[7]
        ));
        out.push_str(
            "--------------------------------------------------------------------------------\n",
        );
        out.push_str("STEP-BY-STEP EXECUTION LOG:\n");

        for step in &self.steps {
            let reg_str = if step.reg_deltas.is_empty() {
                "none".to_string()
            } else {
                step.reg_deltas.join(", ")
            };
            let mem_str = if step.mem_deltas.is_empty() {
                String::new()
            } else {
                format!(" | Mem: {}", step.mem_deltas.join(", "))
            };
            out.push_str(&format!(
                "[{:03}] {} | {:>2} CCK | ΔRegs: {}{}\n",
                step.step_index, step.disassembly, step.guest_cycles, reg_str, mem_str
            ));
        }

        out.push_str(
            "--------------------------------------------------------------------------------\n",
        );
        out.push_str("EXECUTION SUMMARY & VERIFICATION:\n");
        out.push_str(&format!("  Total Steps Executed:   {}\n", self.steps.len()));
        out.push_str(&format!(
            "  Total Guest Cycles:     {} CCK\n",
            self.total_guest_cycles
        ));
        out.push_str(&format!(
            "  Terminated Cleanly:     {} (Final PC: 0x{:06X})\n",
            if self.terminated_cleanly { "YES" } else { "NO" },
            self.final_pc
        ));
        out.push_str(&format!(
            "  Timed Out / Infinite:   {}\n",
            if self.timed_out {
                "YES (ABORTED)"
            } else {
                "NO"
            }
        ));
        out.push_str("FINAL CPU REGISTERS:\n");
        for i in 0..4 {
            out.push_str(&format!(
                "  D{}: 0x{:08X}   D{}: 0x{:08X}   A{}: 0x{:08X}   A{}: 0x{:08X}\n",
                i,
                self.final_d[i],
                i + 4,
                self.final_d[i + 4],
                i,
                self.final_a[i],
                i + 4,
                self.final_a[i + 4]
            ));
        }
        out.push_str(&format!(
            "  SR: 0x{:04X} [S={}, IPL={}]   SSP (A7): 0x{:08X}\n",
            self.final_sr,
            (self.final_sr & 0x2000) != 0,
            (self.final_sr >> 8) & 7,
            self.final_a[7]
        ));
        out.push_str(
            "================================================================================\n",
        );

        out
    }
}

/// Executes a single pass of the program, tracking register and memory deltas
pub fn trace_program(program: &BenchmarkProgram, max_steps: usize) -> BenchmarkTraceLog {
    let mut bus = PhysicalMemory::new();
    let mut cpu = Cpu::new();
    program.inject_into(&mut cpu, &mut bus);

    let initial_pc = program.entry_pc;
    let initial_sr = program.initial_sr;
    let initial_d = program.initial_d;
    let initial_a = program.initial_a;

    let mut steps = Vec::new();
    let mut total_cycles: u64 = 0;
    let mut step_count = 0;
    let mut timed_out = false;

    while !cpu.state.halted && !cpu.state.stopped && cpu.state.instruction_pc != program.exit_pc {
        let pc_before = cpu.state.instruction_pc;
        let (disasm, _) = disassemble(pc_before, |a| bus.read_word_debug(a));
        let disassembly = disasm.format_line();

        let cpu_before = cpu.clone();

        let cycles = cpu.step_instruction(&mut bus);
        if cycles == 0 {
            break;
        }
        total_cycles = total_cycles.wrapping_add(cycles as u64);

        let mut reg_deltas = Vec::new();

        // Check Data Registers
        let d_now = cpu.state.d_regs();
        let d_before = cpu_before.state.d_regs();
        for i in 0..8 {
            if d_now[i] != d_before[i] {
                reg_deltas.push(format!(
                    "D{}: 0x{:08X} -> 0x{:08X}",
                    i, d_before[i], d_now[i]
                ));
            }
        }

        // Check Address Registers
        let a_now = cpu.state.a_regs();
        let a_before = cpu_before.state.a_regs();
        for i in 0..8 {
            if a_now[i] != a_before[i] {
                reg_deltas.push(format!(
                    "A{}: 0x{:08X} -> 0x{:08X}",
                    i, a_before[i], a_now[i]
                ));
            }
        }

        // Check SR
        if cpu.state.sr != cpu_before.state.sr {
            reg_deltas.push(format!(
                "SR: 0x{:04X} -> 0x{:04X}",
                cpu_before.state.sr, cpu.state.sr
            ));
        }

        // Check PC
        if cpu.state.pc != cpu_before.state.pc {
            reg_deltas.push(format!("PC: 0x{:06X}", cpu.state.pc));
        }

        // Check destination RAM buffer writes (if A1 or A0 post-incremented or stack written)
        let mut mem_deltas = Vec::new();
        if cpu.state.a_long(1) != cpu_before.state.a_long(1) {
            let written_addr = cpu_before.state.a_long(1);
            if written_addr >= BENCH_RAM_BUFFER_A1 && written_addr < BENCH_RAM_BUFFER_A1 + 0x2000 {
                let val = bus.read_word_debug(written_addr);
                mem_deltas.push(format!("Mem[0x{:06X}] <= 0x{:04X}", written_addr, val));
            }
        }
        if cpu.state.a_long(7) != cpu_before.state.a_long(7) {
            let new_sp = cpu.state.a_long(7);
            let old_sp = cpu_before.state.a_long(7);
            if new_sp < old_sp {
                // Stack push: read pushed word
                let val = bus.read_word_debug(new_sp);
                mem_deltas.push(format!("StackPush[0x{:06X}] <= 0x{:04X}", new_sp, val));
            }
        }

        step_count += 1;
        steps.push(StepTraceRecord {
            step_index: step_count,
            pc_before,
            disassembly,
            guest_cycles: cycles,
            reg_deltas,
            mem_deltas,
        });

        if step_count >= max_steps {
            timed_out = true;
            break;
        }
    }

    let terminated_cleanly = cpu.state.instruction_pc == program.exit_pc || cpu.state.stopped;

    BenchmarkTraceLog {
        spec: program.spec,
        unroll_k: program.unroll_k,
        initial_pc,
        initial_sr,
        initial_d,
        initial_a,
        steps,
        final_d: *cpu.state.d_regs(),
        final_a: *cpu.state.a_regs(),
        final_sr: cpu.state.sr,
        final_pc: cpu.state.instruction_pc,
        total_guest_cycles: total_cycles,
        terminated_cleanly,
        timed_out,
    }
}

/// Dumps formatted execution traces for all provided specifications into `<out_dir>/traces/<spec_id>.trace`
pub fn dump_benchmark_traces(
    specs: &[&BenchmarkSpec],
    out_dir: &Path,
    unroll_k: usize,
) -> Result<usize, String> {
    let trace_dir = out_dir.join("traces");
    fs::create_dir_all(&trace_dir).map_err(|e| {
        format!(
            "Failed to create trace output directory {:?}: {}",
            trace_dir, e
        )
    })?;

    let max_safety_steps = unroll_k * 3 + 50;
    let mut count = 0;

    for spec in specs {
        let program = BenchmarkProgramBuilder::new(**spec)
            .with_unroll(unroll_k)
            .build();

        let trace_log = trace_program(&program, max_safety_steps);
        let text = trace_log.format_text();

        let trace_path = trace_dir.join(format!("{}.trace", spec.id));
        fs::write(&trace_path, text)
            .map_err(|e| format!("Failed to write trace file {:?}: {}", trace_path, e))?;

        count += 1;
    }

    Ok(count)
}
