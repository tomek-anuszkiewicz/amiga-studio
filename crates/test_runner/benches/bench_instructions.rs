//! Cargo Benchmark Target for M68000 Cycle-Exact Instruction Benchmarking
//!
//! Invoked via:
//! `cargo bench -p test_runner --bench bench_instructions -- [OPTIONS]`
//!
//! Options:
//!   --quick      Fast smoke pass (~10k ops)
//!   --standard   Standard profiling pass (default)
//!   --thorough   Exhaustive profiling
//!   --filter <TAG> Filter by category or mnemonic

use std::path::PathBuf;
use test_runner::benchmark::{run_benchmark_suite, BenchmarkConfig, BenchmarkProfile};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut config = BenchmarkConfig::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--quick" => {
                config.profile = BenchmarkProfile::Quick;
                config.passes = BenchmarkProfile::Quick.default_passes();
                config.iterations = BenchmarkProfile::Quick.default_iterations();
            }
            "--standard" => {
                config.profile = BenchmarkProfile::Standard;
                config.passes = BenchmarkProfile::Standard.default_passes();
                config.iterations = BenchmarkProfile::Standard.default_iterations();
            }
            "--thorough" => {
                config.profile = BenchmarkProfile::Thorough;
                config.passes = BenchmarkProfile::Thorough.default_passes();
                config.iterations = BenchmarkProfile::Thorough.default_iterations();
            }
            "--filter" => {
                if i + 1 < args.len() {
                    config.filter = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--unroll" => {
                if i + 1 < args.len() {
                    if let Ok(k) = args[i + 1].parse::<usize>() {
                        config.unroll_k = k;
                    }
                    i += 1;
                }
            }
            "--passes" => {
                if i + 1 < args.len() {
                    if let Ok(p) = args[i + 1].parse::<usize>() {
                        config.passes = p;
                    }
                    i += 1;
                }
            }
            "--out-dir" => {
                if i + 1 < args.len() {
                    config.out_dir = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--no-pin" => {
                config.pin_core = false;
            }
            _ => {}
        }
        i += 1;
    }

    if let Err(e) = run_benchmark_suite(&config) {
        eprintln!("Benchmark failed: {}", e);
        std::process::exit(1);
    }
}
