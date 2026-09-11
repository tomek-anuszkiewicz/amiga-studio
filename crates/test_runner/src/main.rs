//! CLI tool for inspecting SingleStepTest results, coverage summaries, and regression diffs

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use test_runner::reporter::{resolve_results_dir, GlobalTestSummary, SuiteResult};

fn print_usage() {
    println!("M68000 SingleStepTest Runner & Diagnostic Tool");
    println!("Usage: cargo run -p test_runner -- [COMMAND]\n");
    println!("Commands:");
    println!("  bench [OPTIONS]  Execute M68000 instruction benchmarking suite");
    println!("                   Options: --quick, --standard, --thorough");
    println!("                            --filter <TAG>, --unroll <K>, --passes <N>");
    println!("                            --out-dir <PATH>, --no-pin, --dump-traces");
    println!("  --summary        Print global pass/fail coverage table across all tested opcodes");
    println!("  --diff           Compare latest test runs against previous runs to detect regressions/fixes");
    println!(
        "  --suite <OP>     Execute and report diagnostics for a specific opcode (e.g. ADD.b)"
    );
    println!("  --help           Print this help message");
}

fn print_summary(results_dir: &Path) {
    let summary_file = results_dir.join("summary.json");
    if !summary_file.exists() {
        eprintln!(
            "No test results found at {:?}. Run tests first using 'cargo test -p test_runner'.",
            summary_file
        );
        return;
    }

    let file = File::open(&summary_file).expect("Failed to open summary.json");
    let summary: GlobalTestSummary =
        serde_json::from_reader(BufReader::new(file)).expect("Failed to parse summary.json");

    println!("\n=========================================================================================");
    println!("📊 M68000 SINGLESTEPTEST COVERAGE SUMMARY");
    println!(
        "========================================================================================="
    );
    println!("Suites Tested:    {}", summary.total_suites);
    println!("Total Executed:   {}", summary.total_executed);
    println!("Total Passed:     {}", summary.total_passed);
    println!("Total Failed:     {}", summary.total_failed);
    println!(
        "Overall Pass Rate: {:.2}%\n",
        summary.overall_pass_rate_percent
    );

    println!(
        "{:<32} {:>10} {:>10} {:>10} {:>12}",
        "Suite Name", "Executed", "Passed", "Failed", "Pass Rate"
    );
    println!("{}", "-".repeat(89));

    for (name, stats) in &summary.suites {
        let status_indicator = if stats.failed == 0 { "✅" } else { "❌" };
        println!(
            "{:<30} {} {:>10} {:>10} {:>10} {:>11.1}%",
            name,
            status_indicator,
            stats.total,
            stats.passed,
            stats.failed,
            stats.pass_rate_percent
        );
        if !stats.active_failures.is_empty() {
            for failure in stats.active_failures.iter().take(3) {
                println!("    ↳ Failed: \"{}\"", failure);
            }
            if stats.active_failures.len() > 3 {
                println!(
                    "    ↳ ... and {} more failures",
                    stats.active_failures.len() - 3
                );
            }
        }
    }
    println!("=========================================================================================\n");
}

fn print_diff(results_dir: &Path) {
    let latest_dir = results_dir.join("latest");
    let previous_dir = results_dir.join("previous");

    if !latest_dir.exists() || !previous_dir.exists() {
        eprintln!("Both 'latest' and 'previous' result sets are required to compute diffs.");
        eprintln!("Run tests at least twice to accumulate differential history.");
        return;
    }

    println!("\n=========================================================================================");
    println!("🔍 REGRESSION & DIFFERENTIAL REPORT (Latest Run vs Previous Run)");
    println!(
        "========================================================================================="
    );

    let mut total_regressions = 0;
    let mut total_improvements = 0;

    if let Ok(entries) = fs::read_dir(&latest_dir) {
        for entry in entries.flatten() {
            let latest_path = entry.path();
            if latest_path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let file_name = latest_path.file_name().unwrap();
            let previous_path = previous_dir.join(file_name);

            if !previous_path.exists() {
                continue;
            }

            let latest: SuiteResult = match File::open(&latest_path)
                .ok()
                .and_then(|f| serde_json::from_reader(BufReader::new(f)).ok())
            {
                Some(r) => r,
                None => continue,
            };

            let prev: SuiteResult = match File::open(&previous_path)
                .ok()
                .and_then(|f| serde_json::from_reader(BufReader::new(f)).ok())
            {
                Some(r) => r,
                None => continue,
            };

            let prev_passed: HashSet<_> = prev.passed_test_names.iter().collect();
            let prev_failed: HashSet<_> = prev.failures.iter().map(|f| &f.test_name).collect();

            let cur_failed: HashSet<_> = latest.failures.iter().map(|f| &f.test_name).collect();
            let cur_passed: HashSet<_> = latest.passed_test_names.iter().collect();

            let regressions: Vec<_> = cur_failed.intersection(&prev_passed).cloned().collect();
            let improvements: Vec<_> = cur_passed.intersection(&prev_failed).cloned().collect();

            if !regressions.is_empty() || !improvements.is_empty() {
                println!("Suite: {}", latest.suite_name);
                for reg in &regressions {
                    println!("  🔴 REGRESSION: \"{}\" previously PASSED, now FAILED", reg);
                    total_regressions += 1;
                }
                for imp in &improvements {
                    println!(
                        "  🟢 IMPROVEMENT: \"{}\" previously FAILED, now PASSED",
                        imp
                    );
                    total_improvements += 1;
                }
            }
        }
    }

    if total_regressions == 0 && total_improvements == 0 {
        println!("No status changes detected between runs. All test outcomes remain identical.");
    } else {
        println!(
            "\nDifferential Summary: {} regressions, {} improvements.",
            total_regressions, total_improvements
        );
    }
    println!("=========================================================================================\n");
}

fn run_specific_suite(opcode: &str) {
    let mame_path = format!("ref_src/SingleStepTests-m68000/v1/{}.json", opcode);
    let harte_path = format!("ref_src/SingleStepTests-680x0/68000/v1/{}.json", opcode);

    println!("Running MAME suite for '{}'...", opcode);
    match test_runner::runner::run_test_file(&mame_path, Some(50)) {
        Ok((p, f)) => println!("MAME {}: {} passed, {} failed", opcode, p, f),
        Err(e) => eprintln!("Failed MAME test: {}", e),
    }

    println!("Running Real 68k (Tom Harte) suite for '{}'...", opcode);
    match test_runner::runner::run_test_file(&harte_path, Some(50)) {
        Ok((p, f)) => println!("Real 68k {}: {} passed, {} failed", opcode, p, f),
        Err(e) => eprintln!("Failed Real 68k test: {}", e),
    }
}

fn run_benchmarks_cli(args: &[String]) {
    use std::path::PathBuf;
    use test_runner::benchmark::{
        dump_benchmark_traces, filter_specs, run_benchmark_suite, BenchmarkConfig, BenchmarkProfile,
    };

    let mut config = BenchmarkConfig::default();
    let mut dump_traces = false;
    let mut i = 0;
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
            "--dump-traces" => {
                dump_traces = true;
            }
            other => {
                eprintln!("Unknown benchmark flag: {}", other);
            }
        }
        i += 1;
    }

    if dump_traces {
        let pattern = config.filter.as_deref().unwrap_or("*");
        let specs = filter_specs(pattern);
        let unroll = config.unroll_k.min(20);
        match dump_benchmark_traces(&specs, &config.out_dir, unroll) {
            Ok(n) => {
                println!(
                    "[*] Successfully generated {} execution audit traces in: {}/traces/",
                    n,
                    config.out_dir.display()
                );
            }
            Err(e) => {
                eprintln!("Error dumping traces: {}", e);
            }
        }
    }

    if let Err(e) = run_benchmark_suite(&config) {
        eprintln!("Benchmark failed: {}", e);
        std::process::exit(1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let results_dir = resolve_results_dir();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "bench" => run_benchmarks_cli(&args[2..]),
        "--summary" => print_summary(&results_dir),
        "--diff" => print_diff(&results_dir),
        "--suite" => {
            if args.len() > 2 {
                run_specific_suite(&args[2]);
            } else {
                eprintln!("Error: Missing opcode name after --suite (e.g. --suite ADD.b)");
            }
        }
        "--help" | "-h" => print_usage(),
        other => {
            eprintln!("Unknown option '{}'", other);
            print_usage();
        }
    }
}
