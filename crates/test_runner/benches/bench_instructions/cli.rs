//! Cargo entry point for M68000 instruction benchmarks.

#[path = "../../../../benchmarks/m68k/runner.rs"]
mod asm_runner;
mod candidate_measure;
mod candidate_probe;
mod composition;
mod inventory;
mod platform;

fn benchmarks_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.agent/tmp/benchmarks")
}

pub const USAGE: &str = "M68000 Instruction Benchmarks
Usage: cargo bench -p test_runner --bench bench_instructions -- COMMAND

Options:
  --inventory                       Write the decoder opcode inventory
  --probe-candidates                Validate one diagnostic step per candidate
  --measure-candidates              Measure all validated candidate workloads
  --measure-candidates-pilot        Measure a small representative subset
  --retry-candidates                Remeasure cases listed by the retry plan
  --export-composition              Record executed instruction counts per candidate
  --measure-asm CASE                 Measure one assembly case by family/case ID
  --help, -h                        Print these instructions

Prepare retries with the verification analyzer before --retry-candidates.
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
    ExportComposition,
    MeasureAsm(String),
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
    if args.len() == 2 && args[0] == "--measure-asm" {
        return Ok(Command::MeasureAsm(args[1].clone()));
    }
    let [option] = args else {
        return Err("Select one benchmark command".to_owned());
    };
    match option.as_str() {
        "--help" | "-h" => return Ok(Command::Help),
        "--inventory" => return Ok(Command::Inventory),
        "--probe-candidates" => return Ok(Command::ProbeCandidates),
        "--measure-candidates" => return Ok(Command::MeasureCandidates),
        "--measure-candidates-pilot" => return Ok(Command::MeasureCandidatesPilot),
        "--retry-candidates" => return Ok(Command::RetryCandidates),
        "--export-composition" => return Ok(Command::ExportComposition),
        _ => Err(format!("Unknown option: {option}")),
    }
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
        Command::MeasureAsm(case_id) => {
            if let Err(error) = asm_runner::measure(&case_id) {
                eprintln!("ASM benchmark failed: {error}");
                std::process::exit(1);
            }
        }
        Command::Help => println!("{USAGE}"),
        Command::Inventory => {
            let out_dir = benchmarks_dir().join("inventory");
            match inventory::write_decoder_inventory(&out_dir) {
                Ok(counts) => println!(
                    "Decoder inventory: {} legal encodings, {} decode exceptions, {} structural groups. CSV: {}",
                    counts.legal_encodings,
                    counts.decode_exceptions,
                    counts.structural_groups,
                    out_dir.join("decoder_inventory.csv").display()
                ),
                Err(error) => {
                    eprintln!("Inventory failed: {error}");
                    std::process::exit(1);
                }
            }
        }
        Command::ProbeCandidates => {
            let directory = benchmarks_dir().join("inventory");
            if let Err(error) = candidate_probe::probe_candidates(&directory) {
                eprintln!("Candidate probe failed: {error}");
                std::process::exit(1);
            }
        }
        Command::MeasureCandidates => {
            let directory = benchmarks_dir().join("inventory");
            if let Err(error) = candidate_measure::measure_candidates(&directory, false, false) {
                eprintln!("Candidate measurement failed: {error}");
                std::process::exit(1);
            }
        }
        Command::MeasureCandidatesPilot => {
            let directory = benchmarks_dir().join("inventory");
            if let Err(error) = candidate_measure::measure_candidates(&directory, true, false) {
                eprintln!("Candidate measurement pilot failed: {error}");
                std::process::exit(1);
            }
        }
        Command::RetryCandidates => {
            let directory = benchmarks_dir().join("inventory");
            if let Err(error) = candidate_measure::measure_candidates(&directory, false, true) {
                eprintln!("Candidate retry failed: {error}");
                std::process::exit(1);
            }
        }
        Command::ExportComposition => {
            let directory = benchmarks_dir().join("inventory");
            if let Err(error) = composition::export_composition(&directory) {
                eprintln!("Instruction composition export failed: {error}");
                std::process::exit(1);
            }
        }
    }
}
