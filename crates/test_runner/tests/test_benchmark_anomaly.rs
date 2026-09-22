#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Thorough Benchmark Empirical Anomaly & Architecture Invariant Regression Test Suite
//!
//! Validates empirical performance bounds on the high-precision `--thorough` benchmark dataset:
//! 1. Automatically invokes `tools/benchmarks/analyze_benchmarks.py` to regenerate
//!    `tests/benchmarks/thorough/analysis_report.md`.
//! 2. Asserts Zero Anomalies (`anomaly == false`) across all 108 specifications.
//! 3. Asserts 16-bit Register ALU Functional Family Symmetry (spread <= 10.0%).
//! 4. Asserts Addressing Mode Latency Ladder Monotonicity (Reg < Indirect < Displacement < AbsoluteLong).
//! 5. Asserts Statistical Noise Bound (Jitter CV < 3.0%).
//! 6. Asserts Normalized Emulation Tax ($R_{norm} \le 4.5$ ns/CCK for ALU and data movement).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Locates repository root for accessing `tests/benchmarks/` and `tools/` on disk
fn find_repo_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    while !dir.join("ROADMAP.md").exists() {
        if !dir.pop() {
            return None;
        }
    }
    Some(dir)
}

/// Detects available python executable (`python` or `python3`)
fn detect_python() -> Option<&'static str> {
    if Command::new("python").arg("--version").output().is_ok() {
        Some("python")
    } else if Command::new("python3").arg("--version").output().is_ok() {
        Some("python3")
    } else {
        None
    }
}

/// Resolves the thorough CSV dataset path (`m68k_benchmark_baseline.csv`, `m68k_benchmark_latest.csv`, or `m68k_benchmark.csv`)
fn find_thorough_csv(repo_root: &Path) -> Option<PathBuf> {
    let baseline = repo_root.join("tests/benchmarks/m68k_benchmark_baseline.csv");
    if baseline.exists() {
        return Some(baseline);
    }
    let latest = repo_root.join("tests/benchmarks/thorough/m68k_benchmark_latest.csv");
    if latest.exists() {
        return Some(latest);
    }
    let canonical = repo_root.join("tests/benchmarks/thorough/m68k_benchmark.csv");
    if canonical.exists() {
        return Some(canonical);
    }
    None
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct CsvRecord {
    mnemonic: String,
    variant: String,
    mode: String,
    category: String,
    amiga_cck: u32,
    host_ns_op: f64,
    host_ns_cck: f64,
    host_mips: f64,
    jitter_pct: f64,
    anomaly: bool,
}

fn load_thorough_records(path: &Path) -> Result<Vec<CsvRecord>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read CSV: {}", e))?;
    let mut records = Vec::with_capacity(108);

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || idx == 0 {
            continue;
        }
        let cols: Vec<&str> = trimmed.split(',').collect();
        if cols.len() < 14 {
            return Err(format!(
                "Malformed CSV line {} in {}: expected 14 columns, got {}",
                idx + 1,
                path.display(),
                cols.len()
            ));
        }

        records.push(CsvRecord {
            mnemonic: cols[1].to_string(),
            variant: cols[2].to_string(),
            mode: cols[3].to_string(),
            category: cols[4].to_string(),
            amiga_cck: cols[6].parse().unwrap_or(0),
            host_ns_op: cols[9].parse().unwrap_or(0.0),
            host_ns_cck: cols[10].parse().unwrap_or(0.0),
            host_mips: cols[11].parse().unwrap_or(0.0),
            jitter_pct: cols[12].parse().unwrap_or(0.0),
            anomaly: cols[13].trim().eq_ignore_ascii_case("true"),
        });
    }

    Ok(records)
}

#[test]
fn test_thorough_analysis_report_generation() {
    let repo_root = match find_repo_root() {
        Some(r) => r,
        None => return,
    };

    let thorough_csv = match find_thorough_csv(&repo_root) {
        Some(p) => p,
        None => {
            println!("[*] Skipping thorough report generation: thorough CSV not found on disk");
            return;
        }
    };

    let py_bin = match detect_python() {
        Some(py) => py,
        None => {
            println!(
                "[!] Python not found in PATH; skipping automated analysis_report.md execution"
            );
            return;
        }
    };

    let script_path = repo_root.join("tools/benchmarks/analyze_benchmarks.py");
    let report_out = repo_root.join("tests/benchmarks/thorough/analysis_report.md");

    let output = Command::new(py_bin)
        .arg(&script_path)
        .arg(&thorough_csv)
        .arg("--output")
        .arg(&report_out)
        .output()
        .expect("Failed to execute analyze_benchmarks.py");

    assert!(
        output.status.success(),
        "analyze_benchmarks.py failed with stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        report_out.exists(),
        "Expected generated analysis report at {}",
        report_out.display()
    );

    let report_text = fs::read_to_string(&report_out).expect("Failed to read generated report");
    assert!(
        report_text.contains("# M68000 Instruction Benchmark Analysis Report"),
        "Report header missing or malformed in {}",
        report_out.display()
    );
    assert!(
        report_text.contains("Functional Family Symmetry"),
        "Functional Family Symmetry section missing in report"
    );
    assert!(
        report_text.contains("Addressing Mode Latency Ladder"),
        "Addressing Mode section missing in report"
    );

    println!(
        "[*] Successfully regenerated and verified {}",
        report_out.display()
    );
}

#[test]
fn test_thorough_benchmark_empirical_anomalies_and_invariants() {
    let repo_root = match find_repo_root() {
        Some(r) => r,
        None => return,
    };

    let thorough_csv = match find_thorough_csv(&repo_root) {
        Some(p) => p,
        None => {
            println!("[*] Skipping thorough anomaly verification: thorough CSV not found on disk");
            return;
        }
    };

    let records = load_thorough_records(&thorough_csv).unwrap_or_else(|e| {
        panic!(
            "Failed to load records from {}: {}",
            thorough_csv.display(),
            e
        )
    });

    assert_eq!(
        records.len(),
        108,
        "Thorough benchmark dataset must contain all 108 specifications"
    );

    // 1. Zero Anomalies Check (Column 13 must be false for all specs in thorough run)
    let anomalies: Vec<&CsvRecord> = records.iter().filter(|r| r.anomaly).collect();
    if !anomalies.is_empty() {
        let details: Vec<String> = anomalies
            .iter()
            .map(|r| {
                format!(
                    "  - [{}] {} ({}): host_ns_op={:.2}, host_mips={:.1}, jitter={:.2}%",
                    r.mnemonic, r.variant, r.mode, r.host_ns_op, r.host_mips, r.jitter_pct
                )
            })
            .collect();
        panic!(
            "\n🚨 SEVERE ANOMALIES DETECTED IN THOROUGH BENCHMARK ({} specifications flagged):\n{}\n\
             ACTION: Investigate instruction implementation in crates/cpu/src/instructions/ for branch stalls or inlining defects.\n",
            anomalies.len(),
            details.join("\n")
        );
    }

    // 2. Functional Family Symmetry Check (16-bit Register ALU)
    // ADD.W, SUB.W, AND.W, OR.W, EOR.W, CMP.W should execute with tight latency parity (spread <= 10.0%)
    let symmetry_targets = [
        "ADD.W D1  D0",
        "SUB.W D1  D0",
        "AND.W D1  D0",
        "OR.W D1  D0",
        "EOR.W D1  D0",
        "CMP.W D1  D0",
    ];
    let mut symmetry_samples = Vec::new();
    for target in &symmetry_targets {
        if let Some(matched) = records.iter().find(|r| r.variant == *target) {
            symmetry_samples.push(matched.host_ns_op);
        }
    }

    assert_eq!(
        symmetry_samples.len(),
        6,
        "All 6 symmetry reference operations must be present in thorough run"
    );

    let min_ns = symmetry_samples
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let max_ns = symmetry_samples
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let spread_pct = ((max_ns - min_ns) / min_ns) * 100.0;

    println!(
        "[*] 16-bit ALU Family Latency: Min = {:.2} ns, Max = {:.2} ns, Spread = {:.1}%",
        min_ns, max_ns, spread_pct
    );
    assert!(
        spread_pct <= 10.0,
        "16-bit ALU Functional Family Symmetry broken! Spread was {:.1}% (limit: <= 10.0%)",
        spread_pct
    );

    // 3. Addressing Mode Latency Ladder Monotonicity (MOVE.W)
    let move_direct = records
        .iter()
        .find(|r| r.mnemonic == "MOVE" && r.variant == "MOVE.W D1  D0")
        .expect("MOVE.W D1 D0 missing");
    let move_indirect = records
        .iter()
        .find(|r| r.mnemonic == "MOVE" && r.variant == "MOVE.W (A0)  D0")
        .expect("MOVE.W (A0) D0 missing");
    let move_disp = records
        .iter()
        .find(|r| r.mnemonic == "MOVE" && r.variant == "MOVE.W 16(A0)  D0")
        .expect("MOVE.W 16(A0) D0 missing");
    let move_absl = records
        .iter()
        .find(|r| r.mnemonic == "MOVE" && r.variant == "MOVE.W ($00002000).L  D0")
        .expect("MOVE.W ($00002000).L D0 missing");

    assert!(
        move_indirect.host_ns_op > move_direct.host_ns_op,
        "Memory indirect ({:.2} ns) must be slower than register direct ({:.2} ns)",
        move_indirect.host_ns_op,
        move_direct.host_ns_op
    );
    assert!(
        move_disp.host_ns_op > move_indirect.host_ns_op,
        "Displacement with extension word ({:.2} ns) must be slower than indirect ({:.2} ns)",
        move_disp.host_ns_op,
        move_indirect.host_ns_op
    );
    assert!(
        move_absl.host_ns_op > move_disp.host_ns_op,
        "Absolute Long with 2 extension words ({:.2} ns) must be slower than 1 extension word ({:.2} ns)",
        move_absl.host_ns_op,
        move_disp.host_ns_op
    );

    // 4. Jitter Upper Bound (CV < 3.0% on dedicated thorough run)
    for r in &records {
        assert!(
            r.jitter_pct < 3.0,
            "Excessive host jitter detected on [{}] {}: {:.2}% (limit: < 3.0%)",
            r.mnemonic,
            r.variant,
            r.jitter_pct
        );
    }

    // 5. Normalized Emulation Tax Upper Bound
    for r in &records {
        if r.category == "Arithmetic" || r.category == "Logic" || r.category == "DataMovement" {
            assert!(
                r.host_ns_cck <= 4.5,
                "Elevated emulation tax on [{}] {}: {:.3} ns/CCK (limit: <= 4.5 ns/CCK)",
                r.mnemonic,
                r.variant,
                r.host_ns_cck
            );
        }
    }

    println!(
        "[*] Verified 108 specifications in thorough benchmark: 0 anomalies, tight symmetry, monotonic addressing mode ladders"
    );
}
