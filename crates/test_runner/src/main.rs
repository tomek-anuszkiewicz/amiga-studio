#![allow(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! CLI tool for inspecting SingleStepTest results, coverage summaries, and regression diffs

mod cli;

use cli::Command;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use test_runner::reporter::{resolve_results_dir, GlobalTestSummary, SuiteResult};

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
    let harte_path = format!("ref_src/SingleStepTests-680x0/68000/v1/{}.json", opcode);

    println!("Running Real 68k (Tom Harte) suite for '{}'...", opcode);
    match test_runner::runner::run_test_file(&harte_path, Some(50)) {
        Ok((p, f)) => println!("Real 68k {}: {} passed, {} failed", opcode, p, f),
        Err(e) => eprintln!("Failed Real 68k test: {}", e),
    }
}

fn run_benchmarks_cli(config: &test_runner::benchmark::BenchmarkConfig) {
    if let Err(e) = test_runner::benchmark::run_benchmark_suite(config) {
        eprintln!("Benchmark failed: {}", e);
        std::process::exit(1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match cli::parse(&args) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("Error: {error}\nUse 'help' for command instructions.");
            std::process::exit(2);
        }
    };
    match command {
        Command::Help => println!("{}", cli::USAGE),
        Command::Bench(config) => run_benchmarks_cli(&config),
        Command::Summary => print_summary(&resolve_results_dir()),
        Command::Diff => print_diff(&resolve_results_dir()),
        Command::Suite(opcode) => run_specific_suite(&opcode),
    }
}
