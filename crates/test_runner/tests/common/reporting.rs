//! Test result recording, persistence, and regression tracking

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
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
    /// Empty in legacy reports whose selection cannot be identified reliably.
    #[serde(default)]
    pub selection: String,
    #[serde(default)]
    pub sample_limit: Option<usize>,
    #[serde(default)]
    pub run_id: Option<String>,
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
    pub run: Option<RunMetadata>,
    pub current_run_suites: usize,
    pub current_run_vectors: usize,
}

/// Group completion recorded by the aggregate test, independently of saved history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetadata {
    pub id: String,
    pub status: String,
    pub profile: String,
    pub selected_tests: usize,
    pub completed_tests: usize,
    // Windows process failures may use an unsigned 32-bit exception status.
    pub exit_code: Option<i64>,
}

/// Resolves the SingleStep test results root directory path (`.agent/tmp/singlestep`)
pub fn resolve_results_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("SINGLESTEP_RESULTS_DIR") {
        return PathBuf::from(path);
    }
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

/// Saves one suite and rotates its history. The aggregate Rust test owns global aggregation.
pub fn record_suite_result(result: &SuiteResult, base_dir: &Path) -> io::Result<()> {
    if !result.overridden_test_names.is_empty() {
        eprintln!(
            "[OVERRIDE] {}: {} executed vectors use reviewed corrections: {}",
            result.suite_name,
            result.overridden_test_names.len(),
            result.overridden_test_names.join(", ")
        );
    }
    let latest_dir = base_dir.join("latest");
    let previous_dir = base_dir.join("previous");

    fs::create_dir_all(&latest_dir)?;
    fs::create_dir_all(&previous_dir)?;

    let sanitized = sanitize_name(&result.suite_name);
    let latest_file = latest_dir.join(format!("{}.json", sanitized));
    let previous_file = previous_dir.join(format!("{}.json", sanitized));

    // Check for previous run
    let previous_result: Option<SuiteResult> = if latest_file.exists() {
        let prev = read_suite(&latest_file)?;

        // Rotate current latest to previous
        fs::copy(&latest_file, &previous_file)?;
        Some(prev)
    } else {
        None
    };

    // Analyze regression vs improvements
    if let Some(prev) = previous_result.filter(|previous| comparable(previous, result)) {
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
    write_json(&latest_file, result)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.flush()
}

fn comparable(previous: &SuiteResult, latest: &SuiteResult) -> bool {
    !previous.selection.is_empty()
        && previous.selection == latest.selection
        && previous.sample_limit == latest.sample_limit
}

/// Aggregates all suite results in `.agent/tmp/singlestep/latest` into a unified `summary.json`
pub fn update_global_summary(base_dir: &Path, run: Option<RunMetadata>) -> io::Result<()> {
    let latest_dir = base_dir.join("latest");
    if !latest_dir.exists() {
        fs::create_dir_all(&latest_dir)?;
    }

    let mut suites_map = BTreeMap::new();
    let mut total_executed = 0;
    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut current_run_suites = 0;
    let mut current_run_vectors = 0;

    for entry in fs::read_dir(&latest_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let suite = read_suite(&path)?;
        if run
            .as_ref()
            .is_some_and(|run| suite.run_id.as_ref() == Some(&run.id))
        {
            current_run_suites += 1;
            current_run_vectors += suite.total_executed;
        }
        total_executed += suite.total_executed;
        total_passed += suite.passed_count;
        total_failed += suite.failed_count;
        let pass_rate = if suite.total_executed > 0 {
            (suite.passed_count as f64 / suite.total_executed as f64) * 100.0
        } else {
            0.0
        };
        let active_failures = suite.failures.iter().map(|f| f.test_name.clone()).collect();
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
        run,
        current_run_suites,
        current_run_vectors,
    };

    let summary_path = base_dir.join("summary.json");
    let diff = diff_markdown(base_dir)?;
    write_json(&summary_path, &summary)?;
    fs::write(base_dir.join("summary.md"), summary_markdown(&summary))?;
    fs::write(base_dir.join("diff.md"), diff)
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
    if let Some(run) = &summary.run {
        let _ = write!(report, "Current execution: **{}** (profile `{}`).\n\n- Selected tests: {}\n- Completed tests: {}\n- Available current suite reports: {}\n- Reported current vectors: {}\n\n", cell(&run.status), cell(&run.profile), run.selected_tests, run.completed_tests, summary.current_run_suites, summary.current_run_vectors);
    } else {
        report.push_str("Saved results only; no current execution verdict is available.\n\n");
    }
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
    let mut incompatible_baselines = BTreeSet::new();
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
            if !comparable(&previous, &latest) {
                incompatible_baselines.insert(latest.suite_name);
                continue;
            }
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
    report.push_str("Latest versus previous saved result for each suite. History rotates independently per suite. Only matching selections and sample limits are compared. Legacy unidentified selections are excluded. Only shared test names changing between passed and failed are compared; new or removed cases are excluded.\n\n");
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
    if !incompatible_baselines.is_empty() {
        report.push_str("\n## Suites With an Incompatible or Unidentified Previous Selection\n\n| Suite |\n| --- |\n");
        for suite in incompatible_baselines {
            let _ = write!(report, "| {} |\n", cell(&suite));
        }
    }
    Ok(report)
}

/// Shares one execution identity across all workers, including direct Cargo runs.
pub fn current_run_id() -> &'static str {
    static ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ID.get_or_init(|| {
        std::env::var("SINGLESTEP_RUN_ID").unwrap_or_else(|_| {
            let elapsed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            format!("{}-{}", std::process::id(), elapsed.as_nanos())
        })
    })
}
