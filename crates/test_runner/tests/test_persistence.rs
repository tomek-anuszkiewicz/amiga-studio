#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Unit tests for M68000 Benchmark Results Persistence & Baseline Loading
//!
//! Validates timestamp filename formatting, JSON/CSV serialization roundtrips,
//! canonical overwrite invariance, and historical baseline discovery.

use std::fs;
use std::path::PathBuf;
use test_runner::benchmark::persistence::{
    format_timestamp_filename, load_historical_baseline, load_previous_report,
    persist_benchmark_report, BenchmarkEnvironmentInfo, BenchmarkItemResult, BenchmarkSuiteReport,
};

fn create_temp_test_dir(sub: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "amiga_bench_persist_test_{}_{}",
        sub,
        std::process::id()
    ));
    if dir.exists() {
        let _ = fs::remove_dir_all(&dir);
    }
    fs::create_dir_all(&dir).expect("Failed to create temp dir");
    dir
}

fn create_mock_report() -> BenchmarkSuiteReport {
    BenchmarkSuiteReport {
        version: 1,
        timestamp: "2026-09-11T20:00:00Z".to_string(),
        environment: BenchmarkEnvironmentInfo {
            host_os: "windows".to_string(),
            host_arch: "x86_64".to_string(),
            rustc_version: "rustc 1.85.0".to_string(),
            profile: "quick".to_string(),
            unroll_factor: 700,
            outer_passes: 3,
            p_core_pinning_active: true,
        },
        results: vec![
            BenchmarkItemResult {
                mnemonic: "NOP".to_string(),
                variant: "NOP".to_string(),
                addressing_mode: "Implied".to_string(),
                category: "Baseline".to_string(),
                opcode_hex: "4E71".to_string(),
                amiga_cck_cycles: 4,
                total_guest_instructions: 31500,
                total_guest_cck: 126000,
                host_duration_median_ms: 0.15,
                host_duration_min_ms: 0.14,
                host_duration_max_ms: 0.16,
                host_jitter_pct: 0.5,
                host_ns_per_instruction: 14.1,
                host_ns_per_guest_cck: 3.525,
                host_mips: 70.9,
                differential_net_ns_op: None,
                anomaly_flag: false,
                anomaly_notes: Vec::new(),
            },
            BenchmarkItemResult {
                mnemonic: "ADD".to_string(),
                variant: "ADD.W D1  D0".to_string(),
                addressing_mode: "DataRegDirect".to_string(),
                category: "Arithmetic".to_string(),
                opcode_hex: "D041".to_string(),
                amiga_cck_cycles: 4,
                total_guest_instructions: 31500,
                total_guest_cck: 126000,
                host_duration_median_ms: 0.16,
                host_duration_min_ms: 0.15,
                host_duration_max_ms: 0.17,
                host_jitter_pct: 0.7,
                host_ns_per_instruction: 15.5,
                host_ns_per_guest_cck: 3.875,
                host_mips: 64.5,
                differential_net_ns_op: Some(1.4),
                anomaly_flag: false,
                anomaly_notes: Vec::new(),
            },
        ],
    }
}

#[test]
fn test_persistence_timestamp_format() {
    let ts = format_timestamp_filename();
    assert!(
        ts.starts_with("epoch_"),
        "Timestamp filename must begin with 'epoch_'"
    );

    // Format: epoch_DDDDDDDD_HHMMSS (length: 6 + 8 + 1 + 6 = 21)
    assert_eq!(
        ts.len(),
        21,
        "Timestamp filename must be 21 chars, got: {}",
        ts
    );
    let parts: Vec<&str> = ts.split('_').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "epoch");
    assert!(parts[1].chars().all(|c| c.is_ascii_digit()));
    assert!(parts[2].chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn test_persistence_load_historical_baseline_missing() {
    let temp_dir = create_temp_test_dir("missing");
    let result = load_historical_baseline(&temp_dir);
    assert!(
        result.is_none(),
        "Expected None when baseline file does not exist"
    );
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_persistence_report_roundtrip_and_overwrite() {
    let temp_dir = create_temp_test_dir("roundtrip");
    let report = create_mock_report();

    // 1. Persist initial report
    let (json_path, csv_path) =
        persist_benchmark_report(&report, &temp_dir).expect("Failed to persist report");

    assert!(json_path.exists(), "m68k_benchmark.json must exist");
    assert!(csv_path.exists(), "m68k_benchmark.csv must exist");
    assert!(
        temp_dir.join("m68k_benchmark_latest.json").exists(),
        "latest JSON must exist"
    );
    assert!(
        temp_dir.join("m68k_benchmark_latest.csv").exists(),
        "latest CSV must exist"
    );

    // 2. Deserialize JSON and assert roundtrip equality
    let json_bytes = fs::read_to_string(&json_path).expect("Failed to read JSON");
    let loaded_report: BenchmarkSuiteReport =
        serde_json::from_str(&json_bytes).expect("Failed to parse JSON");

    assert_eq!(loaded_report.version, report.version);
    assert_eq!(loaded_report.results.len(), 2);
    assert_eq!(loaded_report.results[0].mnemonic, "NOP");
    assert_eq!(loaded_report.results[1].mnemonic, "ADD");
    assert_eq!(loaded_report.results[1].amiga_cck_cycles, 4);
    assert_eq!(loaded_report.results[1].total_guest_instructions, 31500);

    // 3. Inspect CSV content
    let csv_text = fs::read_to_string(&csv_path).expect("Failed to read CSV");
    let lines: Vec<&str> = csv_text.lines().collect();
    assert_eq!(lines.len(), 3, "Expected header + 2 data rows");
    assert!(lines[0].starts_with("timestamp,mnemonic,variant,addressing_mode,category"));
    assert!(lines[1].contains("NOP,NOP,Implied,Baseline,4E71,4,31500"));
    assert!(lines[2].contains("ADD,ADD.W D1  D0,DataRegDirect,Arithmetic,D041,4,31500"));

    // 4. Test load_historical_baseline by copying to baseline name
    let baseline_path = temp_dir.join("m68k_benchmark_baseline.json");
    fs::copy(&json_path, &baseline_path).expect("Failed to copy baseline");
    let loaded_baseline =
        load_historical_baseline(&temp_dir).expect("Failed to load historical baseline");
    assert_eq!(loaded_baseline.results.len(), 2);

    // 5. Verify two-tier rotation: prior to second run, previous report doesn't exist
    assert!(
        !temp_dir.join("m68k_benchmark_previous.json").exists(),
        "previous JSON should not exist before second run"
    );
    assert!(
        !temp_dir.join("m68k_benchmark_previous.csv").exists(),
        "previous CSV should not exist before second run"
    );

    // Create a modified report for the second run
    let mut report2 = report.clone();
    report2.timestamp = "2026-09-11T12:00:00Z".to_string();
    let _ = persist_benchmark_report(&report2, &temp_dir);

    // After second run, previous files must exist and contain report 1
    assert!(
        temp_dir.join("m68k_benchmark_previous.json").exists(),
        "previous JSON must exist after second run"
    );
    assert!(
        temp_dir.join("m68k_benchmark_previous.csv").exists(),
        "previous CSV must exist after second run"
    );

    let previous_loaded = load_previous_report(&temp_dir)
        .expect("Failed to load previous report via load_previous_report");
    assert_eq!(previous_loaded.timestamp, report.timestamp);

    let current_loaded: BenchmarkSuiteReport = serde_json::from_str(
        &fs::read_to_string(temp_dir.join("m68k_benchmark.json"))
            .expect("Failed to read current JSON"),
    )
    .expect("Failed to parse current JSON");
    assert_eq!(current_loaded.timestamp, report2.timestamp);

    let entries: Vec<_> = fs::read_dir(&temp_dir)
        .expect("Failed to read dir")
        .flatten()
        .collect();

    // Directory should contain canonical + latest + previous + baseline, zero epoch files:
    let epoch_files_count = entries
        .iter()
        .filter(|e| e.file_name().to_string_lossy().contains("epoch_"))
        .count();
    assert_eq!(
        epoch_files_count, 0,
        "No epoch files should be created by persist_benchmark_report"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
