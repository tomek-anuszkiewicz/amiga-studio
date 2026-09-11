//! Results Persistence: JSON & CSV Output Serialization
//!
//! Writes structured benchmark reports and flat CSV tables to `tests/benchmarks/`
//! per Obsidian/Amiga/Design/CPU Instruction Benchmarking.md.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Top-level report container matching Section 6.1 JSON schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuiteReport {
    pub version: u32,
    pub timestamp: String,
    pub environment: BenchmarkEnvironmentInfo,
    pub results: Vec<BenchmarkItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkEnvironmentInfo {
    pub host_os: String,
    pub host_arch: String,
    pub rustc_version: String,
    pub profile: String,
    pub unroll_factor: usize,
    pub outer_passes: usize,
    pub p_core_pinning_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkItemResult {
    pub mnemonic: String,
    pub variant: String,
    pub addressing_mode: String,
    pub category: String,
    pub opcode_hex: String,
    pub amiga_cck_cycles: u32,
    pub total_guest_instructions: u64,
    pub total_guest_cck: u64,
    pub host_duration_median_ms: f64,
    pub host_duration_min_ms: f64,
    pub host_duration_max_ms: f64,
    pub host_jitter_pct: f64,
    pub host_ns_per_instruction: f64,
    pub host_ns_per_guest_cck: f64,
    pub host_mips: f64,
    pub differential_net_ns_op: Option<f64>,
    pub anomaly_flag: bool,
    pub anomaly_notes: Vec<String>,
}

/// Generates an ISO-8601-like timestamp string suitable for filenames (e.g. 20260910_120000)
pub fn format_timestamp_filename() -> String {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Rough YMD HMS calculation from unix seconds for deterministic naming
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;
    format!("epoch_{:08}_{:02}{:02}{:02}", days, hours, mins, seconds)
}

/// Persists the full benchmark report to JSON and CSV in `out_dir`
pub fn persist_benchmark_report(
    report: &BenchmarkSuiteReport,
    out_dir: &Path,
) -> Result<(PathBuf, PathBuf), String> {
    if !out_dir.exists() {
        fs::create_dir_all(out_dir).map_err(|e| format!("Failed to create out dir: {}", e))?;
    }

    let canonical_json_path = out_dir.join("m68k_benchmark.json");
    let canonical_csv_path = out_dir.join("m68k_benchmark.csv");
    let latest_json_path = out_dir.join("m68k_benchmark_latest.json");
    let latest_csv_path = out_dir.join("m68k_benchmark_latest.csv");
    let previous_json_path = out_dir.join("m68k_benchmark_previous.json");
    let previous_csv_path = out_dir.join("m68k_benchmark_previous.csv");

    // 0. Two-tier rotation: preserve existing current/latest results as previous before overwriting
    if canonical_json_path.exists() {
        let _ = fs::copy(&canonical_json_path, &previous_json_path);
    }
    if canonical_csv_path.exists() {
        let _ = fs::copy(&canonical_csv_path, &previous_csv_path);
    }

    // 1. Serialize and write JSON (always overwriting canonical and latest)
    let json_content = serde_json::to_string_pretty(report)
        .map_err(|e| format!("JSON serialization error: {}", e))?;
    fs::write(&canonical_json_path, &json_content).map_err(|e| {
        format!(
            "Failed to write JSON {}: {}",
            canonical_json_path.display(),
            e
        )
    })?;
    let _ = fs::write(&latest_json_path, &json_content);

    // 2. Serialize and write CSV (always overwriting canonical and latest)
    let mut csv_lines = Vec::new();
    csv_lines.push(
        "timestamp,mnemonic,variant,addressing_mode,category,opcode_hex,amiga_cck_cycles,total_guest_instructions,host_duration_median_ms,host_ns_per_instruction,host_ns_per_guest_cck,host_mips,host_jitter_pct,anomaly_flag".to_string()
    );

    for item in &report.results {
        csv_lines.push(format!(
            "{},{},{},{},{},{},{},{},{:.2},{:.2},{:.3},{:.2},{:.2},{}",
            report.timestamp,
            item.mnemonic,
            item.variant.replace(',', " "),
            item.addressing_mode,
            item.category,
            item.opcode_hex,
            item.amiga_cck_cycles,
            item.total_guest_instructions,
            item.host_duration_median_ms,
            item.host_ns_per_instruction,
            item.host_ns_per_guest_cck,
            item.host_mips,
            item.host_jitter_pct,
            item.anomaly_flag,
        ));
    }

    let csv_content = csv_lines.join("\n");
    fs::write(&canonical_csv_path, &csv_content).map_err(|e| {
        format!(
            "Failed to write CSV {}: {}",
            canonical_csv_path.display(),
            e
        )
    })?;
    let _ = fs::write(&latest_csv_path, &csv_content);

    Ok((canonical_json_path, canonical_csv_path))
}

/// Attempts to load the previous benchmark report from `out_dir/m68k_benchmark_previous.json`.
pub fn load_previous_report(out_dir: &Path) -> Option<BenchmarkSuiteReport> {
    let previous_path = out_dir.join("m68k_benchmark_previous.json");
    if previous_path.exists() {
        let content = fs::read_to_string(previous_path).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

/// Attempts to load the historical baseline report from `out_dir/m68k_benchmark_baseline.json`,
/// or if not found, checks the parent directory (`out_dir/../m68k_benchmark_baseline.json`).
pub fn load_historical_baseline(out_dir: &Path) -> Option<BenchmarkSuiteReport> {
    let baseline_path = out_dir.join("m68k_benchmark_baseline.json");
    if baseline_path.exists() {
        let content = fs::read_to_string(baseline_path).ok()?;
        return serde_json::from_str(&content).ok();
    }
    if let Some(parent) = out_dir.parent() {
        let parent_baseline = parent.join("m68k_benchmark_baseline.json");
        if parent_baseline.exists() {
            let content = fs::read_to_string(parent_baseline).ok()?;
            return serde_json::from_str(&content).ok();
        }
    }
    None
}
