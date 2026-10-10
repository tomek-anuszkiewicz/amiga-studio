#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Smoke & Verification Test for M68000 Instruction Benchmarking Engine

use cpu::Cpu;
use physical_memory::PhysicalMemory;
use test_runner::benchmark::{
    find_spec_by_id, BenchmarkConfig, BenchmarkProfile, BenchmarkProgramBuilder,
};

#[test]
fn test_cascading_rts_execution() {
    let rts_spec = find_spec_by_id("FLOW-05").expect("FLOW-05 spec missing");
    let program = BenchmarkProgramBuilder::new(*rts_spec)
        .with_unroll(20)
        .build();

    let mut bus = PhysicalMemory::new();
    let mut cpu = Cpu::new();
    program.inject_into(&mut cpu, &mut bus);

    // Run execution loop
    let mut steps = 0;
    while !cpu.state.halted && !cpu.state.stopped && cpu.state.instruction_pc != program.exit_pc {
        let cycles = cpu.step_instruction(&mut bus);
        assert!(cycles > 0, "CPU should step valid instruction");
        steps += 1;
        if steps > 500 {
            panic!("Execution exceeded expected step count in cascading RTS");
        }
    }

    // CPU should reach exit PC cleanly or enter STOP
    assert!(
        cpu.state.instruction_pc == program.exit_pc || cpu.state.stopped,
        "CPU should reach exit PC cleanly, current instruction_pc: 0x{:06X}, stopped: {}",
        cpu.state.instruction_pc,
        cpu.state.stopped
    );
}

#[test]
fn test_benchmark_runner_quick_smoke() {
    let out_dir = std::env::temp_dir().join("amiga_bench_test_smoke");

    let config = BenchmarkConfig {
        profile: BenchmarkProfile::Quick,
        filter: Some("BASE-00|ARITH-02|FLOW-05".to_string()),
        unroll_k: 20,
        passes: 2,
        iterations: 5,
        out_dir,
        pin_core: false,
        verbose: false,
    };

    let report = test_runner::benchmark::run_benchmark_suite(&config)
        .expect("run_benchmark_suite should succeed");

    assert!(!report.results.is_empty(), "Report should contain results");
    for res in &report.results {
        assert!(res.total_guest_instructions > 0);
        assert!(res.host_duration_median_ms >= 0.0);
    }
}
