#![allow(unsafe_code)]

//! Measured CPU stepping for fixtures that passed validation before timing.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cpu::Cpu;
use physical_memory::PhysicalMemory;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::candidate_probe::{
    final_state_sha256, instruction_words, validate_candidate, Candidate,
};
use super::platform::HostEnvironment;

const TARGET_SERIES_NS: u128 = 400_000_000;
const MIN_CALIBRATION_NS: u128 = 5_000_000;
const MAX_INSTRUCTIONS_PER_BLOCK: usize = 5000;

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
}

#[derive(Debug, Serialize)]
struct Series {
    attempt: u8,
    ordinal: u8,
    blocks: u64,
    target_count: u64,
    elapsed_ns: u128,
    ns_per_target: f64,
}

#[derive(Debug, Serialize)]
struct CalibrationStep {
    blocks: u64,
    elapsed_ns: u128,
}

#[derive(Debug, Serialize)]
struct MeasurementRow {
    case_id: String,
    role: &'static str,
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
            "sequential_700_reached_exit" | "single_target_stop"
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
    let started = Instant::now();
    let mut steps = 0;
    while !cpu.state.is_stopped()
        && !cpu.state.is_halted()
        && cpu.state.instruction_pc() != 0x4FFE
        && steps < MAX_INSTRUCTIONS_PER_BLOCK
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
        TARGET_SERIES_NS.saturating_mul(u128::from(pilot_blocks)) / pilot_elapsed_ns;
    let mut target_duration_capped = estimated_blocks > 5_000_000;
    let mut blocks = estimated_blocks.clamp(1, 5_000_000) as u64;
    let mut series = Vec::with_capacity(6);
    let first = run_series(prepared, blocks, 0, 0, &mut cpu, &mut bus)?;
    if !target_duration_capped
        && !(TARGET_SERIES_NS * 4 / 5..=TARGET_SERIES_NS * 6 / 5).contains(&first.elapsed_ns)
    {
        calibration.push(CalibrationStep {
            blocks,
            elapsed_ns: first.elapsed_ns,
        });
        let revised = TARGET_SERIES_NS.saturating_mul(u128::from(blocks)) / first.elapsed_ns;
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
            (TARGET_SERIES_NS * 3 / 4..=TARGET_SERIES_NS * 5 / 4).contains(&sample.elapsed_ns)
        });
    let stable = dispersion <= 5.0 && duration_valid;
    Ok(MeasurementRow {
        case_id: candidate.case_id.clone(),
        role,
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
    })
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let contents = fs::read(path).map_err(|error| error.to_string())?;
    let mut hash = Sha256::new();
    hash.update(contents);
    Ok(format!("{:x}", hash.finalize()))
}

fn command_output(program: &str, args: &[&str]) -> String {
    std::process::Command::new(program)
        .args(args)
        .output()
        .map(|result| String::from_utf8_lossy(&result.stdout).trim().to_string())
        .unwrap_or_else(|error| format!("unavailable: {error}"))
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn GetPriorityClass(process: *mut c_void) -> u32;
        fn GetCurrentThread() -> *mut c_void;
        fn SetThreadAffinityMask(thread: *mut c_void, mask: usize) -> usize;
        fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    }

    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    impl FileTime {
        fn value(&self) -> u64 {
            (u64::from(self.high) << 32) | u64::from(self.low)
        }
    }

    pub struct AffinityGuard {
        previous: usize,
        pub core: usize,
        pub priority_class: u32,
    }

    impl AffinityGuard {
        pub fn pin(core: usize) -> Result<Self, String> {
            let mask = 1usize
                .checked_shl(core as u32)
                .ok_or("core exceeds affinity mask")?;
            let priority_class = unsafe { GetPriorityClass(GetCurrentProcess()) };
            if priority_class != 0x20 {
                return Err(format!(
                    "expected normal process priority, found {priority_class:#X}"
                ));
            }
            let previous = unsafe { SetThreadAffinityMask(GetCurrentThread(), mask) };
            if previous == 0 {
                return Err(format!("failed to pin logical CPU {core}"));
            }
            let confirmed_previous = unsafe { SetThreadAffinityMask(GetCurrentThread(), mask) };
            if confirmed_previous != mask {
                unsafe {
                    SetThreadAffinityMask(GetCurrentThread(), previous);
                }
                return Err(format!("logical CPU {core} affinity was not confirmed"));
            }
            Ok(Self {
                previous,
                core,
                priority_class,
            })
        }
    }

    impl Drop for AffinityGuard {
        fn drop(&mut self) {
            unsafe {
                SetThreadAffinityMask(GetCurrentThread(), self.previous);
            }
        }
    }

    fn times() -> Result<(u64, u64), String> {
        let (mut idle, mut kernel, mut user) = (
            FileTime::default(),
            FileTime::default(),
            FileTime::default(),
        );
        if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
            return Err("GetSystemTimes failed".to_string());
        }
        Ok((idle.value(), kernel.value().wrapping_add(user.value())))
    }

    pub fn busy_percent() -> Result<f64, String> {
        let before = times()?;
        std::thread::sleep(std::time::Duration::from_millis(300));
        let after = times()?;
        let total = after.1.wrapping_sub(before.1);
        if total == 0 {
            return Err("system time did not advance".to_string());
        }
        Ok(100.0 * total.saturating_sub(after.0.wrapping_sub(before.0)) as f64 / total as f64)
    }
}

#[cfg(windows)]
fn idle_check(threshold: f64) -> Result<f64, String> {
    let started = Instant::now();
    loop {
        let busy = windows::busy_percent()?;
        if busy <= threshold {
            return Ok(busy);
        }
        if started.elapsed() >= Duration::from_secs(60) {
            return Err(format!("host remained busy ({busy:.2}% > {threshold:.2}%)"));
        }
    }
}

#[cfg(not(windows))]
fn idle_check(_threshold: f64) -> Result<f64, String> {
    Ok(0.0)
}

fn retry_source(
    directory: &Path,
    candidates: &[Candidate],
) -> Result<(Vec<usize>, String), String> {
    let mut full_runs = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("full_"))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    full_runs.sort();
    let source = full_runs.last().ok_or("no full measurement run exists")?;
    let data =
        fs::read_to_string(source.join("measurements.jsonl")).map_err(|error| error.to_string())?;
    let mut seen = HashSet::new();
    let mut retry_ids = HashSet::new();
    let mut control_unstable = false;
    for line in data.lines() {
        let row: serde_json::Value =
            serde_json::from_str(line).map_err(|error| error.to_string())?;
        if row["role"] == "control" {
            control_unstable = row["status"] != "measured";
        }
        if row["role"] == "candidate" {
            let id = row["case_id"]
                .as_str()
                .ok_or("candidate row lacks case ID")?;
            if !seen.insert(id.to_string()) {
                return Err(format!("duplicate case ID in full run: {id}"));
            }
            if row["status"] != "measured" || control_unstable {
                retry_ids.insert(id.to_string());
            }
        }
    }
    if seen.len() != candidates.len() || retry_ids.is_empty() {
        return Err(format!(
            "full run is incomplete or has no retry cases: {} rows, {} retries",
            seen.len(),
            retry_ids.len()
        ));
    }
    let selected = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| retry_ids.contains(&candidate.case_id).then_some(index))
        .collect::<Vec<_>>();
    if selected.len() != retry_ids.len() {
        return Err("retry IDs do not match the current candidate manifest".to_string());
    }
    Ok((
        selected,
        source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    ))
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
    let (selected, retry_source_name): (Vec<usize>, Option<String>) = if retry {
        let (selected, source_name) = retry_source(directory, &candidates)?;
        (selected, Some(source_name))
    } else if pilot {
        // Exercise short/long, memory, control, exception and STOP fixtures.
        let desired = [
            "4E71", "0033", "88C3", "6602", "4E43", "4E72", "4E73", "4893",
        ];
        (
            desired
                .iter()
                .filter_map(|opcode| {
                    candidates
                        .iter()
                        .position(|candidate| candidate.representative_opcode == *opcode)
                })
                .collect(),
            None,
        )
    } else {
        ((0..candidates.len()).collect(), None)
    };
    if (pilot && selected.len() != 8) || (!pilot && !retry && selected.len() != candidates.len()) {
        return Err(format!(
            "pilot selection incomplete: {} cases",
            selected.len()
        ));
    }
    let host = HostEnvironment::detect();
    #[cfg(windows)]
    let affinity =
        windows::AffinityGuard::pin(host.p_core_id.ok_or("no performance core selected")?)?;
    #[cfg(windows)]
    let (core_id, priority_class) = (Some(affinity.core), affinity.priority_class);
    #[cfg(not(windows))]
    let (core_id, priority_class) = (host.p_core_id, 0u32);
    #[cfg(windows)]
    let baseline_busy = windows::busy_percent()?;
    #[cfg(not(windows))]
    let baseline_busy = 0.0;
    if baseline_busy > 12.0 {
        return Err(format!(
            "host idle baseline is too busy: {baseline_busy:.2}%"
        ));
    }
    let threshold = baseline_busy + 3.0;
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
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let environment = serde_json::json!({
        "version": 1,
        "run_id": run_id,
        "candidate_count": candidates.len(),
        "measured_candidate_count": selected.len(),
        "pilot": pilot,
        "retry_source": retry_source_name,
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
        "series_per_attempt": 3,
        "dispersion_retry_threshold_pct": 5.0,
        "initial_host_busy_pct": baseline_busy,
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
    let mut unstable_controls = 0usize;
    for (ordinal, &index) in selected.iter().enumerate() {
        if ordinal % 100 == 0 {
            let busy = idle_check(threshold)?;
            let control = measure_case(&candidates[nop_index], &nop, "control")?;
            serde_json::to_writer(&mut writer, &control).map_err(|error| error.to_string())?;
            writer.write_all(b"\n").map_err(|error| error.to_string())?;
            writer.flush().map_err(|error| error.to_string())?;
            if control.status != "measured" {
                unstable_controls += 1;
            }
            println!("Control before case {ordinal}: {:.2} ns/target, dispersion {:.2}%, host idle {:.2}%, status {}", control.selected_median_ns_per_target.unwrap_or(0.0), control.dispersion_pct.unwrap_or(0.0), busy, control.status);
        }
        let candidate = &candidates[index];
        let result = prepare_case(candidate, &validated[index])
            .and_then(|prepared| measure_case(candidate, &prepared, "candidate"));
        match result {
            Ok(row) => {
                if row.status != "measured" {
                    failures += 1;
                }
                serde_json::to_writer(&mut writer, &row).map_err(|error| error.to_string())?;
                writer.write_all(b"\n").map_err(|error| error.to_string())?;
            }
            Err(error) => {
                failures += 1;
                let diagnostic = serde_json::json!({"case_id": candidate.case_id, "role": "candidate", "status": "failed", "reason": error});
                serde_json::to_writer(&mut writer, &diagnostic)
                    .map_err(|write_error| write_error.to_string())?;
                writer
                    .write_all(b"\n")
                    .map_err(|write_error| write_error.to_string())?;
            }
        }
        writer.flush().map_err(|error| error.to_string())?;
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
    if failures > 0 || unstable_controls > 0 {
        Err(format!(
            "{failures} cases and {unstable_controls} NOP controls were unstable or failed; diagnostics were retained"
        ))
    } else {
        Ok(())
    }
}
