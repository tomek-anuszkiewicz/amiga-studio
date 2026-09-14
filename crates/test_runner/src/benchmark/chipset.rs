//! Chipset Execution Benchmarking & Performance Regression Sentinel
//!
//! Provides automated throughput measurement, golden baseline persistence, and regression detection
//! across representative custom chipset workloads per Obsidian/Amiga/Design/Performance Profiling and Optimization Strategy.md.

use config::A500Config;
use machine_loop::A500Machine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::vamiga::catalog::VamigaCatalog;
use crate::vamiga::injector::inject_vamiga_test;

/// Golden baseline filename for chipset benchmarks
pub const CHIPSET_BASELINE_FILENAME: &str = "chipset_benchmark_baseline.json";

/// Maximum allowed performance drop percentage before flagging regression
pub const REGRESSION_THRESHOLD_PERCENT: f64 = -5.0;

/// Default frames to execute per benchmark workload
pub const DEFAULT_BENCHMARK_FRAMES: u32 = 50;

/// Single benchmark workload execution record
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChipsetBenchmarkResult {
    /// Workload identifier / test name
    pub name: String,
    /// Subsystem category exercised
    pub category: String,
    /// Number of vertical video frames executed
    pub frames: u32,
    /// Color Clock cycles stepped
    pub cck_count: u64,
    /// Total wall-clock elapsed time in milliseconds
    pub elapsed_ms: f64,
    /// Effective throughput in frames per second
    pub fps: f64,
    /// Effective Color Clock frequency in MHz
    pub cck_mhz: f64,
}

/// Persistent baseline dataset storing golden performance references
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChipsetBenchmarkBaseline {
    /// Schema format version
    pub version: u32,
    /// Timestamp when baseline was recorded
    pub recorded_at: String,
    /// Benchmark results
    pub results: Vec<ChipsetBenchmarkResult>,
}

impl Default for ChipsetBenchmarkBaseline {
    fn default() -> Self {
        Self {
            version: 1,
            recorded_at: "initial_baseline".to_string(),
            results: Vec::new(),
        }
    }
}

/// Resolves the golden chipset baseline path in `tests/benchmarks/`
pub fn resolve_chipset_baseline_path(repo_root: &Path) -> PathBuf {
    repo_root
        .join("tests")
        .join("benchmarks")
        .join(CHIPSET_BASELINE_FILENAME)
}

/// Executes a single benchmark test directly from an ADF file, measuring wall-clock performance
pub fn benchmark_test_workload(
    test_name: &str,
    category: &str,
    adf_bytes: &[u8],
    frames: u32,
) -> Result<ChipsetBenchmarkResult, String> {
    let mut machine = A500Machine::new(A500Config::default());
    inject_vamiga_test(&mut machine, adf_bytes)?;

    let cck_start = machine.cck;
    let start = Instant::now();

    for _ in 0..frames {
        machine.step_frame();
    }

    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    let cck_count = machine.cck.wrapping_sub(cck_start);

    let fps = if elapsed_ms > 0.0 {
        (frames as f64) / (elapsed.as_secs_f64())
    } else {
        0.0
    };

    let cck_mhz = if elapsed_ms > 0.0 {
        (cck_count as f64) / (elapsed.as_secs_f64() * 1_000_000.0)
    } else {
        0.0
    };

    Ok(ChipsetBenchmarkResult {
        name: test_name.to_string(),
        category: category.to_string(),
        frames,
        cck_count,
        elapsed_ms,
        fps,
        cck_mhz,
    })
}

/// Runs the standard representative suite of chipset benchmarks
pub fn run_chipset_benchmarks(
    repo_root: &Path,
    frames: u32,
) -> Result<Vec<ChipsetBenchmarkResult>, String> {
    let catalog = VamigaCatalog::discover(repo_root);

    // List of representative workloads exercising major custom chip engines
    let targets = [("coptim1", "Copper Coprocessor")];

    let mut results = Vec::new();

    for (target_name, category) in targets {
        let desc = catalog.find_test(target_name).ok_or_else(|| {
            format!(
                "Benchmark workload '{}' not found in test catalog",
                target_name
            )
        })?;

        let adf_bytes = fs::read(&desc.adf_path)
            .map_err(|e| format!("Failed to read benchmark ADF {:?}: {}", desc.adf_path, e))?;

        let res = benchmark_test_workload(target_name, category, &adf_bytes, frames)?;
        results.push(res);
    }

    Ok(results)
}

/// Records active benchmark throughput to golden baseline file
pub fn record_chipset_baseline(
    repo_root: &Path,
    frames: u32,
) -> Result<ChipsetBenchmarkBaseline, String> {
    let results = run_chipset_benchmarks(repo_root, frames)?;
    let baseline = ChipsetBenchmarkBaseline {
        version: 1,
        recorded_at: "2026-09-14".to_string(),
        results,
    };

    let baseline_path = resolve_chipset_baseline_path(repo_root);
    if let Some(parent) = baseline_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let json = serde_json::to_string_pretty(&baseline).map_err(|e| e.to_string())?;
    fs::write(&baseline_path, json).map_err(|e| e.to_string())?;

    println!(
        "[OK] Recorded {} chipset benchmark baselines to: {}",
        baseline.results.len(),
        baseline_path.display()
    );

    Ok(baseline)
}

/// Compares active execution throughput against golden baseline, returning false if regressed
pub fn compare_chipset_baseline(repo_root: &Path, frames: u32) -> Result<bool, String> {
    let baseline_path = resolve_chipset_baseline_path(repo_root);
    if !baseline_path.exists() {
        return Err(format!(
            "Chipset baseline file not found: {}. Run with --record first to create baseline.",
            baseline_path.display()
        ));
    }

    let json = fs::read_to_string(&baseline_path).map_err(|e| e.to_string())?;
    let baseline: ChipsetBenchmarkBaseline =
        serde_json::from_str(&json).map_err(|e| format!("Invalid baseline format: {}", e))?;

    let active_results = run_chipset_benchmarks(repo_root, frames)?;

    println!("\n=========================================================================================");
    println!("🏁 CHIPSET PERFORMANCE REGRESSION AUDIT (vs Golden Baseline)");
    println!(
        "========================================================================================="
    );
    println!(
        "{:<28} {:>14} {:>14} {:>12}   {:<8}",
        "Workload Target", "Baseline FPS", "Active FPS", "Delta (%)", "Verdict"
    );
    println!("{}", "-".repeat(89));

    let mut all_passed = true;

    for active in &active_results {
        let golden = baseline.results.iter().find(|b| b.name == active.name);

        match golden {
            Some(g) => {
                let delta_pct = if g.fps > 0.0 {
                    ((active.fps - g.fps) / g.fps) * 100.0
                } else {
                    0.0
                };

                let passed = delta_pct >= REGRESSION_THRESHOLD_PERCENT;
                if !passed {
                    all_passed = false;
                }

                let verdict_str = if passed { "✅ PASS" } else { "❌ REGRESSION" };

                let delta_sign = if delta_pct >= 0.0 { "+" } else { "" };

                println!(
                    "{:<28} {:>12.1} FPS {:>12.1} FPS {:>11}{:.1}%   {:<8}",
                    format!("{} ({})", active.name, active.category),
                    g.fps,
                    active.fps,
                    delta_sign,
                    delta_pct,
                    verdict_str
                );
            }
            None => {
                println!(
                    "{:<28} {:>14} {:>12.1} FPS {:>12}   {:<8}",
                    format!("{} ({})", active.name, active.category),
                    "N/A",
                    active.fps,
                    "NEW",
                    "ℹ️ INFO"
                );
            }
        }
    }

    println!(
        "========================================================================================="
    );
    if all_passed {
        println!("🚀 All chipset benchmarks meet or exceed golden throughput baselines!\n");
    } else {
        println!(
            "⚠️  PERFORMANCE REGRESSION DETECTED: throughput dropped below -5.0% threshold.\n"
        );
    }

    Ok(all_passed)
}
