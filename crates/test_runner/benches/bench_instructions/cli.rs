//! Cargo entry point and profile selection for M68000 instruction benchmarks.

mod anomaly;
mod builder;
mod catalog;
mod catalog_data_a;
mod catalog_data_b;
mod persistence;
mod platform;
mod prng;
mod runner;
mod stats;

use runner::run_benchmark_suite;
pub(crate) use runner::{BenchmarkConfig, BenchmarkProfile};

pub const USAGE: &str = "M68000 Instruction Benchmarks
Usage: cargo bench -p test_runner --bench bench_instructions -- [PROFILE]

Options:
  --quick | --standard | --thorough  Select measurement profile
  --help, -h                        Print these instructions

Run without options to print these instructions.
Select one measurement profile. Unknown or repeated options are errors.";

#[derive(Debug)]
pub enum Command {
    Help,
    Run(BenchmarkConfig),
}

pub fn parse(args: &[String]) -> Result<Command, String> {
    // Cargo appends its benchmark marker even when the target disables the harness.
    let args = if args.last().is_some_and(|arg| arg == "--bench") {
        &args[..args.len() - 1]
    } else {
        args
    };
    if args.is_empty() {
        return Ok(Command::Help);
    }
    let [option] = args else {
        return Err("Select one measurement profile or help".to_owned());
    };
    let profile = match option.as_str() {
        "--help" | "-h" => return Ok(Command::Help),
        "--quick" => BenchmarkProfile::Quick,
        "--standard" => BenchmarkProfile::Standard,
        "--thorough" => BenchmarkProfile::Thorough,
        _ => return Err(format!("Unknown option: {option}")),
    };
    Ok(Command::Run(BenchmarkConfig {
        profile,
        passes: profile.default_passes(),
        iterations: profile.default_iterations(),
        ..BenchmarkConfig::default()
    }))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match parse(&args) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("Error: {error}\nUse '--help' for benchmark instructions.");
            std::process::exit(2);
        }
    };
    match command {
        Command::Help => println!("{USAGE}"),
        Command::Run(config) => {
            if let Err(error) = run_benchmark_suite(&config) {
                eprintln!("Benchmark failed: {error}");
                std::process::exit(1);
            }
        }
    }
}
