//! M68000 Instruction Benchmarking & Performance Profiling Subsystem
//!
//! Provides cycle-exact instruction micro-benchmarking, automated program synthesis (K = 700 ops),
//! P-core affinity pinning, multi-pass statistical noise filtering, and anomaly detection
//! per Obsidian/Amiga/Design/CPU Instruction Benchmarking.md.

pub mod anomaly;
pub mod builder;
pub mod catalog;
pub mod catalog_data_a;
pub mod catalog_data_b;
pub mod chipset;
pub mod persistence;
pub mod platform;
pub mod prng;
pub mod runner;
pub mod stats;
pub mod tracer;

pub use anomaly::{
    evaluate_anomaly, evaluate_historical_diff, AnomalyType, AnomalyVerdict, RegressionStatus,
};
pub use builder::{
    BenchmarkProgram, BenchmarkProgramBuilder, BENCH_DEFAULT_UNROLL, BENCH_ENTRY_PC, BENCH_EXIT_PC,
    BENCH_RAM_BUFFER_A0, BENCH_RAM_BUFFER_A1, BENCH_STACK_TOP,
};
pub use catalog::{
    all_benchmark_specs, filter_specs, find_spec_by_id, total_benchmark_specs_count,
    AddressingMode, BenchmarkSpec, BenchmarkStrategy, InstructionCategory,
};
pub use chipset::*;
pub use persistence::{
    format_timestamp_filename, load_historical_baseline, load_previous_report,
    persist_benchmark_report, BenchmarkEnvironmentInfo, BenchmarkItemResult, BenchmarkSuiteReport,
};
pub use platform::HostEnvironment;
pub use prng::{XorShift64, BENCH_PRNG_SEED};
pub use runner::{
    execute_single_spec, resolve_benchmarks_dir, run_benchmark_suite, BenchmarkConfig,
    BenchmarkExecutionResult, BenchmarkProfile,
};
pub use stats::{analyze_pass_samples, BenchmarkMetrics};
pub use tracer::{dump_benchmark_traces, trace_program, BenchmarkTraceLog, StepTraceRecord};
