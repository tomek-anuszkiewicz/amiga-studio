//! Cargo entry point and profile selection for M68000 instruction benchmarks.

mod anomaly;
mod builder;
mod candidate_measure;
mod candidate_probe;
mod catalog;
mod catalog_data_a;
mod catalog_data_b;
mod inventory;
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
  --quick | --standard | --thorough  Run a legacy 108-case profile
  --inventory                       Write the decoder opcode inventory
  --probe-candidates                Validate one diagnostic step per candidate
  --measure-candidates              Measure all validated candidate workloads
  --measure-candidates-pilot        Measure a small representative subset
  --retry-candidates                Remeasure invalid cases or control segments
  --help, -h                        Print these instructions

Run without options to print these instructions.
Select one command. Unknown or repeated options are errors.";

#[derive(Debug)]
pub enum Command {
    Help,
    Inventory,
    ProbeCandidates,
    MeasureCandidates,
    MeasureCandidatesPilot,
    RetryCandidates,
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
        return Err("Select one measurement profile, inventory, or help".to_owned());
    };
    let profile = match option.as_str() {
        "--help" | "-h" => return Ok(Command::Help),
        "--inventory" => return Ok(Command::Inventory),
        "--probe-candidates" => return Ok(Command::ProbeCandidates),
        "--measure-candidates" => return Ok(Command::MeasureCandidates),
        "--measure-candidates-pilot" => return Ok(Command::MeasureCandidatesPilot),
        "--retry-candidates" => return Ok(Command::RetryCandidates),
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
        Command::Inventory => {
            let out_dir = runner::resolve_benchmarks_dir().join("inventory");
            match inventory::write_decoder_inventory(&out_dir) {
                Ok(counts) => println!(
                    "Decoder inventory: {} legal encodings, {} decode exceptions, {} structural groups, {} legacy catalog cases, {} legal encodings in the legacy catalog, {} legal encodings absent from the legacy catalog, {} catalog conflicts. CSV: {}",
                    counts.legal_encodings,
                    counts.decode_exceptions,
                    counts.structural_groups,
                    counts.catalog_cases,
                    counts.declared_encodings,
                    counts.undeclared_legal_encodings,
                    counts.catalog_decode_exceptions,
                    out_dir.join("decoder_inventory.csv").display()
                ),
                Err(error) => {
                    eprintln!("Inventory failed: {error}");
                    std::process::exit(1);
                }
            }
        }
        Command::ProbeCandidates => {
            let directory = runner::resolve_benchmarks_dir().join("inventory");
            if let Err(error) = candidate_probe::probe_candidates(&directory) {
                eprintln!("Candidate probe failed: {error}");
                std::process::exit(1);
            }
        }
        Command::MeasureCandidates => {
            let directory = runner::resolve_benchmarks_dir().join("inventory");
            if let Err(error) = candidate_measure::measure_candidates(&directory, false, false) {
                eprintln!("Candidate measurement failed: {error}");
                std::process::exit(1);
            }
        }
        Command::MeasureCandidatesPilot => {
            let directory = runner::resolve_benchmarks_dir().join("inventory");
            if let Err(error) = candidate_measure::measure_candidates(&directory, true, false) {
                eprintln!("Candidate measurement pilot failed: {error}");
                std::process::exit(1);
            }
        }
        Command::RetryCandidates => {
            let directory = runner::resolve_benchmarks_dir().join("inventory");
            if let Err(error) = candidate_measure::measure_candidates(&directory, false, true) {
                eprintln!("Candidate retry failed: {error}");
                std::process::exit(1);
            }
        }
        Command::Run(config) => {
            if let Err(error) = run_benchmark_suite(&config) {
                eprintln!("Benchmark failed: {error}");
                std::process::exit(1);
            }
        }
    }
}
