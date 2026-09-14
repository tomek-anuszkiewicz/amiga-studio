//! Benchmark Runner Orchestrator & Dual-Loop Harness
//!
//! Executes the inner unrolled instruction block (K = 700) and outer multi-pass
//! statistical loop with compiler optimization defenses and NOP baseline subtraction
//! per Obsidian/Amiga/Design/CPU Instruction Benchmarking.md.

use m68000::Cpu;
use physical_memory::MemoryBus;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::anomaly::{evaluate_anomaly, RegressionStatus};
use super::builder::BenchmarkProgramBuilder;
use super::catalog::{filter_specs, find_spec_by_id, BenchmarkSpec, InstructionCategory};
use super::persistence::{
    format_timestamp_filename, load_historical_baseline, persist_benchmark_report,
    BenchmarkEnvironmentInfo, BenchmarkItemResult, BenchmarkSuiteReport,
};
use super::platform::HostEnvironment;
use super::stats::{analyze_pass_samples, BenchmarkMetrics};

/// Benchmark execution scaling profile
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkProfile {
    Quick,
    Standard,
    Thorough,
}

impl BenchmarkProfile {
    pub const fn default_passes(&self) -> usize {
        match self {
            Self::Quick => 3,
            Self::Standard => 7,
            Self::Thorough => 15,
        }
    }

    pub const fn default_iterations(&self) -> usize {
        match self {
            Self::Quick => 15,
            Self::Standard => 1_428,
            Self::Thorough => 14_285,
        }
    }

    pub const fn dir_name(&self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Thorough => "thorough",
        }
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Thorough => "thorough",
        }
    }
}

/// Global benchmark execution configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub profile: BenchmarkProfile,
    pub filter: Option<String>,
    pub unroll_k: usize,
    pub passes: usize,
    pub iterations: usize,
    pub out_dir: PathBuf,
    pub pin_core: bool,
    pub verbose: bool,
}

/// Resolves the benchmark output root directory path (`tests/benchmarks`)
pub fn resolve_benchmarks_dir() -> PathBuf {
    let candidates = [
        PathBuf::from("tests/benchmarks"),
        PathBuf::from("../../tests/benchmarks"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = Path::new(&manifest_dir).join("../../tests/benchmarks");
        return p;
    }

    PathBuf::from("tests/benchmarks")
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            profile: BenchmarkProfile::Standard,
            filter: None,
            unroll_k: 700,
            passes: BenchmarkProfile::Standard.default_passes(),
            iterations: BenchmarkProfile::Standard.default_iterations(),
            out_dir: resolve_benchmarks_dir(),
            pin_core: true,
            verbose: true,
        }
    }
}

/// Result of executing a full benchmark across all outer passes
#[derive(Debug, Clone)]
pub struct BenchmarkExecutionResult {
    pub spec: BenchmarkSpec,
    pub metrics: BenchmarkMetrics,
    pub differential_net_ns_op: Option<f64>,
    pub anomaly_flag: bool,
    pub anomaly_notes: Vec<String>,
    pub regression_status: Option<RegressionStatus>,
    pub regression_delta_pct: Option<f64>,
}

/// Main runner entry point executing benchmark catalog against cycle-exact CPU core
pub fn run_benchmark_suite(config: &BenchmarkConfig) -> Result<BenchmarkSuiteReport, String> {
    let env = HostEnvironment::detect();
    if config.pin_core && env.affinity_supported {
        let _ = env.pin_to_p_core();
    }

    let specs = match &config.filter {
        Some(f) => filter_specs(f),
        None => filter_specs("*"),
    };

    if specs.is_empty() {
        return Err("No benchmark specifications matched the provided filter".to_string());
    }

    // Always ensure BASE-00 (NOP baseline) is present for differential calibration
    let nop_spec = find_spec_by_id("BASE-00")
        .copied()
        .ok_or_else(|| "BASE-00 specification missing".to_string())?;

    let mut bus = MemoryBus::new();
    let mut cpu = Cpu::new();

    if config.verbose {
        println!("===============================================================================");
        println!(
            "M68000 Cycle-Exact Instruction Benchmarking Suite [Profile: {}]",
            config.profile.name()
        );
        println!(
            "Platform: {}, P-Core Pinning: {} (Core ID: {:?})",
            env.platform_name,
            config.pin_core && env.affinity_supported,
            env.p_core_id
        );
        println!(
            "Unroll Factor: K = {}, Outer Passes: {}",
            config.unroll_k, config.passes
        );
        println!("Target Specifications to Profile: {}", specs.len());
        println!("===============================================================================");
    }

    // 1. Warm-up Pass & BASE-00 Calibration
    let nop_result = execute_single_spec(
        &nop_spec,
        config.unroll_k,
        config.passes,
        config.iterations,
        &mut cpu,
        &mut bus,
        None,
        None,
    );

    let baseline_nop_ns_op = nop_result.metrics.host_ns_per_instruction;
    let baseline_nop_ns_cck = nop_result.metrics.host_ns_per_guest_cck;

    if config.verbose {
        println!(
            "[*] Baseline Calibrated [BASE-00 NOP]: {:.2} ns/op ({:.3} ns/CCK) | {:.2} MIPS",
            baseline_nop_ns_op, baseline_nop_ns_cck, nop_result.metrics.host_mips
        );
        println!("-------------------------------------------------------------------------------");
    }

    let profile_name = config.profile.name();
    let target_out_dir = if config.out_dir.ends_with(profile_name) {
        config.out_dir.clone()
    } else {
        config.out_dir.join(profile_name)
    };

    let baseline_report = load_historical_baseline(&target_out_dir);

    let baseline_nop_ns = baseline_report.as_ref().and_then(|rep| {
        rep.results
            .iter()
            .find(|item| item.mnemonic == "NOP")
            .map(|item| item.host_ns_per_instruction)
    });

    let cross_hardware = match (&baseline_report, baseline_nop_ns) {
        (Some(base), Some(base_nop)) => {
            let nop_divergence = ((baseline_nop_ns_op - base_nop) / base_nop).abs();
            nop_divergence > 0.15
                || base.environment.host_arch != std::env::consts::ARCH
                || base.environment.host_os != env.platform_name
        }
        _ => false,
    };

    if config.verbose {
        if let Some(ref base) = baseline_report {
            println!(
                "[*] Loaded Golden Baseline (Profile: {}, Timestamp: {})",
                base.environment.profile, base.timestamp
            );
            if cross_hardware {
                println!(
                    "[*] Notice: Cross-hardware diffing active (Host NOP: {:.2}ns vs Baseline NOP: {:.2}ns). Diffing via NOP-normalized ratio.",
                    baseline_nop_ns_op,
                    baseline_nop_ns.unwrap_or(0.0)
                );
            }
            println!(
                "-------------------------------------------------------------------------------"
            );
        }
    }

    let mut results = Vec::new();
    let mut item_results = Vec::new();

    // If BASE-00 is explicitly part of the run, record it
    if specs.iter().any(|s| s.id == "BASE-00") {
        results.push(nop_result.clone());
    }

    // Benchmark remaining filtered specifications
    for spec in specs {
        if spec.id == "BASE-00" {
            continue;
        }

        let res = execute_single_spec(
            spec,
            config.unroll_k,
            config.passes,
            config.iterations,
            &mut cpu,
            &mut bus,
            Some(baseline_nop_ns_cck),
            Some(baseline_nop_ns_op),
        );

        let (baseline_delta, baseline_reg_status) = if let Some(ref base_rep) = baseline_report {
            base_rep
                .results
                .iter()
                .find(|item| item.variant == res.spec.representative_syntax)
                .map(|matched| {
                    if cross_hardware && baseline_nop_ns.is_some() && baseline_nop_ns_op > 0.0 {
                        let base_nop = baseline_nop_ns.unwrap();
                        let curr_ratio = res.metrics.host_ns_per_instruction / baseline_nop_ns_op;
                        let base_ratio = matched.host_ns_per_instruction / base_nop;
                        let (status, delta) =
                            super::anomaly::evaluate_historical_ratio_diff(curr_ratio, base_ratio);
                        (Some(delta), Some(status))
                    } else {
                        let (status, delta) = super::anomaly::evaluate_historical_diff(
                            res.metrics.host_ns_per_instruction,
                            matched.host_ns_per_instruction,
                        );
                        (Some(delta), Some(status))
                    }
                })
                .unwrap_or((None, None))
        } else {
            (None, None)
        };

        let diff_net = (res.metrics.host_ns_per_instruction - baseline_nop_ns_op).max(0.0);

        if config.verbose {
            let flag = if res.anomaly_flag {
                "⚠️ ANOMALY"
            } else if let Some(status) = baseline_reg_status {
                match status {
                    super::anomaly::RegressionStatus::SevereRegressionFailure => "🚨 REGRESSION",
                    super::anomaly::RegressionStatus::MinorRegressionWarning => "⚠️ MINOR REG",
                    _ => "OK",
                }
            } else {
                "OK"
            };

            let base_str = if let Some(delta) = baseline_delta {
                if cross_hardware {
                    format!("| Base Δ: {:>+5.1}% (R_nop)", delta)
                } else {
                    format!("| Base Δ: {:>+5.1}%", delta)
                }
            } else {
                String::new()
            };

            println!(
                "[{:8}] {:<20} | {:>6.2} ns/op | {:>5.3} ns/CCK | Net Δ: {:>5.2} ns | {:>6.1} MIPS {} | [{}]",
                res.spec.id,
                res.spec.representative_syntax,
                res.metrics.host_ns_per_instruction,
                res.metrics.host_ns_per_guest_cck,
                diff_net,
                res.metrics.host_mips,
                base_str,
                flag
            );
        }

        results.push(res);
    }

    // Convert execution results to serializable schema
    for r in &results {
        let diff_net = (r.metrics.host_ns_per_instruction - baseline_nop_ns_op).max(0.0);
        let opcode_hex = r
            .spec
            .opcode_words
            .iter()
            .map(|w| format!("{:04X}", w))
            .collect::<Vec<_>>()
            .join(" ");

        let mut notes = r.anomaly_notes.clone();
        let mut anomaly_flag = r.anomaly_flag;
        if let Some(ref base_rep) = baseline_report {
            if let Some(matched) = base_rep
                .results
                .iter()
                .find(|item| item.variant == r.spec.representative_syntax)
            {
                let (status, delta) =
                    if cross_hardware && baseline_nop_ns.is_some() && baseline_nop_ns_op > 0.0 {
                        let base_nop = baseline_nop_ns.unwrap();
                        let curr_ratio = r.metrics.host_ns_per_instruction / baseline_nop_ns_op;
                        let base_ratio = matched.host_ns_per_instruction / base_nop;
                        super::anomaly::evaluate_historical_ratio_diff(curr_ratio, base_ratio)
                    } else {
                        super::anomaly::evaluate_historical_diff(
                            r.metrics.host_ns_per_instruction,
                            matched.host_ns_per_instruction,
                        )
                    };
                if status == super::anomaly::RegressionStatus::SevereRegressionFailure {
                    anomaly_flag = true;
                    let desc = if cross_hardware {
                        format!("Severe regression vs baseline ratio: {:+.1}%", delta)
                    } else {
                        format!("Severe regression vs baseline: {:+.1}%", delta)
                    };
                    notes.push(desc);
                } else if status == super::anomaly::RegressionStatus::MinorRegressionWarning {
                    let desc = if cross_hardware {
                        format!("Minor regression vs baseline ratio: {:+.1}%", delta)
                    } else {
                        format!("Minor regression vs baseline: {:+.1}%", delta)
                    };
                    notes.push(desc);
                }
            }
        }

        item_results.push(BenchmarkItemResult {
            mnemonic: r.spec.mnemonic.to_string(),
            variant: r.spec.representative_syntax.to_string(),
            addressing_mode: r.spec.addressing_mode.name().to_string(),
            category: r.spec.category.name().to_string(),
            opcode_hex,
            amiga_cck_cycles: r.spec.amiga_cck,
            total_guest_instructions: r.metrics.total_guest_instructions,
            total_guest_cck: r.metrics.total_guest_cck,
            host_duration_median_ms: r.metrics.host_duration_median_ms,
            host_duration_min_ms: r.metrics.host_duration_min_ms,
            host_duration_max_ms: r.metrics.host_duration_max_ms,
            host_jitter_pct: r.metrics.host_jitter_pct,
            host_ns_per_instruction: r.metrics.host_ns_per_instruction,
            host_ns_per_guest_cck: r.metrics.host_ns_per_guest_cck,
            host_mips: r.metrics.host_mips,
            differential_net_ns_op: Some(diff_net),
            anomaly_flag,
            anomaly_notes: notes,
        });
    }

    let report = BenchmarkSuiteReport {
        version: 1,
        timestamp: format_timestamp_filename(),
        environment: BenchmarkEnvironmentInfo {
            host_os: env.platform_name.to_string(),
            host_arch: std::env::consts::ARCH.to_string(),
            rustc_version: "rustc 1.82+".to_string(),
            profile: config.profile.name().to_string(),
            unroll_factor: config.unroll_k,
            outer_passes: config.passes,
            p_core_pinning_active: config.pin_core && env.affinity_supported,
        },
        results: item_results,
    };

    // Persist JSON and CSV reports into profile-isolated subfolder
    let (json_path, csv_path) = persist_benchmark_report(&report, &target_out_dir)?;
    if config.verbose {
        println!("-------------------------------------------------------------------------------");
        println!("Benchmark report saved to: {}", json_path.display());
        println!("Tabular CSV saved to:      {}", csv_path.display());
    }

    Ok(report)
}

/// Executes all passes for a single specification with noise compensation & compiler defense
pub fn execute_single_spec(
    spec: &BenchmarkSpec,
    unroll_k: usize,
    passes_count: usize,
    iterations: usize,
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
    family_baseline_ns_cck: Option<f64>,
    register_baseline_ns_op: Option<f64>,
) -> BenchmarkExecutionResult {
    let program = BenchmarkProgramBuilder::new(*spec)
        .with_unroll(unroll_k)
        .build();

    let total_pass_ops = program.total_ops_per_pass * (iterations as u64);
    let total_pass_cck = program.total_cck_per_pass * (iterations as u64);

    // 1. Warm-up Pass (unmeasured)
    execute_pass_inner(&program, iterations, cpu, bus);

    let mut samples_ns = Vec::with_capacity(passes_count);
    let mut retry_count = 0;
    let mut any_timed_out = false;

    loop {
        for _ in 0..passes_count {
            let (dur_ns, timed_out) = execute_pass_inner(&program, iterations, cpu, bus);
            samples_ns.push(dur_ns);
            if timed_out {
                any_timed_out = true;
            }
        }

        let metrics = analyze_pass_samples(&samples_ns, total_pass_ops, total_pass_cck);

        // "Wait Out the Storm" cooldown: If jitter > 3%, sleep and extend passes
        if metrics.host_jitter_pct > 3.0 && retry_count < 2 {
            retry_count += 1;
            samples_ns.clear();
            std::thread::sleep(std::time::Duration::from_millis(500));
            continue;
        }

        let is_mem = match spec.category {
            InstructionCategory::DataMovement => spec.category_tag.contains("mem"),
            InstructionCategory::Arithmetic => spec.category_tag.contains("mem"),
            InstructionCategory::Comparison => spec.category_tag.contains("mem"),
            InstructionCategory::Bcd => spec.category_tag.contains("mem"),
            _ => false,
        };

        let mut verdict = evaluate_anomaly(
            metrics.host_ns_per_guest_cck,
            family_baseline_ns_cck,
            register_baseline_ns_op,
            metrics.host_jitter_pct,
            is_mem,
        );

        if any_timed_out {
            verdict.is_anomaly = true;
            verdict
                .anomaly_types
                .push(super::anomaly::AnomalyType::ExecutionTimeoutOrInfiniteLoop);
            verdict.notes.push(
                "Execution exceeded 10x expected cycle threshold (possible infinite loop abort)"
                    .to_string(),
            );
        }

        let diff_net =
            register_baseline_ns_op.map(|base| (metrics.host_ns_per_instruction - base).max(0.0));

        return BenchmarkExecutionResult {
            spec: *spec,
            metrics,
            differential_net_ns_op: diff_net,
            anomaly_flag: verdict.is_anomaly,
            anomaly_notes: verdict.notes,
            regression_status: None,
            regression_delta_pct: None,
        };
    }
}

/// Executes a single pass of the synthesized benchmark block, measuring host elapsed time in nanoseconds
#[inline(never)]
fn execute_pass_inner(
    program: &super::builder::BenchmarkProgram,
    iterations: usize,
    cpu: &mut Cpu,
    bus: &mut MemoryBus,
) -> (f64, bool) {
    let max_cycles = (program.total_cck_per_pass.max(1_000) * 10) as u64;
    let mut has_timed_out = false;

    let start = Instant::now();

    for _ in 0..iterations {
        program.inject_into(cpu, bus);

        let mut elapsed_cycles: u64 = 0;
        while !cpu.state.halted && !cpu.state.stopped && cpu.state.instruction_pc != program.exit_pc
        {
            let step_clocks = cpu.step_instruction(bus);
            if step_clocks == 0 {
                break;
            }
            elapsed_cycles = elapsed_cycles.wrapping_add(step_clocks as u64);
            if elapsed_cycles >= max_cycles {
                has_timed_out = true;
                break;
            }
        }

        std::hint::black_box(&mut *cpu);
    }

    let elapsed = start.elapsed();
    let elapsed_ns = elapsed.as_nanos() as f64;

    // Compiler Optimization Defense: Pass CPU reference and computed CRC through black_box
    let mut crc: u64 = 0;
    for &d in cpu.state.d_regs() {
        crc ^= d as u64;
    }
    for &a in cpu.state.a_regs() {
        crc ^= a as u64;
    }
    crc ^= (cpu.state.sr as u64) << 16;
    crc ^= cpu.state.pc as u64;
    std::hint::black_box(crc);

    (elapsed_ns, has_timed_out)
}
