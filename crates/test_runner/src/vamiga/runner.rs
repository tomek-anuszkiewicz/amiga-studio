//! vAmigaTS Test Execution Runner
//!
//! Autonomous test harness driver that boots test ADFs via direct injection,
//! steps the machine for N frames, extracts the 716 x 285 RGB24 viewport,
//! and compares rendered video against reference `.raw` frame captures.

use std::fs;
use std::path::Path;
use std::time::Instant;

use config::A500Config;
use machine_loop::A500Machine;

use super::catalog::{VamigaCatalog, VamigaCategory, VamigaTestDescriptor, VamigaTestStatus};
use super::injector::inject_vamiga_test;
use super::matcher::{compare_raw_frames, VamigaTestResult, VAMIGA_RAW_BYTE_SIZE};
use super::script::VamigaScript;

/// Execution parameters for running a vAmigaTS test
#[derive(Debug, Clone)]
pub struct VamigaRunConfig {
    /// Number of vertical video frames to run in direct-injection mode (default: 8)
    pub frames_to_run: u32,
    /// Hardware machine configuration
    pub machine_config: A500Config,
    /// Whether to print verbose per-test status lines during execution
    pub verbose: bool,
}

impl Default for VamigaRunConfig {
    fn default() -> Self {
        Self {
            frames_to_run: 8,
            machine_config: A500Config::default(),
            verbose: false,
        }
    }
}

/// Comprehensive summary of a test suite execution run
#[derive(Debug, Clone, Default)]
pub struct VamigaSuiteSummary {
    /// Category executed
    pub category: String,
    /// Total tests inspected in category
    pub total_tests: usize,
    /// Total tests eligible for Phase 1 Baseline execution
    pub runnable_tests: usize,
    /// Total tests deferred to subsequent roadmap phases
    pub deferred_tests: usize,
    /// Total tests executed
    pub executed_tests: usize,
    /// Total tests that passed with 100% pixel match
    pub passed_tests: usize,
    /// Total tests that had pixel mismatches or errors
    pub failed_tests: usize,
    /// Elapsed execution time in milliseconds
    pub elapsed_ms: u128,
    /// Detailed failures: (test_name, result)
    pub failures: Vec<(String, VamigaTestResult)>,
    /// Errors preventing test from running: (test_name, error_message)
    pub errors: Vec<(String, String)>,
}

impl VamigaSuiteSummary {
    /// Calculates pass rate percentage among executed tests
    pub fn pass_rate(&self) -> f64 {
        if self.executed_tests == 0 {
            0.0
        } else {
            (self.passed_tests as f64 / self.executed_tests as f64) * 100.0
        }
    }

    /// Prints a formatted summary table to stdout
    pub fn print_summary(&self) {
        println!("\n=========================================================================================");
        println!(
            "🏁 vAmigaTS VERIFICATION SUMMARY: [{}]",
            self.category.to_uppercase()
        );
        println!("=========================================================================================");
        println!("Total Discovered:   {}", self.total_tests);
        println!("Active Baseline:    {}", self.runnable_tests);
        println!("Deferred to Future: {}", self.deferred_tests);
        println!("Executed:           {}", self.executed_tests);
        println!("Passed (100% RGB):  {}", self.passed_tests);
        println!("Failed / Regressed: {}", self.failed_tests);
        if !self.errors.is_empty() {
            println!("Errors / Invalid:   {}", self.errors.len());
        }
        println!("Pass Rate:          {:.2}%", self.pass_rate());
        println!(
            "Elapsed Time:       {:.2}s",
            self.elapsed_ms as f64 / 1000.0
        );
        println!("-----------------------------------------------------------------------------------------");

        if !self.failures.is_empty() {
            println!("\n❌ FAILED TEST CASES (First 10):");
            for (name, res) in self.failures.iter().take(10) {
                let pct = (res.mismatched_pixels as f64 / res.total_pixels as f64) * 100.0;
                print!(
                    "  • {:<28} mismatched: {:>6}/{} ({:>5.1}%)",
                    name, res.mismatched_pixels, res.total_pixels, pct
                );
                if let Some(diff) = &res.first_mismatch {
                    print!(
                        " | first mismatch at ({}, {}): actual={:?}, expected={:?}",
                        diff.x, diff.y, diff.actual, diff.expected
                    );
                }
                println!();
            }
            if self.failures.len() > 10 {
                println!(
                    "    ... and {} more failing tests",
                    self.failures.len() - 10
                );
            }
        }

        if !self.errors.is_empty() {
            println!("\n⚠️ RUNTIME TEST ERRORS (First 5):");
            for (name, err) in self.errors.iter().take(5) {
                println!("  • {}: {}", name, err);
            }
        }
        println!("=========================================================================================\n");
    }
}

/// Executes a test directly from memory buffers for ADF and expected RAW reference.
pub fn run_vamiga_test_buffers(
    adf_bytes: &[u8],
    expected_raw_bytes: &[u8],
    frames_to_run: u32,
    machine_config: &A500Config,
) -> Result<VamigaTestResult, String> {
    let mut machine = A500Machine::new(machine_config.clone());
    inject_vamiga_test(&mut machine, adf_bytes)?;

    // Run for requested number of full frames
    for _ in 0..frames_to_run {
        machine.step_frame();
    }

    // Extract rendered 716 x 285 RGB24 viewport
    let mut actual_raw = [0u8; VAMIGA_RAW_BYTE_SIZE];
    machine
        .denise
        .frame_builder
        .extract_vamiga_raw_viewport(&mut actual_raw);

    // Compare with expected reference
    compare_raw_frames(&actual_raw, expected_raw_bytes)
}

/// Executes a single test from its `VamigaTestDescriptor`
pub fn run_vamiga_test(
    desc: &VamigaTestDescriptor,
    config: &VamigaRunConfig,
) -> Result<VamigaTestResult, String> {
    if desc.status != VamigaTestStatus::Runnable {
        return Err(format!(
            "Test {} is deferred and cannot run in Phase 1 Baseline",
            desc.name
        ));
    }

    let adf_bytes = fs::read(&desc.adf_path)
        .map_err(|e| format!("Failed to read ADF {:?}: {}", desc.adf_path, e))?;

    let raw_path = desc
        .raw_path
        .as_ref()
        .ok_or_else(|| format!("Test {} has no reference .raw file", desc.name))?;
    let raw_bytes =
        fs::read(raw_path).map_err(|e| format!("Failed to read RAW {:?}: {}", raw_path, e))?;

    // Determine frame count from script if available
    let frames = if let Some(retrosh_path) = &desc.retrosh_path {
        if let Ok(script) = VamigaScript::load_from_file(retrosh_path) {
            script.effective_frames(config.frames_to_run)
        } else {
            config.frames_to_run
        }
    } else {
        config.frames_to_run
    };

    run_vamiga_test_buffers(&adf_bytes, &raw_bytes, frames, &config.machine_config)
}

/// Executes a test given a test directory path and test name.
pub fn run_vamiga_test_from_dir(
    test_dir: &Path,
    test_name: &str,
    config: &VamigaRunConfig,
) -> Result<VamigaTestResult, String> {
    let adf_path = test_dir.join(format!("{}.adf", test_name));
    if !adf_path.exists() {
        return Err(format!("ADF file not found: {:?}", adf_path));
    }
    let adf_bytes = fs::read(&adf_path)
        .map_err(|e| format!("Failed to read ADF file {:?}: {}", adf_path, e))?;

    // Try finding raw file: prefer <test_name>_ocs.raw, then <test_name>.raw, then <test_name>_ecs.raw
    let raw_path = {
        let ocs = test_dir.join(format!("{}_ocs.raw", test_name));
        let direct = test_dir.join(format!("{}.raw", test_name));
        let ecs = test_dir.join(format!("{}_ecs.raw", test_name));
        if ocs.exists() {
            ocs
        } else if direct.exists() {
            direct
        } else if ecs.exists() {
            ecs
        } else {
            return Err(format!("No reference .raw file found in {:?}", test_dir));
        }
    };

    let raw_bytes = fs::read(&raw_path)
        .map_err(|e| format!("Failed to read reference raw file {:?}: {}", raw_path, e))?;

    run_vamiga_test_buffers(
        &adf_bytes,
        &raw_bytes,
        config.frames_to_run,
        &config.machine_config,
    )
}

/// Executes a batch suite of tests from a catalog
pub fn run_vamiga_suite(
    catalog: &VamigaCatalog,
    category: VamigaCategory,
    config: &VamigaRunConfig,
    max_tests: Option<usize>,
) -> VamigaSuiteSummary {
    let start_time = Instant::now();
    let all_in_cat = catalog.filter_category(category);

    let total_tests = all_in_cat.len();
    let runnable: Vec<&VamigaTestDescriptor> = all_in_cat
        .iter()
        .copied()
        .filter(|t| t.status == VamigaTestStatus::Runnable)
        .collect();

    let runnable_tests = runnable.len();
    let deferred_tests = total_tests.saturating_sub(runnable_tests);

    let target_tests: Vec<&VamigaTestDescriptor> = if let Some(limit) = max_tests {
        runnable.into_iter().take(limit).collect()
    } else {
        runnable
    };

    let mut passed_tests = 0;
    let mut failed_tests = 0;
    let mut failures = Vec::new();
    let mut errors = Vec::new();

    for (idx, desc) in target_tests.iter().enumerate() {
        if config.verbose {
            print!(
                "[{:>3}/{}] Running {} ({:?})... ",
                idx + 1,
                target_tests.len(),
                desc.name,
                desc.category
            );
        }

        match run_vamiga_test(desc, config) {
            Ok(result) => {
                if result.passed {
                    passed_tests += 1;
                    if config.verbose {
                        println!("PASS");
                    }
                } else {
                    failed_tests += 1;
                    if config.verbose {
                        println!("FAIL ({} diffs)", result.mismatched_pixels);
                    }
                    failures.push((desc.name.clone(), result));
                }
            }
            Err(err) => {
                failed_tests += 1;
                if config.verbose {
                    println!("ERROR ({})", err);
                }
                errors.push((desc.name.clone(), err));
            }
        }
    }

    let elapsed_ms = start_time.elapsed().as_millis();

    VamigaSuiteSummary {
        category: category.as_str().to_string(),
        total_tests,
        runnable_tests,
        deferred_tests,
        executed_tests: target_tests.len(),
        passed_tests,
        failed_tests,
        elapsed_ms,
        failures,
        errors,
    }
}
