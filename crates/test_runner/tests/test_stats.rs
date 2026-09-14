//! Unit tests for M68000 Benchmark Statistical Analysis & Tukey Outlier Filtering
//!
//! Validates percentile calculation, Tukey's fences outlier rejection,
//! jitter variance metrics, and nominal instruction count invariance.

use test_runner::benchmark::stats::analyze_pass_samples;

#[test]
fn test_stats_empty_samples() {
    let metrics = analyze_pass_samples(&[], 1000, 4000);

    assert_eq!(metrics.raw_samples_count, 0);
    assert_eq!(metrics.filtered_samples_count, 0);
    assert_eq!(metrics.total_guest_instructions, 0);
    assert_eq!(metrics.total_guest_cck, 0);
    assert_eq!(metrics.host_duration_median_ms, 0.0);
    assert_eq!(metrics.host_jitter_pct, 0.0);
    assert_eq!(metrics.confidence_level, "none");
    assert!(!metrics.environment_noisy);
}

#[test]
fn test_stats_single_sample() {
    // 1 pass of 1,000,000 ns (1.0 ms), 100,000 instructions, 400,000 CCK
    let samples = [1_000_000.0];
    let metrics = analyze_pass_samples(&samples, 100_000, 400_000);

    assert_eq!(metrics.raw_samples_count, 1);
    assert_eq!(metrics.filtered_samples_count, 1);
    assert_eq!(metrics.total_guest_instructions, 100_000);
    assert_eq!(metrics.total_guest_cck, 400_000);
    assert!((metrics.host_duration_median_ms - 1.0).abs() < 1e-6);
    assert!((metrics.host_duration_min_ms - 1.0).abs() < 1e-6);
    assert!((metrics.host_duration_max_ms - 1.0).abs() < 1e-6);
    assert_eq!(metrics.host_jitter_pct, 0.0);
    assert_eq!(metrics.confidence_level, "high");

    // host_ns_per_instruction: 1,000,000 ns / 100,000 ops = 10.0 ns/op
    assert!((metrics.host_ns_per_instruction - 10.0).abs() < 1e-6);
    // host_ns_per_guest_cck: 1,000,000 ns / 400,000 CCK = 2.5 ns/CCK
    assert!((metrics.host_ns_per_guest_cck - 2.5).abs() < 1e-6);
    // host_mips: 100,000 ops * 1,000 / 1,000,000 ns = 100.0 MIPS
    assert!((metrics.host_mips - 100.0).abs() < 1e-6);
}

#[test]
fn test_stats_median_odd_and_even() {
    // Odd number of samples (3 samples): median is the middle element
    let samples_odd = [10_000.0, 30_000.0, 20_000.0]; // sorted: [10000, 20000, 30000]
    let metrics_odd = analyze_pass_samples(&samples_odd, 100, 400);
    assert!((metrics_odd.host_duration_median_ms - 0.020).abs() < 1e-6);

    // Even number of samples (4 samples): median is interpolated halfway
    let samples_even = [10_000.0, 20_000.0, 30_000.0, 40_000.0];
    let metrics_even = analyze_pass_samples(&samples_even, 100, 400);
    assert!((metrics_even.host_duration_median_ms - 0.025).abs() < 1e-6);
}

#[test]
fn test_stats_tukey_outlier_rejection() {
    // 8 normal passes clustered around 1,000,000 ns, plus 2 severe OS context switch outliers
    let samples = [
        1_000_000.0,
        1_005_000.0,
        995_000.0,
        1_002_000.0,
        998_000.0,
        1_001_000.0,
        1_003_000.0,
        997_000.0,
        15_000_000.0, // Outlier 1
        20_000_000.0, // Outlier 2
    ];

    let instructions_per_pass = 100_000u64;
    let cck_per_pass = 400_000u64;
    let metrics = analyze_pass_samples(&samples, instructions_per_pass, cck_per_pass);

    // Verify outlier rejection
    assert_eq!(metrics.raw_samples_count, 10);
    assert_eq!(
        metrics.filtered_samples_count, 8,
        "Tukey's fences must discard the two extreme outliers"
    );

    // Max duration should be around 1.005 ms, NOT 20.0 ms
    assert!(metrics.host_duration_max_ms < 2.0);

    // CRITICAL: Nominal total_guest_instructions must be based on raw count (10 passes),
    // guaranteeing determinism regardless of host scheduling noise!
    assert_eq!(
        metrics.total_guest_instructions,
        instructions_per_pass * 10,
        "Total guest instructions must remain strictly nominal across runs"
    );
    assert_eq!(metrics.total_guest_cck, cck_per_pass * 10);
}

#[test]
fn test_stats_jitter_and_confidence_tiers() {
    // 1. High confidence (jitter <= 2.0%)
    let stable_samples = [1_000_000.0, 1_002_000.0, 998_000.0, 1_001_000.0];
    let metrics_stable = analyze_pass_samples(&stable_samples, 1000, 4000);
    assert!(metrics_stable.host_jitter_pct <= 2.0);
    assert_eq!(metrics_stable.confidence_level, "high");
    assert!(!metrics_stable.environment_noisy);

    // 2. Medium confidence (2.0% < jitter <= 5.0%)
    let medium_samples = [1_000_000.0, 1_050_000.0, 950_000.0, 1_020_000.0];
    let metrics_medium = analyze_pass_samples(&medium_samples, 1000, 4000);
    assert!(metrics_medium.host_jitter_pct > 3.0);
    assert!(metrics_medium.host_jitter_pct <= 5.0);
    assert_eq!(metrics_medium.confidence_level, "medium");
    // environment_noisy is true when jitter > 3.0%
    assert!(metrics_medium.environment_noisy);

    // 3. Low confidence (jitter > 5.0%)
    let noisy_samples = [1_000_000.0, 1_150_000.0, 850_000.0, 1_050_000.0];
    let metrics_noisy = analyze_pass_samples(&noisy_samples, 1000, 4000);
    assert!(metrics_noisy.host_jitter_pct > 5.0);
    assert_eq!(metrics_noisy.confidence_level, "low");
    assert!(metrics_noisy.environment_noisy);
}
