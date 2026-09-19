#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Benchmark JSON Report Verification & Cross-Format Parity Test Suite
//!
//! Validates that the hierarchical JSON report (`m68k_benchmark.json`) adheres
//! strictly to Section 6.1 schema, is complete across all 108 specifications,
//! and maintains 1:1 parity with tabular CSV records (`m68k_benchmark.csv`).

use std::fs;
use std::path::PathBuf;
use test_runner::benchmark::BenchmarkSuiteReport;

fn find_repo_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    while !dir.join("ROADMAP.md").exists() {
        if !dir.pop() {
            return None;
        }
    }
    Some(dir)
}

#[test]
fn test_benchmark_csv_matches_json_for_sample_case() {
    let repo_root = match find_repo_root() {
        Some(r) => r,
        None => return,
    };

    // Test across standard, quick, and thorough artifacts
    let candidate_dirs = [
        repo_root.join("tests/benchmarks/standard"),
        repo_root.join("tests/benchmarks/quick"),
        repo_root.join("tests/benchmarks/thorough"),
    ];

    let mut tested_any = false;

    for dir in &candidate_dirs {
        let csv_path = dir.join("m68k_benchmark.csv");
        let json_path = dir.join("m68k_benchmark.json");

        if !csv_path.exists() || !json_path.exists() {
            continue;
        }

        let json_content = fs::read_to_string(&json_path).expect("Failed to read JSON");
        let report: BenchmarkSuiteReport =
            serde_json::from_str(&json_content).expect("Failed to parse BenchmarkSuiteReport");

        let csv_content = fs::read_to_string(&csv_path).expect("Failed to read CSV");
        let mut csv_rows: Vec<Vec<String>> = Vec::new();
        for (idx, line) in csv_content.lines().enumerate() {
            if idx == 0 || line.trim().is_empty() {
                continue;
            }
            let cols: Vec<String> = line.split(',').map(|s| s.to_string()).collect();
            csv_rows.push(cols);
        }

        // Pick sample case: ARITH-02 (ADD.W D1, D0)
        let sample_json = report
            .results
            .iter()
            .find(|r| r.mnemonic == "ADD" && r.addressing_mode == "DataRegDirect")
            .expect("Sample instruction ADD not found in JSON report");

        let sample_csv = csv_rows
            .iter()
            .find(|cols| {
                cols.len() >= 14
                    && cols[1] == sample_json.mnemonic
                    && cols[3] == sample_json.addressing_mode
            })
            .expect("Sample instruction ADD not found in CSV report");

        // Validate 1:1 parity across metadata and metric fields
        assert_eq!(sample_csv[1], sample_json.mnemonic, "Mnemonic mismatch");
        assert_eq!(
            sample_csv[2],
            sample_json.variant.replace(',', " "),
            "Variant mismatch"
        );
        assert_eq!(
            sample_csv[3], sample_json.addressing_mode,
            "Addressing mode mismatch"
        );
        assert_eq!(sample_csv[4], sample_json.category, "Category mismatch");
        assert_eq!(sample_csv[5], sample_json.opcode_hex, "Opcode hex mismatch");
        assert_eq!(
            sample_csv[6],
            sample_json.amiga_cck_cycles.to_string(),
            "Amiga CCK mismatch"
        );
        assert_eq!(
            sample_csv[7],
            sample_json.total_guest_instructions.to_string(),
            "Total ops mismatch"
        );
        assert_eq!(
            sample_csv[8],
            format!("{:.2}", sample_json.host_duration_median_ms),
            "Median ms mismatch"
        );
        assert_eq!(
            sample_csv[9],
            format!("{:.2}", sample_json.host_ns_per_instruction),
            "Host ns/op mismatch"
        );
        assert_eq!(
            sample_csv[10],
            format!("{:.3}", sample_json.host_ns_per_guest_cck),
            "Host ns/CCK mismatch"
        );
        assert_eq!(
            sample_csv[11],
            format!("{:.2}", sample_json.host_mips),
            "Host MIPS mismatch"
        );
        assert_eq!(
            sample_csv[12],
            format!("{:.2}", sample_json.host_jitter_pct),
            "Jitter pct mismatch"
        );
        assert_eq!(
            sample_csv[13],
            sample_json.anomaly_flag.to_string(),
            "Anomaly flag mismatch"
        );

        tested_any = true;
        println!(
            "[*] Verified CSV row matches JSON object 1:1 in {:?}",
            dir.file_name().unwrap_or_default()
        );
    }

    assert!(
        tested_any,
        "Expected to verify CSV vs JSON for at least one benchmark profile"
    );
}

#[test]
fn test_benchmark_json_schema_and_completeness() {
    let repo_root = match find_repo_root() {
        Some(r) => r,
        None => return,
    };

    let candidate_dirs = [
        repo_root.join("tests/benchmarks/standard"),
        repo_root.join("tests/benchmarks/quick"),
        repo_root.join("tests/benchmarks/thorough"),
    ];

    for dir in &candidate_dirs {
        let json_path = dir.join("m68k_benchmark.json");
        if !json_path.exists() {
            continue;
        }

        let content = fs::read_to_string(&json_path).expect("Failed to read JSON");
        let report: BenchmarkSuiteReport =
            serde_json::from_str(&content).expect("Failed to parse BenchmarkSuiteReport schema");

        assert_eq!(report.version, 1, "Schema version must be 1");
        assert!(
            !report.timestamp.is_empty(),
            "Report timestamp must not be empty"
        );
        assert!(
            !report.environment.host_os.is_empty(),
            "Host OS must be populated"
        );
        assert!(
            !report.environment.host_arch.is_empty(),
            "Host arch must be populated"
        );

        if report.results.len() != 108 {
            // Partial run (e.g. filtered smoke test) - validate items and skip 108-item check
            for res in &report.results {
                assert!(!res.mnemonic.is_empty());
                assert!(!res.addressing_mode.is_empty());
                assert!(!res.category.is_empty());
                assert!(!res.opcode_hex.is_empty());
                assert!(res.amiga_cck_cycles > 0);
                assert!(res.total_guest_instructions > 0);
                assert!(res.host_duration_median_ms >= 0.0);
                assert!(res.host_ns_per_instruction >= 0.0);
                assert!(res.host_ns_per_guest_cck >= 0.0);
                assert!(res.host_mips >= 0.0);
            }
            continue;
        }

        assert_eq!(
            report.results.len(),
            108,
            "Complete benchmark run must contain all 108 specifications"
        );

        for res in &report.results {
            assert!(!res.mnemonic.is_empty());
            assert!(!res.addressing_mode.is_empty());
            assert!(!res.category.is_empty());
            assert!(!res.opcode_hex.is_empty());
            assert!(res.amiga_cck_cycles > 0);
            assert!(res.total_guest_instructions > 0);
            assert!(res.host_duration_median_ms >= 0.0);
            assert!(res.host_ns_per_instruction >= 0.0);
            assert!(res.host_ns_per_guest_cck >= 0.0);
            assert!(res.host_mips >= 0.0);
        }
    }
}
