#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Unit tests for M68000 Benchmark Program Builder & Strategy Synthesis
//!
//! Validates unrolled block generation, PRNG memory buffers, cascading stacks,
//! subroutine linkage, and CPU/Bus memory injection across all benchmark strategies.

use cpu::Cpu;
use physical_memory::PhysicalMemory;
use test_runner::benchmark::builder::{
    BenchmarkProgramBuilder, BENCH_ENTRY_PC, BENCH_EXIT_PC, BENCH_RAM_BUFFER_A0,
    BENCH_RAM_BUFFER_A1, BENCH_STACK_TOP,
};
use test_runner::benchmark::catalog::{all_benchmark_specs, find_spec_by_id, AddressingMode};

#[test]
fn test_builder_all_catalog_specs_succeed() {
    let specs: Vec<_> = all_benchmark_specs().collect();
    assert_eq!(specs.len(), 108);

    for spec in specs {
        let program = BenchmarkProgramBuilder::new(spec.clone()).build();

        assert_eq!(program.entry_pc, BENCH_ENTRY_PC);
        assert_eq!(program.exit_pc, BENCH_EXIT_PC);
        assert_eq!(program.unroll_k, 700);
        assert!(program.total_ops_per_pass >= 700);
        assert!(program.total_cck_per_pass > 0);
        assert!(!program.memory_writes.is_empty());

        // Verify initial SSP is set at or below top of stack
        assert!(program.initial_ssp <= BENCH_STACK_TOP);

        // Verify supervisor mode is set (SR bit 13)
        assert_ne!(program.initial_sr & 0x2000, 0);
    }
}

#[test]
fn test_builder_custom_unroll_factor() {
    let spec = find_spec_by_id("ARITH-02").expect("ARITH-02 must exist");
    let program = BenchmarkProgramBuilder::new(spec.clone())
        .with_unroll(50)
        .build();

    assert_eq!(program.unroll_k, 50);
    assert_eq!(program.total_ops_per_pass, 50);
    assert_eq!(program.total_cck_per_pass, 50 * 4);
}

#[test]
fn test_builder_prng_seeding_effect() {
    // DIVU requires PRNG data for non-zero divisors
    let spec = find_spec_by_id("ARITH-18").expect("ARITH-18 must exist");

    let prog1 = BenchmarkProgramBuilder::new(spec.clone())
        .with_prng_seed(0x1234_5678_9ABC_DEF0)
        .build();
    let prog2 = BenchmarkProgramBuilder::new(spec.clone())
        .with_prng_seed(0x1234_5678_9ABC_DEF0)
        .build();
    let prog3 = BenchmarkProgramBuilder::new(spec.clone())
        .with_prng_seed(0x9999_8888_7777_6666)
        .build();

    // Identical seeds must yield bit-for-bit identical memory writes
    assert_eq!(prog1.memory_writes, prog2.memory_writes);

    // Different seeds must yield different memory writes
    assert_ne!(prog1.memory_writes, prog3.memory_writes);
}

#[test]
fn test_builder_memory_addressing_modes_setup() {
    let spec_postinc = find_spec_by_id("MOV-05").expect("MOV-05 must exist");
    assert_eq!(spec_postinc.addressing_mode, AddressingMode::PostIncrement);

    let program = BenchmarkProgramBuilder::new(spec_postinc.clone()).build();

    // In PostIncrement mode, A0 must point to BENCH_RAM_BUFFER_A0
    assert_eq!(program.initial_a[0], BENCH_RAM_BUFFER_A0);

    // Verify buffer contains initial written data
    let has_writes_in_buffer = program
        .memory_writes
        .iter()
        .any(|&(addr, _)| addr >= BENCH_RAM_BUFFER_A0 && addr < BENCH_RAM_BUFFER_A1);
    assert!(
        has_writes_in_buffer,
        "Expected memory writes inside RAM buffer A0"
    );
}

#[test]
fn test_builder_injection_into_cpu_and_bus() {
    let spec = find_spec_by_id("ARITH-02").expect("ARITH-02 must exist");
    let program = BenchmarkProgramBuilder::new(spec.clone()).build();

    let mut cpu = Cpu::new();
    let mut bus = PhysicalMemory::new();

    program.inject_into(&mut cpu, &mut bus);

    // 1. Check CPU register state
    assert_eq!(cpu.state.sr, program.initial_sr);
    assert_eq!(cpu.state.ssp, program.initial_ssp);
    for i in 0..8 {
        assert_eq!(cpu.state.d_long(i), program.initial_d[i]);
        if i == 7 {
            assert_eq!(cpu.state.a_long(7), program.initial_ssp);
        } else {
            assert_eq!(cpu.state.a_long(i), program.initial_a[i]);
        }
    }

    // 2. Prefetch priming reads 2 words from entry_pc, advancing PC by 4
    assert_eq!(cpu.state.pc, program.entry_pc + 4);
    assert_eq!(cpu.state.ir, 0xD041); // ADD.W D1, D0 opcode

    // 3. Check memory contents at entry_pc (ADD.W D1, D0 opcode = 0xD041)
    let entry = program.entry_pc as usize;
    assert_eq!(bus.chip_ram[entry], 0xD0);
    assert_eq!(bus.chip_ram[entry + 1], 0x41);

    // 4. Check STOP #$2700 sentinel at exit_pc (0x4E72 0x2700)
    let exit = program.exit_pc as usize;
    assert_eq!(bus.chip_ram[exit], 0x4E);
    assert_eq!(bus.chip_ram[exit + 1], 0x72);
    assert_eq!(bus.chip_ram[exit + 2], 0x27);
    assert_eq!(bus.chip_ram[exit + 3], 0x00);
}

#[test]
fn test_benchmark_program_builder_debug_derive() {
    let spec = find_spec_by_id("ARITH-02").expect("ARITH-02 must exist");
    let builder = BenchmarkProgramBuilder::new(*spec);
    let debug_str = format!("{:?}", builder);
    assert!(debug_str.contains("BenchmarkProgramBuilder"));
}

#[test]
fn test_builder_cpu_registers_slice_views() {
    let spec = find_spec_by_id("ARITH-02").expect("ARITH-02 must exist");
    let program = BenchmarkProgramBuilder::new(spec.clone()).build();

    let mut cpu = Cpu::new();
    let mut bus = PhysicalMemory::new();

    program.inject_into(&mut cpu, &mut bus);

    let d_slice = cpu.state.d_regs();
    let a_slice = cpu.state.a_regs();
    for i in 0..8 {
        assert_eq!(d_slice[i], program.initial_d[i]);
        if i == 7 {
            assert_eq!(a_slice[7], program.initial_ssp);
        } else {
            assert_eq!(a_slice[i], program.initial_a[i]);
        }
    }
}
