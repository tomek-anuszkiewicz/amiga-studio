//! Smoke & Verification Test for M68000 Instruction Benchmarking Engine

use m68000::Cpu;
use memory_bus::MemoryBus;
use test_runner::benchmark::{
    filter_specs, find_spec_by_id, total_benchmark_specs_count, BenchmarkConfig, BenchmarkProfile,
    BenchmarkProgramBuilder,
};

#[test]
fn test_catalog_integrity() {
    let count = total_benchmark_specs_count();
    assert!(
        count >= 70,
        "Catalog should have at least 70 entries, got {}",
        count
    );

    // Verify baseline exists
    let nop_spec = find_spec_by_id("BASE-00");
    assert!(
        nop_spec.is_some(),
        "BASE-00 NOP baseline must be registered"
    );

    // Verify filtering
    let move_specs = filter_specs("move_reg");
    assert!(!move_specs.is_empty(), "Should filter move_reg specs");
    assert!(move_specs.iter().all(|s| s.category_tag == "move_reg"));
}

#[test]
fn test_program_builder_synthesis() {
    let add_spec = find_spec_by_id("ARITH-02").expect("ARITH-02 spec missing");
    let program = BenchmarkProgramBuilder::new(*add_spec)
        .with_unroll(50)
        .build();

    assert_eq!(program.unroll_k, 50);
    assert_eq!(program.total_ops_per_pass, 50);
    assert_eq!(program.total_cck_per_pass, 50 * (add_spec.amiga_cck as u64));

    let mut bus = MemoryBus::new();
    let mut cpu = Cpu::new();
    program.inject_into(&mut cpu, &mut bus);

    // Initial PC should be primed at entry
    assert_eq!(cpu.state.pc, program.entry_pc + 4);
    assert_eq!(cpu.state.ir, 0xD041); // ADD.W D1, D0
}

#[test]
fn test_cascading_rts_execution() {
    let rts_spec = find_spec_by_id("FLOW-05").expect("FLOW-05 spec missing");
    let program = BenchmarkProgramBuilder::new(*rts_spec)
        .with_unroll(20)
        .build();

    let mut bus = MemoryBus::new();
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

#[test]
fn test_catalog_modes_and_categories() {
    let modes = [
        test_runner::benchmark::AddressingMode::Implied,
        test_runner::benchmark::AddressingMode::DataRegDirect,
        test_runner::benchmark::AddressingMode::AddrRegDirect,
        test_runner::benchmark::AddressingMode::AddrIndirect,
        test_runner::benchmark::AddressingMode::PostIncrement,
        test_runner::benchmark::AddressingMode::PreDecrement,
        test_runner::benchmark::AddressingMode::Displacement,
        test_runner::benchmark::AddressingMode::Index,
        test_runner::benchmark::AddressingMode::AbsoluteShort,
        test_runner::benchmark::AddressingMode::AbsoluteLong,
        test_runner::benchmark::AddressingMode::PcDisplacement,
        test_runner::benchmark::AddressingMode::PcIndex,
        test_runner::benchmark::AddressingMode::Immediate,
    ];

    for m in modes {
        assert!(!m.name().is_empty());
    }

    let categories = [
        test_runner::benchmark::InstructionCategory::Baseline,
        test_runner::benchmark::InstructionCategory::DataMovement,
        test_runner::benchmark::InstructionCategory::Arithmetic,
        test_runner::benchmark::InstructionCategory::Logic,
        test_runner::benchmark::InstructionCategory::ShiftRotate,
        test_runner::benchmark::InstructionCategory::Comparison,
        test_runner::benchmark::InstructionCategory::Bcd,
        test_runner::benchmark::InstructionCategory::ControlFlow,
        test_runner::benchmark::InstructionCategory::System,
    ];

    for c in categories {
        assert!(!c.name().is_empty());
    }

    // Lookup of nonexistent ID returns None
    assert!(find_spec_by_id("NONEXISTENT_99").is_none());
}

#[test]
fn test_benchmark_profiles_and_paths() {
    use test_runner::benchmark::resolve_benchmarks_dir;

    assert_eq!(BenchmarkProfile::Quick.name(), "quick");
    assert_eq!(BenchmarkProfile::Quick.default_passes(), 3);
    assert_eq!(BenchmarkProfile::Quick.default_iterations(), 15);

    assert_eq!(BenchmarkProfile::Standard.name(), "standard");
    assert_eq!(BenchmarkProfile::Standard.default_passes(), 7);
    assert_eq!(BenchmarkProfile::Standard.default_iterations(), 1428);

    assert_eq!(BenchmarkProfile::Thorough.name(), "thorough");
    assert_eq!(BenchmarkProfile::Thorough.default_passes(), 15);
    assert_eq!(BenchmarkProfile::Thorough.default_iterations(), 14285);

    let bench_dir = resolve_benchmarks_dir();
    assert!(
        bench_dir.ends_with("tests/benchmarks") || bench_dir.ends_with("tests\\benchmarks"),
        "resolve_benchmarks_dir must point to tests/benchmarks, got: {:?}",
        bench_dir
    );
}
