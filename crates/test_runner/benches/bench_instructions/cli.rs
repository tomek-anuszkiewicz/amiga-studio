//! Cargo entry point for M68000 instruction benchmarks.

#[path = "../../../../benchmarks/m68k/runner.rs"]
mod asm_runner;
mod inventory;
mod platform;

fn benchmarks_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.agent/tmp/benchmarks")
}

pub const USAGE: &str = "M68000 Instruction Benchmarks
Usage: cargo bench -p test_runner --bench bench_instructions -- COMMAND

Options:
  --inventory                       Write the decoder opcode inventory
  --measure-case CASE DURATION_MS SERIES REPORT_PATH
                                    Measure one assembly case for the catalog workflow
  --help, -h                        Print these instructions

Run without options to print these instructions.
Select one command. Unknown or repeated options are errors.";

#[derive(Debug)]
pub enum Command {
    Help,
    Inventory,
    MeasureCase(String, u64, usize, std::path::PathBuf),
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
    if args.len() == 5 && args[0] == "--measure-case" {
        let duration = args[2]
            .parse::<u64>()
            .map_err(|_| "duration must be positive")?;
        let series = args[3]
            .parse::<usize>()
            .map_err(|_| "series must be positive")?;
        if duration == 0 || series == 0 {
            return Err("duration and series must be positive".to_owned());
        }
        return Ok(Command::MeasureCase(
            args[1].clone(),
            duration,
            series,
            args[4].clone().into(),
        ));
    }
    let [option] = args else {
        return Err("Select one benchmark command".to_owned());
    };
    match option.as_str() {
        "--help" | "-h" => return Ok(Command::Help),
        "--inventory" => return Ok(Command::Inventory),
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
        Command::MeasureCase(case_id, duration, series, report_path) => {
            if let Err(error) = asm_runner::measure(&case_id, duration, series, &report_path) {
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
    }
}
