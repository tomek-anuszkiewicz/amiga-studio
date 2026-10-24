#![allow(unsafe_code)]

//! Measured CPU stepping for fixtures that passed validation before timing.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cpu::Cpu;
use physical_memory::PhysicalMemory;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::candidate_probe::{
    final_state_sha256, instruction_words, validate_candidate, Candidate,
};
#[cfg(windows)]
use super::platform::windows;
use super::platform::HostEnvironment;

const TARGET_SERIES_NS: u128 = 400_000_000;
const CONTROL_TARGET_SERIES_NS: u128 = TARGET_SERIES_NS * 3;
const MIN_CALIBRATION_NS: u128 = 5_000_000;
pub const MAX_INSTRUCTIONS_PER_BLOCK: usize = 5000;

#[derive(Debug, Deserialize)]
struct ExpectedProbe {
    case_id: String,
    fixture_sha256: String,
    final_state_sha256: String,
    block_status: String,
    replay_status: String,
    block_targets: usize,
    block_clocks: u64,
}

#[derive(Debug)]
struct PreparedCase {
    initial_cpu: Cpu,
    initial_bus: PhysicalMemory,
    final_cpu: Cpu,
    final_state_sha256: String,
    fixture_sha256: String,
    targets_per_block: usize,
    clocks_per_block: u64,
    stopped: bool,
    counted_jumps: bool,
}

#[derive(Clone, Debug, Serialize)]
struct Series {
    attempt: u8,
    ordinal: u8,
    blocks: u64,
    target_count: u64,
    elapsed_ns: u128,
    ns_per_target: f64,
}

#[derive(Clone, Debug, Serialize)]
struct CalibrationStep {
    blocks: u64,
    elapsed_ns: u128,
}

#[derive(Clone, Debug, Serialize)]
struct MeasurementRow {
    case_id: String,
    role: &'static str,
    target_series_ns: u128,
    fixture_sha256: String,
    target_count_per_block: usize,
    clocks_per_block: u64,
    pilot_blocks: u64,
    pilot_elapsed_ns: u128,
    calibration: Vec<CalibrationStep>,
    blocks_per_series: u64,
    target_duration_capped: bool,
    series: Vec<Series>,
    selected_median_ns_per_target: Option<f64>,
    min_ns_per_target: Option<f64>,
    max_ns_per_target: Option<f64>,
    dispersion_pct: Option<f64>,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_repeat: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    confirmed_with: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RetryPlanCase {
    case_id: String,
    reason: String,
    full_ns_per_target: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RetryPlan {
    version: u32,
    source_run: String,
    full_measurements_sha256: String,
    candidate_manifest_sha256: String,
    validation_sha256: String,
    agreement_tolerance_pct: f64,
    max_repeat_attempts: usize,
    selected: Vec<RetryPlanCase>,
}

struct RetrySource {
    indices: Vec<usize>,
    source_name: String,
    full_medians: HashMap<String, f64>,
    plan_sha256: String,
    agreement_tolerance_pct: f64,
    max_repeat_attempts: usize,
}

fn prepare_case(candidate: &Candidate, expected: &ExpectedProbe) -> Result<PreparedCase, String> {
    let opcode = u16::from_str_radix(&candidate.representative_opcode, 16)
        .map_err(|error| error.to_string())?;
    let words = instruction_words(opcode, candidate);
    let mut cpu = Cpu::new();
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();
    let mut fingerprint = String::new();
    let mut captured = None;
    let audit = validate_candidate(
        &mut bus,
        &mut cpu,
        candidate,
        opcode,
        &words,
        &mut fingerprint,
        Some(&mut captured),
    );
    let (initial_cpu, initial_bus) = captured
        .ok_or_else(|| format!("{} did not produce a prepared fixture", candidate.case_id))?;
    let final_digest = final_state_sha256(&cpu, &bus);
    if expected.case_id != candidate.case_id
        || expected.replay_status != "identical"
        || !matches!(
            expected.block_status.as_str(),
            "sequential_700_reached_exit" | "single_target_stop" | "counted_700_jumps"
        )
        || audit.0 != expected.block_status
        || audit.1 != expected.block_targets
        || audit.2 != expected.block_clocks
        || fingerprint != expected.fixture_sha256
        || final_digest != expected.final_state_sha256
    {
        return Err(format!(
            "{} no longer matches the validation record",
            candidate.case_id
        ));
    }
    Ok(PreparedCase {
        initial_cpu,
        initial_bus,
        final_cpu: cpu,
        final_state_sha256: final_digest,
        fixture_sha256: fingerprint,
        targets_per_block: audit.1,
        clocks_per_block: audit.2,
        stopped: audit.0 == "single_target_stop",
        counted_jumps: audit.0 == "counted_700_jumps",
    })
}

fn run_block(
    prepared: &PreparedCase,
    cpu: &mut Cpu,
    bus: &mut PhysicalMemory,
) -> Result<u128, String> {
    cpu.clone_from(&prepared.initial_cpu);
    if !prepared.stopped {
        bus.chip_ram[..0x10000].copy_from_slice(&prepared.initial_bus.chip_ram[..0x10000]);
    }
    let step_limit = if prepared.counted_jumps {
        prepared.targets_per_block
    } else {
        MAX_INSTRUCTIONS_PER_BLOCK
    };
    let started = Instant::now();
    let mut steps = 0;
    while !cpu.state.is_stopped()
        && !cpu.state.is_halted()
        && cpu.state.instruction_pc() != 0x4FFE
        && steps < step_limit
    {
        if cpu.step_instruction(bus) == 0 {
            break;
        }
        steps += 1;
    }
    let elapsed = started.elapsed().as_nanos();
    if cpu != &prepared.final_cpu {
        return Err(format!(
            "timed replay diverged after {steps} guest instructions"
        ));
    }
    Ok(elapsed)
}

fn run_series(
    prepared: &PreparedCase,
    blocks: u64,
    attempt: u8,
    ordinal: u8,
    cpu: &mut Cpu,
    bus: &mut PhysicalMemory,
) -> Result<Series, String> {
    let mut elapsed_ns = 0u128;
    for _ in 0..blocks {
        elapsed_ns += run_block(prepared, cpu, bus)?;
    }
    if final_state_sha256(cpu, bus) != prepared.final_state_sha256 {
        return Err("timed series ended with a different memory or CPU state".to_string());
    }
    let target_count = blocks.saturating_mul(prepared.targets_per_block as u64);
    Ok(Series {
        attempt,
        ordinal,
        blocks,
        target_count,
        elapsed_ns,
        ns_per_target: elapsed_ns as f64 / target_count as f64,
    })
}

fn spread(series: &[Series]) -> (f64, f64, f64, f64) {
    let mut values: Vec<f64> = series.iter().map(|sample| sample.ns_per_target).collect();
    values.sort_by(f64::total_cmp);
    let median = values[1];
    let min = values[0];
    let max = values[2];
    (
        median,
        min,
        max,
        if median > 0.0 {
            (max - min) * 100.0 / median
        } else {
            0.0
        },
    )
}

fn measure_case(
    candidate: &Candidate,
    prepared: &PreparedCase,
    role: &'static str,
    target_series_ns: u128,
) -> Result<MeasurementRow, String> {
    let mut cpu = prepared.initial_cpu.clone();
    let mut bus = prepared.initial_bus.clone();
    let mut pilot_blocks: u64 = if prepared.stopped { 100 } else { 10 };
    let mut calibration = Vec::with_capacity(6);
    let mut pilot_elapsed_ns;
    loop {
        pilot_elapsed_ns = 0;
        for _ in 0..pilot_blocks {
            pilot_elapsed_ns += run_block(prepared, &mut cpu, &mut bus)?;
        }
        if pilot_elapsed_ns == 0 {
            return Err("pilot clock did not advance".to_string());
        }
        calibration.push(CalibrationStep {
            blocks: pilot_blocks,
            elapsed_ns: pilot_elapsed_ns,
        });
        if pilot_elapsed_ns >= MIN_CALIBRATION_NS || pilot_blocks >= 500_000 {
            break;
        }
        let desired = (MIN_CALIBRATION_NS.saturating_mul(u128::from(pilot_blocks))
            / pilot_elapsed_ns)
            .max(u128::from(pilot_blocks) * 2)
            .min(500_000) as u64;
        pilot_blocks = desired;
    }
    for _ in 0..2 {
        let mut sample_ns = 0;
        for _ in 0..pilot_blocks {
            sample_ns += run_block(prepared, &mut cpu, &mut bus)?;
        }
        calibration.push(CalibrationStep {
            blocks: pilot_blocks,
            elapsed_ns: sample_ns,
        });
        pilot_elapsed_ns = pilot_elapsed_ns.min(sample_ns);
    }
    let estimated_blocks =
        target_series_ns.saturating_mul(u128::from(pilot_blocks)) / pilot_elapsed_ns;
    let mut target_duration_capped = estimated_blocks > 5_000_000;
    let mut blocks = estimated_blocks.clamp(1, 5_000_000) as u64;
    let mut series = Vec::with_capacity(6);
    let first = run_series(prepared, blocks, 0, 0, &mut cpu, &mut bus)?;
    if !target_duration_capped
        && !(target_series_ns * 4 / 5..=target_series_ns * 6 / 5).contains(&first.elapsed_ns)
    {
        calibration.push(CalibrationStep {
            blocks,
            elapsed_ns: first.elapsed_ns,
        });
        let revised = target_series_ns.saturating_mul(u128::from(blocks)) / first.elapsed_ns;
        target_duration_capped = revised > 5_000_000;
        blocks = revised.clamp(1, 5_000_000) as u64;
        series.push(run_series(prepared, blocks, 0, 0, &mut cpu, &mut bus)?);
    } else {
        series.push(first);
    }
    for ordinal in 1..3 {
        series.push(run_series(
            prepared, blocks, 0, ordinal, &mut cpu, &mut bus,
        )?);
    }
    let (mut median, mut min, mut max, mut dispersion) = spread(&series[..3]);
    if dispersion > 5.0 {
        for ordinal in 0..3 {
            series.push(run_series(
                prepared, blocks, 1, ordinal, &mut cpu, &mut bus,
            )?);
        }
        (median, min, max, dispersion) = spread(&series[3..6]);
    }
    let selected = if series.len() == 6 {
        &series[3..6]
    } else {
        &series[..3]
    };
    let duration_valid = target_duration_capped
        || selected.iter().all(|sample| {
            (target_series_ns * 3 / 4..=target_series_ns * 5 / 4).contains(&sample.elapsed_ns)
        });
    let stable = dispersion <= 5.0 && duration_valid;
    Ok(MeasurementRow {
        case_id: candidate.case_id.clone(),
        role,
        target_series_ns,
        fixture_sha256: prepared.fixture_sha256.clone(),
        target_count_per_block: prepared.targets_per_block,
        clocks_per_block: prepared.clocks_per_block,
        pilot_blocks,
        pilot_elapsed_ns,
        calibration,
        blocks_per_series: blocks,
        target_duration_capped,
        series,
        selected_median_ns_per_target: stable.then_some(median),
        min_ns_per_target: stable.then_some(min),
        max_ns_per_target: stable.then_some(max),
        dispersion_pct: Some(dispersion),
        status: if stable {
            "measured"
        } else if !duration_valid {
            "duration_mismatch"
        } else {
            "unstable"
        }
        .to_string(),
        retry_repeat: None,
        confirmed_with: None,
    })
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let contents = fs::read(path).map_err(|error| error.to_string())?;
    Ok(sha256_bytes(&contents))
}

fn sha256_bytes(contents: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(contents);
    format!("{:x}", hash.finalize())
}

fn write_source_snapshot(output_dir: &Path) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let diff = std::process::Command::new("git")
        .args(["diff", "HEAD", "--binary"])
        .current_dir(&root)
        .output()
        .map_err(|error| error.to_string())?;
    if !diff.status.success() {
        return Err("could not capture benchmark source diff".to_string());
    }
    let mut files = serde_json::Map::new();
    for relative in [
        "crates/test_runner/benches/bench_instructions/cli.rs",
        "crates/test_runner/benches/bench_instructions/inventory.rs",
        "crates/test_runner/benches/bench_instructions/candidate_probe.rs",
        "crates/test_runner/benches/bench_instructions/candidate_measure.rs",
        "crates/test_runner/benches/bench_instructions/platform.rs",
        "tools/verification/benchmark_candidates.py",
        "tools/verification/analyze_candidate_benchmarks.py",
    ] {
        files.insert(
            relative.to_string(),
            serde_json::Value::String(sha256_file(&root.join(relative))?),
        );
    }
    let snapshot = serde_json::json!({
        "version": 1,
        "git_diff_sha256": sha256_bytes(&diff.stdout),
        "file_sha256": files,
    });
    fs::write(
        output_dir.join("source_snapshot.json"),
        serde_json::to_vec_pretty(&snapshot).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn command_output(program: &str, args: &[&str]) -> String {
    std::process::Command::new(program)
        .args(args)
        .output()
        .map(|result| String::from_utf8_lossy(&result.stdout).trim().to_string())
        .unwrap_or_else(|error| format!("unavailable: {error}"))
}

fn write_row<T: Serialize>(writer: &mut BufWriter<File>, row: &T) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, row).map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

pub fn retry_agrees(first: f64, second: f64, tolerance_pct: f64) -> bool {
    first.is_finite()
        && second.is_finite()
        && first > 0.0
        && second > 0.0
        && (first - second).abs() * 200.0 / (first + second) <= tolerance_pct
}

fn measure_retry_case(
    writer: &mut BufWriter<File>,
    candidate: &Candidate,
    prepared: &PreparedCase,
    full_median: Option<f64>,
    tolerance_pct: f64,
    max_attempts: usize,
) -> Result<bool, String> {
    let mut measured = Vec::with_capacity(max_attempts + 1);
    if let Some(ns) = full_median {
        measured.push((0, ns));
    }
    let mut last_row = None;
    for repeat in 1..=max_attempts {
        match measure_case(candidate, prepared, "retry_attempt", TARGET_SERIES_NS) {
            Ok(mut row) => {
                row.retry_repeat = Some(repeat);
                write_row(writer, &row)?;
                if let Some(ns) = row.selected_median_ns_per_target {
                    if let Some(&(prior, _)) = measured
                        .iter()
                        .find(|(_, previous)| retry_agrees(*previous, ns, tolerance_pct))
                    {
                        row.role = "candidate";
                        row.confirmed_with = Some(prior);
                        write_row(writer, &row)?;
                        return Ok(true);
                    }
                    measured.push((repeat, ns));
                }
                last_row = Some(row);
            }
            Err(error) => {
                write_row(
                    writer,
                    &serde_json::json!({
                        "case_id": candidate.case_id, "role": "retry_attempt",
                        "retry_repeat": repeat, "status": "failed", "reason": error
                    }),
                )?;
            }
        }
    }
    if let Some(mut row) = last_row {
        row.role = "candidate";
        row.status = "unresolved".to_string();
        row.selected_median_ns_per_target = None;
        row.min_ns_per_target = None;
        row.max_ns_per_target = None;
        write_row(writer, &row)?;
    } else {
        write_row(
            writer,
            &serde_json::json!({
                "case_id": candidate.case_id, "role": "candidate",
                "status": "failed", "reason": "all retry attempts failed"
            }),
        )?;
    }
    Ok(false)
}

fn retry_source(directory: &Path, candidates: &[Candidate]) -> Result<RetrySource, String> {
    let mut full_runs = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("full_"))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    full_runs.sort();
    let source = full_runs.last().ok_or("no full measurement run exists")?;
    let source_environment: serde_json::Value = serde_json::from_slice(
        &fs::read(source.join("environment.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if source_environment["control_policy"] != "once_at_start" {
        return Err(
            "latest full run used an older NOP control policy; measure a new full run before retry"
                .to_string(),
        );
    }
    let source_name = source
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let plan_path = source.join("retry_plan.json");
    let plan: RetryPlan = serde_json::from_slice(
        &fs::read(&plan_path).map_err(|error| format!("prepare retry plan first: {error}"))?,
    )
    .map_err(|error| error.to_string())?;
    if plan.version != 1
        || plan.source_run != source_name
        || plan.full_measurements_sha256 != sha256_file(&source.join("measurements.jsonl"))?
        || plan.candidate_manifest_sha256 != sha256_file(&directory.join("case_candidates.json"))?
        || plan.validation_sha256 != sha256_file(&directory.join("candidate_probe.json"))?
        || !plan.agreement_tolerance_pct.is_finite()
        || !(0.0..=5.0).contains(&plan.agreement_tolerance_pct)
        || !(1..=3).contains(&plan.max_repeat_attempts)
    {
        return Err(
            "retry plan does not match the complete source run or valid limits".to_string(),
        );
    }
    let data =
        fs::read_to_string(source.join("measurements.jsonl")).map_err(|error| error.to_string())?;
    let mut full_rows = HashMap::new();
    let mut control_count = 0usize;
    for line in data.lines() {
        let row: serde_json::Value =
            serde_json::from_str(line).map_err(|error| error.to_string())?;
        if row["role"] == "control" {
            control_count += 1;
            if row["status"] != "measured" {
                return Err("full-run NOP control was unstable".to_string());
            }
        }
        if row["role"] == "candidate" {
            let id = row["case_id"]
                .as_str()
                .ok_or("candidate row lacks case ID")?
                .to_string();
            if full_rows.insert(id.clone(), row).is_some() {
                return Err(format!("duplicate case ID in full run: {id}"));
            }
        }
    }
    if control_count != 1 || full_rows.len() != candidates.len() {
        return Err("full run is incomplete".to_string());
    }
    let indices_by_id = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| (candidate.case_id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut selected = Vec::with_capacity(plan.selected.len());
    let mut selected_ids = HashSet::new();
    let mut full_medians = HashMap::new();
    for entry in plan.selected {
        let Some(&index) = indices_by_id.get(entry.case_id.as_str()) else {
            return Err(format!(
                "retry case is absent from current manifest: {}",
                entry.case_id
            ));
        };
        let row = full_rows
            .get(&entry.case_id)
            .ok_or_else(|| format!("retry case is absent from full run: {}", entry.case_id))?;
        if !selected_ids.insert(entry.case_id.clone())
            || !matches!(entry.reason.as_str(), "invalid" | "raw_time_difference")
            || row["selected_median_ns_per_target"].as_f64() != entry.full_ns_per_target
            || (entry.reason == "invalid") != (row["status"] != "measured")
        {
            return Err(format!(
                "retry plan entry does not match full run: {}",
                entry.case_id
            ));
        }
        if let Some(ns) = entry.full_ns_per_target {
            full_medians.insert(entry.case_id, ns);
        }
        selected.push(index);
    }
    if full_rows
        .iter()
        .any(|(id, row)| row["status"] != "measured" && !selected_ids.contains(id))
    {
        return Err("retry plan omits invalid full-run cases".to_string());
    }
    selected.sort_unstable();
    Ok(RetrySource {
        indices: selected,
        source_name,
        full_medians,
        plan_sha256: sha256_file(&plan_path)?,
        agreement_tolerance_pct: plan.agreement_tolerance_pct,
        max_repeat_attempts: plan.max_repeat_attempts,
    })
}

pub fn measure_candidates(directory: &Path, pilot: bool, retry: bool) -> Result<(), String> {
    let candidates: Vec<Candidate> = serde_json::from_slice(
        &fs::read(directory.join("case_candidates.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let validated: Vec<ExpectedProbe> = serde_json::from_slice(
        &fs::read(directory.join("candidate_probe.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if candidates.is_empty() || candidates.len() != validated.len() {
        return Err("candidate manifest and validation report have different coverage".to_string());
    }
    let retry_details = if retry {
        Some(retry_source(directory, &candidates)?)
    } else {
        None
    };
    let selected: Vec<usize> = if let Some(details) = &retry_details {
        details.indices.clone()
    } else if pilot {
        // Exercise short/long, memory, control, exception and STOP fixtures.
        let desired = [
            "4E71", "0033", "88C3", "6602", "4E43", "4E72", "4E73", "4893",
        ];
        desired
            .iter()
            .filter_map(|opcode| {
                candidates
                    .iter()
                    .position(|candidate| candidate.representative_opcode == *opcode)
            })
            .collect()
    } else {
        (0..candidates.len()).collect()
    };
    if (pilot && selected.len() != 8) || (!pilot && !retry && selected.len() != candidates.len()) {
        return Err(format!(
            "pilot selection incomplete: {} cases",
            selected.len()
        ));
    }
    if retry && selected.is_empty() {
        println!("No cases selected for retry; no measurement session was created");
        return Ok(());
    }
    let host = HostEnvironment::detect();
    #[cfg(windows)]
    let affinity =
        windows::AffinityGuard::pin(host.p_core_id.ok_or("no performance core selected")?)?;
    #[cfg(windows)]
    let (core_id, priority_class) = (Some(affinity.core), affinity.priority_class);
    #[cfg(not(windows))]
    let (core_id, priority_class) = (host.p_core_id, 0u32);
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    let output_dir = directory.join(format!(
        "{}_{run_id}",
        if pilot {
            "pilot"
        } else if retry {
            "retry"
        } else {
            "full"
        }
    ));
    fs::create_dir_all(&output_dir).map_err(|error| error.to_string())?;
    write_source_snapshot(&output_dir)?;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let environment = serde_json::json!({
        "version": 1,
        "run_id": run_id,
        "candidate_count": candidates.len(),
        "measured_candidate_count": selected.len(),
        "pilot": pilot,
        "retry_source": retry_details.as_ref().map(|details| &details.source_name),
        "retry_plan_sha256": retry_details.as_ref().map(|details| &details.plan_sha256),
        "host_os": host.platform_name,
        "host_arch": std::env::consts::ARCH,
        "logical_processors": std::thread::available_parallelism().map(|count| count.get()).unwrap_or(0),
        "core_id": core_id,
        "affinity_verified": cfg!(windows),
        "priority_class": priority_class,
        "active_power_plan": command_output("powercfg", &["/getactivescheme"]),
        "git_head": command_output("git", &["rev-parse", "HEAD"]),
        "git_status": command_output("git", &["status", "--short"]),
        "rustc_version": command_output("rustc", &["--version"]),
        "binary_sha256": sha256_file(&executable)?,
        "candidate_manifest_sha256": sha256_file(&directory.join("case_candidates.json"))?,
        "validation_sha256": sha256_file(&directory.join("candidate_probe.json"))?,
        "target_series_ns": TARGET_SERIES_NS,
        "control_policy": "once_at_start",
        "control_target_series_ns": CONTROL_TARGET_SERIES_NS,
        "series_per_attempt": 3,
        "dispersion_retry_threshold_pct": 5.0,
    });
    fs::write(
        output_dir.join("environment.json"),
        serde_json::to_vec_pretty(&environment).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let file =
        File::create(output_dir.join("measurements.jsonl")).map_err(|error| error.to_string())?;
    let mut writer = BufWriter::new(file);
    let nop_index = candidates
        .iter()
        .position(|candidate| candidate.representative_opcode == "4E71")
        .ok_or("NOP control candidate missing")?;
    let nop = prepare_case(&candidates[nop_index], &validated[nop_index])?;
    let mut failures = 0usize;
    let control = measure_case(
        &candidates[nop_index],
        &nop,
        "control",
        CONTROL_TARGET_SERIES_NS,
    )?;
    write_row(&mut writer, &control)?;
    println!(
        "Initial NOP control: {:.2} ns/target, dispersion {:.2}%, status {}",
        control.selected_median_ns_per_target.unwrap_or(0.0),
        control.dispersion_pct.unwrap_or(0.0),
        control.status
    );
    if control.status != "measured" {
        return Err("initial NOP control was unstable; candidate timing was skipped".to_string());
    }
    for (ordinal, &index) in selected.iter().enumerate() {
        let candidate = &candidates[index];
        match prepare_case(candidate, &validated[index]) {
            Ok(prepared) if retry => {
                let details = retry_details.as_ref().ok_or("retry details missing")?;
                if !measure_retry_case(
                    &mut writer,
                    candidate,
                    &prepared,
                    details.full_medians.get(&candidate.case_id).copied(),
                    details.agreement_tolerance_pct,
                    details.max_repeat_attempts,
                )? {
                    failures += 1;
                }
            }
            Ok(prepared) => {
                match measure_case(candidate, &prepared, "candidate", TARGET_SERIES_NS) {
                    Ok(row) => {
                        if row.status != "measured" {
                            failures += 1;
                        }
                        write_row(&mut writer, &row)?;
                    }
                    Err(error) => {
                        failures += 1;
                        let diagnostic = serde_json::json!({"case_id": candidate.case_id, "role": "candidate", "status": "failed", "reason": error});
                        write_row(&mut writer, &diagnostic)?;
                    }
                }
            }
            Err(error) => {
                failures += 1;
                let diagnostic = serde_json::json!({"case_id": candidate.case_id, "role": "candidate", "status": "failed", "reason": error});
                write_row(&mut writer, &diagnostic)?;
            }
        }
        if ordinal % 25 == 24 || ordinal + 1 == selected.len() {
            println!(
                "Measured {}/{} cases; {} unstable or failed",
                ordinal + 1,
                selected.len(),
                failures
            );
        }
    }
    println!(
        "Raw measurement rows: {}",
        output_dir.join("measurements.jsonl").display()
    );
    if failures > 0 {
        Err(format!(
            "{failures} cases were unstable or failed; diagnostics were retained"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn timed_jump_block_stops_after_700_jumps() {
        let candidate: super::Candidate = serde_json::from_value(serde_json::json!({
            "case_id": "jump:4ED3",
            "representative_opcode": "4ED3",
            "path": "normal",
            "index_choices": "()",
            "cpu_mode": "supervisor",
        }))
        .expect("literal candidate");
        let opcode = 0x4ED3;
        let words = super::instruction_words(opcode, &candidate);
        let mut cpu = super::Cpu::new();
        let mut bus = super::PhysicalMemory::new();
        bus.map_chip_ram_to_low_memory();
        let mut fingerprint = String::new();
        let audit = super::validate_candidate(
            &mut bus,
            &mut cpu,
            &candidate,
            opcode,
            &words,
            &mut fingerprint,
            None,
        );
        let expected = super::ExpectedProbe {
            case_id: candidate.case_id.clone(),
            fixture_sha256: fingerprint,
            final_state_sha256: super::final_state_sha256(&cpu, &bus),
            block_status: audit.0,
            replay_status: "identical".to_string(),
            block_targets: audit.1,
            block_clocks: audit.2,
        };
        let prepared = super::prepare_case(&candidate, &expected).expect("validated case");
        let mut timed_cpu = super::Cpu::new();
        let mut timed_bus = super::PhysicalMemory::new();
        timed_bus.map_chip_ram_to_low_memory();
        super::run_block(&prepared, &mut timed_cpu, &mut timed_bus).expect("timed block");
        assert_eq!(timed_cpu, prepared.final_cpu);
        assert_eq!(
            super::final_state_sha256(&timed_cpu, &timed_bus),
            expected.final_state_sha256
        );
    }
}
