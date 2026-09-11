//! Test result recording, persistence, and regression tracking

use crate::diagnostic::TestFailure;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// Brief summary of a single test failure for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailureSummary {
    pub test_name: String,
    pub test_index: usize,
    pub expected_clocks: u32,
    pub expected_cck: u32,
    pub short_error: String,
}

impl From<&TestFailure> for TestFailureSummary {
    fn from(f: &TestFailure) -> Self {
        let short_error = if let Some(diff) = f.diffs.first() {
            match diff {
                crate::diagnostic::StateDiff::StatusRegister {
                    diverging_flags, ..
                } => {
                    format!("SR mismatch: {}", diverging_flags.join(", "))
                }
                crate::diagnostic::StateDiff::DataRegister {
                    reg,
                    actual,
                    expected,
                } => {
                    format!(
                        "D{} mismatch: got 0x{:08X}, expected 0x{:08X}",
                        reg, actual, expected
                    )
                }
                crate::diagnostic::StateDiff::AddressRegister {
                    reg,
                    actual,
                    expected,
                } => {
                    format!(
                        "A{} mismatch: got 0x{:08X}, expected 0x{:08X}",
                        reg, actual, expected
                    )
                }
                crate::diagnostic::StateDiff::ProgramCounter { actual, expected } => {
                    format!(
                        "PC mismatch: got 0x{:08X}, expected 0x{:08X}",
                        actual, expected
                    )
                }
                crate::diagnostic::StateDiff::RamByte {
                    address,
                    actual,
                    expected,
                } => {
                    format!(
                        "RAM at 0x{:06X}: got 0x{:02X}, expected 0x{:02X}",
                        address, actual, expected
                    )
                }
                crate::diagnostic::StateDiff::UserStackPointer { actual, expected } => {
                    format!(
                        "USP mismatch: got 0x{:08X}, expected 0x{:08X}",
                        actual, expected
                    )
                }
                crate::diagnostic::StateDiff::SupervisorStackPointer { actual, expected } => {
                    format!(
                        "SSP mismatch: got 0x{:08X}, expected 0x{:08X}",
                        actual, expected
                    )
                }
                crate::diagnostic::StateDiff::CycleLength { actual, expected } => {
                    format!(
                        "Cycles: got {} clocks, expected {} clocks",
                        actual, expected
                    )
                }
                crate::diagnostic::StateDiff::TransactionCountMismatch { actual, expected } => {
                    format!("Transactions: recorded {}, expected {}", actual, expected)
                }
                crate::diagnostic::StateDiff::TransactionMismatch { details } => {
                    format!("Transaction mismatch: {}", details)
                }
            }
        } else {
            "Unknown mismatch".to_string()
        };

        Self {
            test_name: f.test_name.clone(),
            test_index: f.test_index,
            expected_clocks: f.expected_clocks,
            expected_cck: f.expected_cck,
            short_error,
        }
    }
}

/// Comprehensive outcome of running a single opcode test suite file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteResult {
    pub suite_name: String,
    pub file_path: String,
    pub total_executed: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub passed_test_names: Vec<String>,
    pub failures: Vec<TestFailureSummary>,
}

/// Concise statistics per suite stored in the global summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteStats {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate_percent: f64,
    pub active_failures: Vec<String>,
}

/// Global repository-wide test run summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalTestSummary {
    pub timestamp_utc: String,
    pub total_suites: usize,
    pub total_executed: usize,
    pub total_passed: usize,
    pub total_failed: usize,
    pub overall_pass_rate_percent: f64,
    pub suites: BTreeMap<String, SuiteStats>,
}

/// Resolves the SingleStep test results root directory path (`tests/singlestep`)
pub fn resolve_results_dir() -> PathBuf {
    // Check if running from sub-crate or workspace root
    let candidates = [
        PathBuf::from("tests/singlestep"),
        PathBuf::from("../../tests/singlestep"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = Path::new(&manifest_dir).join("../../tests/singlestep");
        return p;
    }

    PathBuf::from("tests/singlestep")
}

/// Sanitizes suite names for filesystem filenames
fn sanitize_name(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '], "_")
}

/// Saves a suite result, detects regressions/improvements against previous run, and updates summary
pub fn record_suite_result(result: &SuiteResult) {
    let base_dir = resolve_results_dir();
    let latest_dir = base_dir.join("latest");
    let previous_dir = base_dir.join("previous");

    let _ = fs::create_dir_all(&latest_dir);
    let _ = fs::create_dir_all(&previous_dir);

    let sanitized = sanitize_name(&result.suite_name);
    let latest_file = latest_dir.join(format!("{}.json", sanitized));
    let previous_file = previous_dir.join(format!("{}.json", sanitized));

    // Check for previous run
    let previous_result: Option<SuiteResult> = if latest_file.exists() {
        let prev = File::open(&latest_file)
            .ok()
            .and_then(|f| serde_json::from_reader(BufReader::new(f)).ok());

        // Rotate current latest to previous
        let _ = fs::copy(&latest_file, &previous_file);
        prev
    } else {
        None
    };

    // Analyze regression vs improvements
    if let Some(prev) = previous_result {
        let prev_passed_set: HashSet<_> = prev.passed_test_names.iter().collect();
        let prev_failed_set: HashSet<_> = prev.failures.iter().map(|f| &f.test_name).collect();

        let current_failed_set: HashSet<_> = result.failures.iter().map(|f| &f.test_name).collect();
        let current_passed_set: HashSet<_> = result.passed_test_names.iter().collect();

        let regressions: Vec<_> = current_failed_set
            .intersection(&prev_passed_set)
            .cloned()
            .collect();

        let improvements: Vec<_> = current_passed_set
            .intersection(&prev_failed_set)
            .cloned()
            .collect();

        if !regressions.is_empty() {
            eprintln!("\n⚠️  [REGRESSION DETECTED] Suite '{}': {} test(s) that previously PASSED now FAILED!", result.suite_name, regressions.len());
            for test in &regressions {
                eprintln!("   🔴 Broken: \"{}\"", test);
            }
            eprintln!();
        }

        if !improvements.is_empty() {
            eprintln!(
                "\n🎉 [PROGRESS / FIX] Suite '{}': {} test(s) that previously FAILED now PASSED!",
                result.suite_name,
                improvements.len()
            );
            for test in &improvements {
                eprintln!("   🟢 Fixed:  \"{}\"", test);
            }
            eprintln!();
        }
    }

    // Write new latest file
    if let Ok(f) = File::create(&latest_file) {
        let writer = BufWriter::new(f);
        let _ = serde_json::to_writer_pretty(writer, result);
    }

    // Update global summary
    update_global_summary(&base_dir);
}

/// Aggregates all suite results in `tests/singlestep/latest` into a unified `summary.json`
pub fn update_global_summary(base_dir: &Path) {
    let latest_dir = base_dir.join("latest");
    if !latest_dir.exists() {
        return;
    }

    let mut suites_map = BTreeMap::new();
    let mut total_executed = 0;
    let mut total_passed = 0;
    let mut total_failed = 0;

    if let Ok(entries) = fs::read_dir(&latest_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(file) = File::open(&path) {
                    if let Ok(suite) =
                        serde_json::from_reader::<_, SuiteResult>(BufReader::new(file))
                    {
                        total_executed += suite.total_executed;
                        total_passed += suite.passed_count;
                        total_failed += suite.failed_count;

                        let pass_rate = if suite.total_executed > 0 {
                            (suite.passed_count as f64 / suite.total_executed as f64) * 100.0
                        } else {
                            0.0
                        };

                        let active_failures =
                            suite.failures.iter().map(|f| f.test_name.clone()).collect();

                        suites_map.insert(
                            suite.suite_name,
                            SuiteStats {
                                total: suite.total_executed,
                                passed: suite.passed_count,
                                failed: suite.failed_count,
                                pass_rate_percent: pass_rate,
                                active_failures,
                            },
                        );
                    }
                }
            }
        }
    }

    let overall_pass_rate = if total_executed > 0 {
        (total_passed as f64 / total_executed as f64) * 100.0
    } else {
        0.0
    };

    let summary = GlobalTestSummary {
        timestamp_utc: chrono_stub(),
        total_suites: suites_map.len(),
        total_executed,
        total_passed,
        total_failed,
        overall_pass_rate_percent: overall_pass_rate,
        suites: suites_map,
    };

    let summary_path = base_dir.join("summary.json");
    if let Ok(f) = File::create(&summary_path) {
        let writer = BufWriter::new(f);
        let _ = serde_json::to_writer_pretty(writer, &summary);
    }
}

/// Fallback timestamp without external chrono dependency
fn chrono_stub() -> String {
    // Basic counter / standard placeholder format
    "latest_run".to_string()
}
