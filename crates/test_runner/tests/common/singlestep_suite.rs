//! Parallel group execution and one final report inside the aggregate test.

use super::reporting::{current_run_id, resolve_results_dir, update_global_summary, RunMetadata};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

type TestCase = (&'static str, fn());

fn write_run(directory: &Path, run: &RunMetadata) -> Result<(), Box<dyn Error>> {
    fs::write(directory.join("run.json"), serde_json::to_vec_pretty(run)?)?;
    Ok(())
}

pub fn run_suite(cases: &[TestCase]) -> Result<(), Box<dyn Error>> {
    let filter = std::env::var("SINGLESTEP_CASE").unwrap_or_default();
    let selected: Vec<_> = cases
        .iter()
        .filter(|(name, _)| name.contains(&filter))
        .collect();
    if selected.is_empty() {
        return Err(format!("No SingleStep groups match '{filter}'").into());
    }
    let directory = resolve_results_dir();
    fs::create_dir_all(&directory)?;
    let mut run = RunMetadata {
        id: current_run_id().to_string(),
        status: "running".to_string(),
        profile: std::env::var("SINGLESTEP_PROFILE").unwrap_or_else(|_| "cargo".to_string()),
        selected_tests: selected.len(),
        completed_tests: 0,
        exit_code: None,
    };
    write_run(&directory, &run)?;

    let workers = std::env::var("RUST_TEST_THREADS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|count| *count > 0)
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
        .min(selected.len());
    let next = AtomicUsize::new(0);
    let execution_started = Instant::now();
    let mut failed = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let selected = &selected;
                let next = &next;
                scope.spawn(move || {
                    let mut failed = Vec::new();
                    while let Some((name, execute)) =
                        selected.get(next.fetch_add(1, Ordering::Relaxed))
                    {
                        if std::panic::catch_unwind(*execute).is_err() {
                            failed.push(*name);
                        }
                    }
                    failed
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| {
                handle
                    .join()
                    .expect("SingleStep worker panicked outside a group")
            })
            .collect::<Vec<_>>()
    });
    let execution_seconds = execution_started.elapsed().as_secs_f64();
    failed.sort_unstable();
    run.completed_tests = selected.len();
    run.status = if failed.is_empty() {
        "passed"
    } else {
        "failed"
    }
    .to_string();
    run.exit_code = Some(if failed.is_empty() { 0 } else { 101 });
    write_run(&directory, &run)?;

    let finalization_started = Instant::now();
    let report = update_global_summary(&directory, Some(run.clone()));
    let finalization_seconds = finalization_started.elapsed().as_secs_f64();
    if report.is_err() {
        run.status = "failed".to_string();
        run.exit_code = Some(101);
        write_run(&directory, &run)?;
    }
    fs::write(
        directory.join("timing.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "run_id": run.id,
            "profile": run.profile,
            "execution_seconds": execution_seconds,
            "finalization_seconds": finalization_seconds,
            "pipeline_seconds": execution_seconds + finalization_seconds,
            "report_exit_code": if report.is_ok() { 0 } else { 1 },
        }))?,
    )?;
    println!(
        "[INFO] SingleStep groups: {} passed; {} failed; {} selected.",
        selected.len() - failed.len(),
        failed.len(),
        selected.len()
    );
    for name in &failed {
        eprintln!("[FAIL] {name}");
    }
    report?;
    if !failed.is_empty() {
        return Err(format!("{} SingleStep group(s) failed", failed.len()).into());
    }
    Ok(())
}
