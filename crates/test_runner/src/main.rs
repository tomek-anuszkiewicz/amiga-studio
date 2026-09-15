//! CLI tool for inspecting SingleStepTest results, coverage summaries, and regression diffs

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use test_runner::reporter::{resolve_results_dir, GlobalTestSummary, SuiteResult};
use test_runner::vamiga::{
    run_vamiga_suite, run_vamiga_test, VamigaCatalog, VamigaCategory, VamigaRunConfig,
    VamigaTestStatus,
};

fn print_usage() {
    println!("M68000 SingleStepTest & vAmigaTS Verification Runner");
    println!("Usage: cargo run -p test_runner -- [COMMAND]\n");
    println!("Commands:");
    println!("  bench [OPTIONS]  Execute M68000 instruction benchmarking suite");
    println!("                   Options: --quick, --standard, --thorough");
    println!("                            --filter <TAG>, --unroll <K>, --passes <N>");
    println!("                            --out-dir <PATH>, --no-pin, --dump-traces");
    println!("  benchmark-chipset [OPTIONS] Run chipset throughput benchmark & regression check");
    println!(
        "                   Options: --record (record golden baseline), --compare, --frames <N>"
    );
    println!("  vamiga [OPTIONS] Execute vAmigaTS regression verification test harness");
    println!("                   Options: --category <CAT>, --test <NAME>, --frames <N>");
    println!("                            --max-tests <N>, --list-deferred, --summary, -v");
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
    let harte_path = format!("ref_src/SingleStepTests-680x0/68000/v1/{}.json", opcode);

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

fn run_vamiga_cli(args: &[String]) {
    let repo_root = if Path::new("ref_src/vAmigaTS").is_dir() {
        PathBuf::from("ref_src/vAmigaTS")
    } else if Path::new("../../ref_src/vAmigaTS").is_dir() {
        PathBuf::from("../../ref_src/vAmigaTS")
    } else {
        eprintln!("Error: Cannot find ref_src/vAmigaTS directory.");
        return;
    };

    let mut category = VamigaCategory::All;
    let mut specific_test: Option<String> = None;
    let mut list_deferred = false;
    let mut show_summary = true;
    let mut max_tests: Option<usize> = None;
    let mut config = VamigaRunConfig::default();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--category" | "-c" => {
                if i + 1 < args.len() {
                    if let Some(cat) = VamigaCategory::from_str_loose(&args[i + 1]) {
                        category = cat;
                    } else {
                        eprintln!("Unknown category: {}", args[i + 1]);
                        return;
                    }
                    i += 1;
                }
            }
            "--test" | "-t" => {
                if i + 1 < args.len() {
                    specific_test = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--frames" | "-f" => {
                if i + 1 < args.len() {
                    if let Ok(frames) = args[i + 1].parse::<u32>() {
                        config.frames_to_run = frames;
                    }
                    i += 1;
                }
            }
            "--max-tests" | "-n" => {
                if i + 1 < args.len() {
                    if let Ok(n) = args[i + 1].parse::<usize>() {
                        max_tests = Some(n);
                    }
                    i += 1;
                }
            }
            "--list-deferred" => {
                list_deferred = true;
            }
            "--summary" => {
                show_summary = true;
            }
            "--verbose" | "-v" => {
                config.verbose = true;
            }
            other => {
                eprintln!("Unknown vamiga option: {}", other);
            }
        }
        i += 1;
    }

    println!(
        "[*] Discovering vAmigaTS test catalog in: {}",
        repo_root.display()
    );
    let catalog = VamigaCatalog::discover(&repo_root);
    let stats = catalog.stats();

    println!(
        "[*] Catalog indexed: {} total tests ({} runnable in Phase 1 Baseline, {} deferred)",
        stats.total_tests, stats.runnable_tests, stats.deferred_tests
    );

    if list_deferred {
        println!("\n=========================================================================================");
        println!("📋 vAmigaTS DEFERRED TEST SUITES BREAKDOWN");
        println!("=========================================================================================");
        for (reason, count) in &stats.deferred_by_reason {
            println!("  • {:<55} {:>4} tests", reason, count);
        }
        println!("-----------------------------------------------------------------------------------------");
        println!("Roadmap Targets for Deferred Suites:");
        println!("  1. FPU Coprocessor Required (206)      -> Phase 3: Advanced Graphics Architecture & FPU");
        println!("  2. ECS/AGA Silicon Only (114)          -> Phase 2 (ECS) & Phase 3 (AGA)");
        println!("  3. Motorola 68010 Required (91)        -> 68010 CPU Architecture Milestone");
        println!("  4. AmigaOS Floppy Boot Required (6)    -> Step 6: Real-World Amiga Workloads (MFM Boot)");
        println!(
            "  5. Non-Visual / Photo Only (193)       -> Step 2.7: Non-Visual Register Assertions"
        );
        println!("=========================================================================================\n");
        return;
    }

    if let Some(query) = specific_test {
        let desc = match catalog.find_test(&query) {
            Some(d) => d,
            None => {
                eprintln!("Error: Test '{}' not found in catalog.", query);
                return;
            }
        };

        println!(
            "\n[*] Running single test: {} ({:?})",
            desc.name, desc.category
        );
        println!("    Rel Dir: {}", desc.rel_dir.display());
        println!("    Status:  {:?}", desc.status);

        if desc.status != VamigaTestStatus::Runnable {
            eprintln!("Cannot run deferred test in Phase 1 Baseline.");
            return;
        }

        match run_vamiga_test(desc, &config) {
            Ok(res) => {
                if res.passed {
                    println!("✅ PASS: 100% pixel match ({} pixels)", res.total_pixels);
                } else {
                    println!(
                        "❌ FAIL: {}/{} mismatched pixels ({:.2}%)",
                        res.mismatched_pixels,
                        res.total_pixels,
                        (res.mismatched_pixels as f64 / res.total_pixels as f64) * 100.0
                    );
                    for diff in res.diffs.iter().take(10) {
                        println!(
                            "    Mismatch at ({}, {}): actual={:?}, expected={:?}",
                            diff.x, diff.y, diff.actual, diff.expected
                        );
                    }
                }
            }
            Err(e) => eprintln!("Execution error: {}", e),
        }
        return;
    }

    // Batch run category
    let summary = run_vamiga_suite(&catalog, category, &config, max_tests);
    if show_summary {
        summary.print_summary();
    }
}

fn run_chipset_benchmarks_cli(args: &[String]) {
    use test_runner::benchmark::{
        compare_chipset_baseline, record_chipset_baseline, DEFAULT_BENCHMARK_FRAMES,
    };

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to locate repo root")
        .to_path_buf();

    let mut record = false;
    let mut compare = false;
    let mut frames = DEFAULT_BENCHMARK_FRAMES;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--record" => record = true,
            "--compare" => compare = true,
            "--frames" | "-f" => {
                if i + 1 < args.len() {
                    if let Ok(f) = args[i + 1].parse::<u32>() {
                        frames = f;
                    }
                    i += 1;
                }
            }
            other => eprintln!("Unknown benchmark-chipset option: {}", other),
        }
        i += 1;
    }

    if record {
        println!(
            "[*] Recording chipset performance golden baseline ({} frames)...",
            frames
        );
        if let Err(e) = record_chipset_baseline(&repo_root, frames) {
            eprintln!("Error recording baseline: {}", e);
            std::process::exit(1);
        }
    } else if compare || (!record && !compare) {
        println!(
            "[*] Running chipset performance regression audit ({} frames)...",
            frames
        );
        match compare_chipset_baseline(&repo_root, frames) {
            Ok(true) => {
                // Passed
            }
            Ok(false) => {
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Error comparing baseline: {}", e);
                std::process::exit(1);
            }
        }
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
        "benchmark-chipset" => run_chipset_benchmarks_cli(&args[2..]),
        "vamiga" => run_vamiga_cli(&args[2..]),
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
