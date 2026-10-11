//! Test result recording, persistence, and regression tracking

use super::diagnostic::TestFailure;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
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
                super::diagnostic::StateDiff::StatusRegister {
                    diverging_flags, ..
                } => {
                    format!("SR mismatch: {}", diverging_flags.join(", "))
                }
                super::diagnostic::StateDiff::DataRegister {
                    reg,
                    actual,
                    expected,
                } => {
                    format!(
                        "D{} mismatch: got 0x{:08X}, expected 0x{:08X}",
                        reg, actual, expected
                    )
                }
                super::diagnostic::StateDiff::AddressRegister {
                    reg,
                    actual,
                    expected,
                } => {
                    format!(
                        "A{} mismatch: got 0x{:08X}, expected 0x{:08X}",
                        reg, actual, expected
                    )
                }
                super::diagnostic::StateDiff::ProgramCounter { actual, expected } => {
                    format!(
                        "PC mismatch: got 0x{:08X}, expected 0x{:08X}",
                        actual, expected
                    )
                }
                super::diagnostic::StateDiff::RamByte {
                    address,
                    actual,
                    expected,
                } => {
                    format!(
                        "RAM at 0x{:06X}: got 0x{:02X}, expected 0x{:02X}",
                        address, actual, expected
                    )
                }
                super::diagnostic::StateDiff::UserStackPointer { actual, expected } => {
                    format!(
                        "USP mismatch: got 0x{:08X}, expected 0x{:08X}",
                        actual, expected
                    )
                }
                super::diagnostic::StateDiff::SupervisorStackPointer { actual, expected } => {
                    format!(
                        "SSP mismatch: got 0x{:08X}, expected 0x{:08X}",
                        actual, expected
                    )
                }
                super::diagnostic::StateDiff::CycleLength { actual, expected } => {
                    format!(
                        "Cycles: got {} clocks, expected {} clocks",
                        actual, expected
                    )
                }
                super::diagnostic::StateDiff::TransactionCountMismatch { actual, expected } => {
                    format!("Transactions: recorded {}, expected {}", actual, expected)
                }
                super::diagnostic::StateDiff::TransactionMismatch { details } => {
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
    #[serde(default)]
    pub overridden_test_names: Vec<String>,
}

/// Concise statistics per suite stored in the global summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteStats {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate_percent: f64,
    pub active_failures: Vec<String>,
    #[serde(default)]
    pub overrides: usize,
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

/// Resolves the SingleStep test results root directory path (`.agent/tmp/singlestep`)
pub fn resolve_results_dir() -> PathBuf {
    // Check if running from sub-crate or workspace root
    let candidates = [
        PathBuf::from(".agent/tmp/singlestep"),
        PathBuf::from("../../.agent/tmp/singlestep"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = Path::new(&manifest_dir).join("../../.agent/tmp/singlestep");
        return p;
    }

    PathBuf::from(".agent/tmp/singlestep")
}

/// Sanitizes suite names for filesystem filenames
fn sanitize_name(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '], "_")
}

/// Saves a suite result, detects regressions/improvements against previous run, and updates summary
pub fn record_suite_result(result: &SuiteResult) {
    if !result.overridden_test_names.is_empty() {
        eprintln!(
            "[OVERRIDE] {}: {} executed vectors use reviewed corrections: {}",
            result.suite_name,
            result.overridden_test_names.len(),
            result.overridden_test_names.join(", ")
        );
    }
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
        let (regressions, improvements) = suite_changes(&prev, result);

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

/// Aggregates all suite results in `.agent/tmp/singlestep/latest` into a unified `summary.json`
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
                                overrides: suite.overridden_test_names.len(),
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

    if let Err(error) = fs::write(base_dir.join("summary.md"), summary_markdown(&summary)) {
        eprintln!("Failed to write summary.md: {error}");
    }
    match diff_markdown(base_dir) {
        Ok(report) => {
            if let Err(error) = fs::write(base_dir.join("diff.md"), report) {
                eprintln!("Failed to write diff.md: {error}");
            }
        }
        Err(error) => eprintln!("Failed to generate diff.md: {error}"),
    }
}

/// Fallback timestamp without external chrono dependency
fn chrono_stub() -> String {
    // Basic counter / standard placeholder format
    "latest_run".to_string()
}

fn suite_changes(previous: &SuiteResult, latest: &SuiteResult) -> (Vec<String>, Vec<String>) {
    let previous_passed: BTreeSet<_> = previous.passed_test_names.iter().collect();
    let previous_failed: BTreeSet<_> = previous.failures.iter().map(|f| &f.test_name).collect();
    let latest_passed: BTreeSet<_> = latest.passed_test_names.iter().collect();
    let latest_failed: BTreeSet<_> = latest.failures.iter().map(|f| &f.test_name).collect();
    (
        latest_failed
            .intersection(&previous_passed)
            .map(|s| (*s).clone())
            .collect(),
        latest_passed
            .intersection(&previous_failed)
            .map(|s| (*s).clone())
            .collect(),
    )
}

fn cell(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\\', "&#92;")
        .replace('|', "&#124;")
        .replace('`', "&#96;")
        .replace('*', "&#42;")
        .replace('_', "&#95;")
        .replace('[', "&#91;")
        .replace(']', "&#93;")
        .replace(['\r', '\n'], " ")
}

/// Formats all persisted suite statistics and failure names as Markdown.
fn summary_markdown(summary: &GlobalTestSummary) -> String {
    let mut report = String::from("# M68000 SingleStep Test Summary\n\n");
    report.push_str(
        "Latest saved result for each suite. Suites may come from different executions.\n\n",
    );
    let _ = write!(report, "- Suites: {}\n", summary.total_suites);
    let _ = write!(report, "- Executed: {}\n", summary.total_executed);
    let _ = write!(report, "- Passed: {}\n", summary.total_passed);
    let _ = write!(report, "- Failed: {}\n", summary.total_failed);
    let override_count: usize = summary.suites.values().map(|stats| stats.overrides).sum();
    let _ = write!(report, "- Executed with overrides: {override_count}\n");
    let _ = write!(
        report,
        "- Pass rate: {:.2}%\n\n",
        summary.overall_pass_rate_percent
    );
    report.push_str(
        "| Suite | Executed | Passed | Failed | Overrides | Pass rate |\n| --- | ---: | ---: | ---: | ---: | ---: |\n",
    );
    for (name, stats) in &summary.suites {
        let _ = write!(
            report,
            "| {} | {} | {} | {} | {} | {:.2}% |\n",
            cell(name),
            stats.total,
            stats.passed,
            stats.failed,
            stats.overrides,
            stats.pass_rate_percent
        );
    }
    if summary
        .suites
        .values()
        .any(|stats| !stats.active_failures.is_empty())
    {
        report.push_str("\n## Failing Cases\n\n| Suite | Test |\n| --- | --- |\n");
        for (name, stats) in &summary.suites {
            for failure in &stats.active_failures {
                let _ = write!(report, "| {} | {} |\n", cell(name), cell(failure));
            }
        }
    }
    report
}

fn read_suite(path: &Path) -> io::Result<SuiteResult> {
    serde_json::from_reader(BufReader::new(File::open(path)?)).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {error}", path.display()),
        )
    })
}

/// Compares shared case names against the previous saved result for each suite.
/// Missing baselines are reported separately; unreadable results return an error.
fn diff_markdown(base_dir: &Path) -> io::Result<String> {
    let latest_dir = base_dir.join("latest");
    let previous_dir = base_dir.join("previous");
    let mut changes = BTreeMap::new();
    let mut missing_baselines = BTreeSet::new();
    let mut compared = 0;
    let mut regressions = 0;
    let mut improvements = 0;

    if latest_dir.exists() {
        for entry in fs::read_dir(&latest_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let latest = read_suite(&path)?;
            let previous_path = previous_dir.join(entry.file_name());
            if !previous_path.exists() {
                missing_baselines.insert(latest.suite_name);
                continue;
            }
            let previous = read_suite(&previous_path)?;
            let (broken, fixed) = suite_changes(&previous, &latest);
            compared += 1;
            regressions += broken.len();
            improvements += fixed.len();
            if !broken.is_empty() || !fixed.is_empty() {
                changes.insert(latest.suite_name, (broken, fixed));
            }
        }
    }

    let mut report = String::from("# M68000 SingleStep Test Diff\n\n");
    report.push_str("Latest versus previous saved result for each suite. History rotates independently per suite. Only shared test names changing between passed and failed are compared; new or removed cases are excluded.\n\n");
    let _ = write!(report, "- Compared suites: {compared}\n");
    let _ = write!(
        report,
        "- Suites without a previous result: {}\n",
        missing_baselines.len()
    );
    let _ = write!(report, "- Regressions (passed to failed): {regressions}\n");
    let _ = write!(
        report,
        "- Improvements (failed to passed): {improvements}\n\n"
    );
    if compared == 0 {
        report.push_str(
            "No comparable results yet. Save results for the same suite at least twice.\n",
        );
    } else if changes.is_empty() {
        report.push_str("No pass/fail status changes detected among comparable cases.\n");
    } else {
        report.push_str("## Status Changes\n\n| Suite | Change | Test |\n| --- | --- | --- |\n");
        for (suite, (broken, fixed)) in changes {
            for test in broken {
                let _ = write!(
                    report,
                    "| {} | REGRESSION | {} |\n",
                    cell(&suite),
                    cell(&test)
                );
            }
            for test in fixed {
                let _ = write!(
                    report,
                    "| {} | IMPROVEMENT | {} |\n",
                    cell(&suite),
                    cell(&test)
                );
            }
        }
    }
    if !missing_baselines.is_empty() {
        report.push_str("\n## Suites Without a Previous Result\n\n| Suite |\n| --- |\n");
        for suite in missing_baselines {
            let _ = write!(report, "| {} |\n", cell(&suite));
        }
    }
    Ok(report)
}
