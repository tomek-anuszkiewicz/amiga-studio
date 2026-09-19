//! Programmatic M68000 Benchmark Code Generator
//!
//! Synthesizes unrolled instruction blocks (K = 700 ops), preambles, footers,
//! cascading stacks, and PRNG memory buffers directly in Rust memory per
//! Obsidian/Amiga/Design/CPU Instruction Benchmark Strategies.md.

use cpu::Cpu;
use physical_memory::PhysicalMemory;

use super::catalog::{BenchmarkSpec, BenchmarkStrategy};
use super::prng::XorShift64;

pub const BENCH_ENTRY_PC: u32 = 0x001000;
pub const BENCH_EXIT_PC: u32 = 0x004FFE;
pub const BENCH_STACK_TOP: u32 = 0x07F000;
pub const BENCH_RAM_BUFFER_A0: u32 = 0x006000;
pub const BENCH_RAM_BUFFER_A1: u32 = 0x007000;
pub const BENCH_DEFAULT_UNROLL: usize = 700;

/// Self-contained synthesized benchmark program ready for injection into Chip RAM
#[derive(Debug, Clone)]
pub struct BenchmarkProgram {
    pub spec: BenchmarkSpec,
    pub entry_pc: u32,
    pub exit_pc: u32,
    pub initial_ssp: u32,
    pub initial_sr: u16,
    pub initial_d: [u32; 8],
    pub initial_a: [u32; 8],
    pub memory_writes: Vec<(u32, u8)>,
    pub unroll_k: usize,
    pub total_ops_per_pass: u64,
    pub total_cck_per_pass: u64,
}

impl BenchmarkProgram {
    /// Injects the program image, initial CPU register state, and primed prefetch into the machine
    pub fn inject_into(&self, cpu: &mut Cpu, bus: &mut PhysicalMemory) {
        bus.map_chip_ram_to_low_memory();

        // Write memory image
        for &(addr, byte) in &self.memory_writes {
            let offset = (addr & 0x00FF_FFFF) as usize;
            if offset < bus.chip_ram.len() {
                bus.chip_ram[offset] = byte;
            }
        }

        // Put a STOP #$2700 instruction at exit_pc so reaching exit cleanly halts CPU
        let exit_offset = (self.exit_pc & 0x00FF_FFFF) as usize;
        if exit_offset + 3 < bus.chip_ram.len() {
            bus.chip_ram[exit_offset] = 0x4E;
            bus.chip_ram[exit_offset + 1] = 0x72;
            bus.chip_ram[exit_offset + 2] = 0x27;
            bus.chip_ram[exit_offset + 3] = 0x00;
        }

        // Set initial CPU registers
        cpu.state.sr = self.initial_sr;
        cpu.state.ssp = self.initial_ssp;
        cpu.state.set_d_regs(self.initial_d);
        cpu.state.set_a_regs(self.initial_a);
        cpu.state.set_a_long(7, self.initial_ssp);
        cpu.state.stopped = false;
        cpu.state.halted = false;

        // Reset microcode state
        cpu.state.micro.reset();
        cpu.reset_cycle_counter();

        // Prime PC and prefetch pipeline
        cpu.set_pc_and_prime_prefetch(self.entry_pc, bus);
    }
}

/// Programmatic builder for M68000 benchmark programs
#[derive(Debug)]
pub struct BenchmarkProgramBuilder {
    spec: BenchmarkSpec,
    unroll_k: usize,
    prng: XorShift64,
}

impl BenchmarkProgramBuilder {
    pub fn new(spec: BenchmarkSpec) -> Self {
        Self {
            spec,
            unroll_k: BENCH_DEFAULT_UNROLL,
            prng: XorShift64::default(),
        }
    }

    pub fn with_unroll(mut self, k: usize) -> Self {
        self.unroll_k = k.max(1);
        self
    }

    pub fn with_prng_seed(mut self, seed: u64) -> Self {
        self.prng = XorShift64::new(seed);
        self
    }

    /// Synthesizes the benchmark program in memory
    pub fn build(mut self) -> BenchmarkProgram {
        let mut writes = Vec::new();
        let mut initial_d = [0u32; 8];
        let mut initial_a = [0u32; 8];
        let mut initial_ssp = BENCH_STACK_TOP;
        let mut initial_sr = 0x2700; // Supervisor mode, interrupts disabled

        // Seed registers with deterministic PRNG patterns
        for reg in 0..7 {
            initial_d[reg] = self.prng.next_u32();
            initial_a[reg] = BENCH_RAM_BUFFER_A0 + (reg as u32 * 0x200);
        }
        initial_a[0] = BENCH_RAM_BUFFER_A0;
        initial_a[1] = BENCH_RAM_BUFFER_A1;
        initial_a[6] = BENCH_RAM_BUFFER_A0 + 0x0800; // Frame pointer
        initial_a[7] = initial_ssp;

        // Populate PRNG memory operand buffer ($000400 - $000FFF)
        let mut operand_buffer = vec![0u8; 0x0C00];
        self.prng.fill_bytes(&mut operand_buffer);
        for (i, &b) in operand_buffer.iter().enumerate() {
            writes.push((0x000400 + i as u32, b));
        }

        // Also populate 4KB RAM buffer at A0 and A1
        let mut ram_buf = vec![0u8; 0x1000];
        self.prng.fill_bytes(&mut ram_buf);
        for (i, &b) in ram_buf.iter().enumerate() {
            writes.push((BENCH_RAM_BUFFER_A0 + i as u32, b));
            writes.push((BENCH_RAM_BUFFER_A1 + i as u32, b));
        }

        // Generate program instructions
        let mut code_words = Vec::new();

        match self.spec.strategy {
            BenchmarkStrategy::CascadingReturnRts => {
                self.build_cascading_rts(&mut writes, &mut initial_a, &mut initial_ssp);
            }
            BenchmarkStrategy::CascadingReturnRtr => {
                self.build_cascading_rtr(&mut writes, &mut initial_a, &mut initial_ssp);
            }
            BenchmarkStrategy::CascadingReturnRte => {
                self.build_cascading_rte(&mut writes, &mut initial_a, &mut initial_ssp);
            }
            BenchmarkStrategy::DivideZeroTrap => {
                self.build_divide_zero_trampoline(&mut writes, &mut code_words, &mut initial_d);
            }
            BenchmarkStrategy::DivideValid => {
                // Divisor D1 must be non-zero and Dividend D0 < Divisor * 0x10000
                initial_d[1] = 0x0000_00A5;
                initial_d[0] = 0x0005_0000;
                self.emit_unrolled_opcodes(&mut code_words);
            }
            BenchmarkStrategy::DivideOverflow => {
                // Dividend high word >= Divisor
                initial_d[1] = 0x0000_0002;
                initial_d[0] = 0xFFFF_0000;
                self.emit_unrolled_opcodes(&mut code_words);
            }
            BenchmarkStrategy::CheckInBounds => {
                // CHK.W D1, D0: 0 <= D0 <= D1
                initial_d[1] = 0x0000_7FFF;
                initial_d[0] = 0x0000_0010;
                self.emit_unrolled_opcodes(&mut code_words);
            }
            BenchmarkStrategy::BcdReg | BenchmarkStrategy::BcdMem => {
                initial_d[0] = 0x1234_5678;
                initial_d[1] = 0x0908_0706;
                // Pre-fill memory buffer at A0 and A1 with strictly valid packed BCD bytes
                for i in 0..0x800 {
                    let bcd = self.prng.next_bcd_byte();
                    writes.push((BENCH_RAM_BUFFER_A0 + i as u32, bcd));
                    writes.push((BENCH_RAM_BUFFER_A1 + i as u32, bcd));
                }
                initial_a[0] = BENCH_RAM_BUFFER_A0 + 0x600;
                initial_a[1] = BENCH_RAM_BUFFER_A1 + 0x600;
                self.emit_unrolled_opcodes(&mut code_words);
            }
            BenchmarkStrategy::StackFramePair => {
                // LINK A6, #-16 (4E56 FFF0) + UNLK A6 (4E5E)
                for _ in 0..(self.unroll_k / 2) {
                    code_words.push(0x4E56);
                    code_words.push(0xFFF0);
                    code_words.push(0x4E5E);
                }
            }
            BenchmarkStrategy::BalancedShiftPair => {
                // Alternating shift left and shift right: LSL.W #3, D0 (E748); LSR.W #3, D0 (E648)
                let op1 = self.spec.opcode_words[0];
                let op2 = match op1 {
                    0xE748 => 0xE648, // LSL -> LSR
                    0xE648 => 0xE748, // LSR -> LSL
                    0xE540 => 0xE440, // ASL -> ASR
                    0xE440 => 0xE540, // ASR -> ASL
                    _ => op1,
                };
                for _ in 0..(self.unroll_k / 2) {
                    code_words.push(op1);
                    code_words.push(op2);
                }
            }
            BenchmarkStrategy::IndexMem => {
                // Ensure index register D2 is even and points into valid PRNG buffer
                initial_d[2] = 0x0000_0100;
                self.emit_unrolled_opcodes(&mut code_words);
            }
            BenchmarkStrategy::SystemSr => {
                // Keep Supervisor bit set (S=1, bit 13) and IPL=7 so subsequent MOVE to SR remains privileged
                initial_d[0] = 0x0000_2700;
                self.emit_unrolled_opcodes(&mut code_words);
            }
            BenchmarkStrategy::SystemTrap => {
                // Install RTE handler at vector 32 ($000080) pointing to $000520
                let handler_addr = 0x000520u32;
                writes.push((0x000080, (handler_addr >> 24) as u8));
                writes.push((0x000081, (handler_addr >> 16) as u8));
                writes.push((0x000082, (handler_addr >> 8) as u8));
                writes.push((0x000083, (handler_addr & 0xFF) as u8));
                // RTE opcode (4E73)
                writes.push((handler_addr, 0x4E));
                writes.push((handler_addr + 1, 0x73));
                self.emit_unrolled_opcodes(&mut code_words);
            }
            BenchmarkStrategy::BranchTaken => {
                // BRA.S +2 (6002) or BEQ.S +2 (6702) with Z = 1
                initial_sr |= 0x0004; // Z = 1
                let op = self.spec.opcode_words[0];
                for _ in 0..self.unroll_k {
                    code_words.push(op);
                    code_words.push(0x4E71); // NOP skipped by the +2 displacement
                }
            }
            BenchmarkStrategy::BranchUntaken => {
                // BEQ.S +2 with Z = 0
                initial_sr &= !0x0004; // Z = 0
                for _ in 0..self.unroll_k {
                    code_words.push(self.spec.opcode_words[0]);
                }
            }
            BenchmarkStrategy::SubroutineCall => {
                // In-line BSR.S subroutine call and local return:
                // BSR.S +2 (6102) -> pushes return PC, branches to RTS at offset +4
                // BRA.S +4 (6004) -> return target: skips over RTS and NOP to next BSR
                // RTS      (4E75) -> subroutine: pops return PC and returns to BRA.S
                // NOP      (4E71) -> alignment padding
                for _ in 0..(self.unroll_k / 2) {
                    code_words.push(0x6102);
                    code_words.push(0x6004);
                    code_words.push(0x4E75);
                    code_words.push(0x4E71);
                }
            }
            BenchmarkStrategy::LoopDecrement => {
                if self.spec.id == "FLOW-07" {
                    // DBF D7, target (Taken): initialize D7 so it never hits -1 during unroll
                    initial_d[7] = (self.unroll_k + 100) as u32;
                    for _ in 0..self.unroll_k {
                        code_words.push(0x51CF);
                        code_words.push(0x0002); // 16-bit disp +2 branches to next DBF opcode
                    }
                } else {
                    // FLOW-08 DBF D7, exit (Fallthrough): pair with MOVEQ #0, D7 so D7 decrements to -1
                    for _ in 0..(self.unroll_k / 2) {
                        code_words.push(0x7E00); // MOVEQ #0, D7
                        code_words.push(0x51CF); // DBF D7
                        code_words.push(0x0002); // disp (falls through)
                    }
                }
            }
            _ => {
                // Standard unrolled block
                self.emit_unrolled_opcodes(&mut code_words);
            }
        }

        if !code_words.is_empty() {
            // Conclude inner block with jump to exit sentinel
            // JMP BENCH_EXIT_PC (4EF9 0000 4FFE)
            code_words.push(0x4EF9);
            code_words.push((BENCH_EXIT_PC >> 16) as u16);
            code_words.push((BENCH_EXIT_PC & 0xFFFF) as u16);

            // Emit code words into memory_writes starting at BENCH_ENTRY_PC
            let mut cur_addr = BENCH_ENTRY_PC;
            for &w in &code_words {
                writes.push((cur_addr, (w >> 8) as u8));
                writes.push((cur_addr + 1, (w & 0xFF) as u8));
                cur_addr += 2;
            }
        }

        let total_ops = self.unroll_k as u64;
        let total_cck = total_ops * (self.spec.amiga_cck as u64);

        BenchmarkProgram {
            spec: self.spec,
            entry_pc: BENCH_ENTRY_PC,
            exit_pc: BENCH_EXIT_PC,
            initial_ssp,
            initial_sr,
            initial_d,
            initial_a,
            memory_writes: writes,
            unroll_k: self.unroll_k,
            total_ops_per_pass: total_ops,
            total_cck_per_pass: total_cck,
        }
    }

    fn emit_unrolled_opcodes(&self, code_words: &mut Vec<u16>) {
        for _ in 0..self.unroll_k {
            for &w in self.spec.opcode_words {
                code_words.push(w);
            }
        }
    }

    fn build_cascading_rts(
        &self,
        writes: &mut Vec<(u32, u8)>,
        initial_a: &mut [u32; 8],
        initial_ssp: &mut u32,
    ) {
        // Layout K unrolled RTS instructions (0x4E75) at 0x008000
        let rts_base = 0x008000u32;
        for i in 0..self.unroll_k {
            let addr = rts_base + (i as u32 * 2);
            writes.push((addr, 0x4E));
            writes.push((addr + 1, 0x75));
        }

        // Pre-populate stack with return addresses P1..P(K-1), followed by BENCH_EXIT_PC
        // SP grows downward. Top of stack holds P1.
        let mut sp = BENCH_STACK_TOP;
        let mut stack_addrs = Vec::with_capacity(self.unroll_k);
        for i in 1..self.unroll_k {
            stack_addrs.push(rts_base + (i as u32 * 2));
        }
        stack_addrs.push(BENCH_EXIT_PC);

        // Push onto stack (reverse order)
        for &target in stack_addrs.iter().rev() {
            sp = sp.wrapping_sub(4);
            writes.push((sp, (target >> 24) as u8));
            writes.push((sp + 1, (target >> 16) as u8));
            writes.push((sp + 2, (target >> 8) as u8));
            writes.push((sp + 3, (target & 0xFF) as u8));
        }
        initial_a[7] = sp;
        *initial_ssp = sp;

        // Entry point executes JMP rts_base
        let mut cur = BENCH_ENTRY_PC;
        let jmp_words = [
            0x4EF9u16,
            (rts_base >> 16) as u16,
            (rts_base & 0xFFFF) as u16,
        ];
        for &w in &jmp_words {
            writes.push((cur, (w >> 8) as u8));
            writes.push((cur + 1, (w & 0xFF) as u8));
            cur += 2;
        }
    }

    fn build_cascading_rtr(
        &self,
        writes: &mut Vec<(u32, u8)>,
        initial_a: &mut [u32; 8],
        initial_ssp: &mut u32,
    ) {
        let rtr_base = 0x008000u32;
        for i in 0..self.unroll_k {
            let addr = rtr_base + (i as u32 * 2);
            writes.push((addr, 0x4E));
            writes.push((addr + 1, 0x77)); // RTR
        }

        let mut sp = BENCH_STACK_TOP;
        let mut stack_frames = Vec::with_capacity(self.unroll_k);
        for i in 1..self.unroll_k {
            stack_frames.push((0x0000u16, rtr_base + (i as u32 * 2)));
        }
        stack_frames.push((0x0000u16, BENCH_EXIT_PC));

        for &(ccr, target) in stack_frames.iter().rev() {
            sp = sp.wrapping_sub(6);
            writes.push((sp, (ccr >> 8) as u8));
            writes.push((sp + 1, (ccr & 0xFF) as u8));
            writes.push((sp + 2, (target >> 24) as u8));
            writes.push((sp + 3, (target >> 16) as u8));
            writes.push((sp + 4, (target >> 8) as u8));
            writes.push((sp + 5, (target & 0xFF) as u8));
        }
        initial_a[7] = sp;
        *initial_ssp = sp;

        let mut cur = BENCH_ENTRY_PC;
        let jmp_words = [
            0x4EF9u16,
            (rtr_base >> 16) as u16,
            (rtr_base & 0xFFFF) as u16,
        ];
        for &w in &jmp_words {
            writes.push((cur, (w >> 8) as u8));
            writes.push((cur + 1, (w & 0xFF) as u8));
            cur += 2;
        }
    }

    fn build_cascading_rte(
        &self,
        writes: &mut Vec<(u32, u8)>,
        initial_a: &mut [u32; 8],
        initial_ssp: &mut u32,
    ) {
        let rte_base = 0x008000u32;
        for i in 0..self.unroll_k {
            let addr = rte_base + (i as u32 * 2);
            writes.push((addr, 0x4E));
            writes.push((addr + 1, 0x73)); // RTE
        }

        let mut sp = BENCH_STACK_TOP;
        let mut stack_frames = Vec::with_capacity(self.unroll_k);
        for i in 1..self.unroll_k {
            stack_frames.push((0x2700u16, rte_base + (i as u32 * 2)));
        }
        stack_frames.push((0x2700u16, BENCH_EXIT_PC));

        for &(sr, target) in stack_frames.iter().rev() {
            sp = sp.wrapping_sub(6);
            writes.push((sp, (sr >> 8) as u8));
            writes.push((sp + 1, (sr & 0xFF) as u8));
            writes.push((sp + 2, (target >> 24) as u8));
            writes.push((sp + 3, (target >> 16) as u8));
            writes.push((sp + 4, (target >> 8) as u8));
            writes.push((sp + 5, (target & 0xFF) as u8));
        }
        initial_a[7] = sp;
        *initial_ssp = sp;

        let mut cur = BENCH_ENTRY_PC;
        let jmp_words = [
            0x4EF9u16,
            (rte_base >> 16) as u16,
            (rte_base & 0xFFFF) as u16,
        ];
        for &w in &jmp_words {
            writes.push((cur, (w >> 8) as u8));
            writes.push((cur + 1, (w & 0xFF) as u8));
            cur += 2;
        }
    }

    fn build_divide_zero_trampoline(
        &self,
        writes: &mut Vec<(u32, u8)>,
        code_words: &mut Vec<u16>,
        initial_d: &mut [u32; 8],
    ) {
        // Vector 5 at $000014 -> Trampoline at $000500
        let handler_addr = 0x000500u32;
        writes.push((0x000014, (handler_addr >> 24) as u8));
        writes.push((0x000015, (handler_addr >> 16) as u8));
        writes.push((0x000016, (handler_addr >> 8) as u8));
        writes.push((0x000017, (handler_addr & 0xFF) as u8));

        // Trampoline assembly:
        // SUBQ.W #1, D7       (5347)
        // BEQ.S  +2           (6702) -> to done
        // RTE                 (4E73)
        // done:
        // ADDQ.L #6, SP       (5C8F) -> discard exception frame
        // JMP BENCH_EXIT_PC   (4EF9 0000 4FFE)
        let trampoline = [
            0x5347u16,
            0x6702,
            0x4E73,
            0x5C8F,
            0x4EF9,
            (BENCH_EXIT_PC >> 16) as u16,
            (BENCH_EXIT_PC & 0xFFFF) as u16,
        ];
        let mut cur = handler_addr;
        for &w in &trampoline {
            writes.push((cur, (w >> 8) as u8));
            writes.push((cur + 1, (w & 0xFF) as u8));
            cur += 2;
        }

        // Setup loop counter in D7 and Divisor 0 in D1
        initial_d[7] = self.unroll_k as u32;
        initial_d[1] = 0; // Divisor = 0
        initial_d[0] = 0x1234_5678;

        // Entry executes DIVU.W D1, D0 (80C1) which repeatedly traps to handler until D7 == 0
        code_words.push(0x80C1);
    }
}
