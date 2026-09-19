#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Unit tests for M68000 Benchmark Anomaly Detection & Regression Diffing
//!
//! Tests Type A, Type B, Type C, and execution timeout classifications,
//! alongside baseline delta percentage and regression status evaluation.

use test_runner::benchmark::anomaly::{
    evaluate_anomaly, evaluate_historical_diff, evaluate_historical_ratio_diff, AnomalyType,
    RegressionStatus,
};

#[test]
fn test_anomaly_normal_execution() {
    let verdict = evaluate_anomaly(
        3.5,       // host_ns_per_cck
        Some(3.2), // family_baseline
        Some(3.0), // register_baseline
        1.5,       // jitter_pct (< 5.0%)
        true,      // is_memory_mode (3.5 <= 3.0 * 3.0)
    );

    assert!(
        !verdict.is_anomaly,
        "Normal execution should not trigger anomaly"
    );
    assert!(verdict.anomaly_types.is_empty());
    assert!(verdict.notes.is_empty());
}

#[test]
fn test_anomaly_type_a_hot_path_stall() {
    // Family baseline: 2.0 ns/CCK. 2.5x threshold = 5.0 ns/CCK.
    let verdict = evaluate_anomaly(
        5.5,       // host_ns_per_cck (> 5.0)
        Some(2.0), // family_baseline
        Some(2.0), // register_baseline
        1.2,       // jitter_pct
        false,     // is_memory_mode
    );

    assert!(verdict.is_anomaly, "Should trigger Type A anomaly");
    assert_eq!(verdict.anomaly_types.len(), 1);
    assert_eq!(verdict.anomaly_types[0], AnomalyType::HotPathStall);
    assert!(verdict.notes[0].contains("exceeds 2.5x family baseline"));
}

#[test]
fn test_anomaly_type_b_memory_inlining_defect() {
    // Register baseline: 2.0 ns/CCK. 3.0x threshold = 6.0 ns/CCK.
    let verdict = evaluate_anomaly(
        6.5,       // host_ns_per_cck (> 6.0)
        Some(6.0), // family_baseline
        Some(2.0), // register_baseline
        1.0,       // jitter_pct
        true,      // is_memory_mode
    );

    assert!(verdict.is_anomaly, "Should trigger Type B anomaly");
    assert_eq!(verdict.anomaly_types.len(), 1);
    assert_eq!(
        verdict.anomaly_types[0],
        AnomalyType::BusContentionOrInliningDefect
    );
    assert!(verdict.notes[0].contains("exceeds 3.0x register baseline"));

    // Verify non-memory mode does NOT trigger Type B even if above 3.0x reg baseline
    let non_mem_verdict = evaluate_anomaly(
        6.5,
        Some(6.0),
        Some(2.0),
        1.0,
        false, // NOT memory mode
    );
    assert!(
        !non_mem_verdict
            .anomaly_types
            .contains(&AnomalyType::BusContentionOrInliningDefect),
        "Register mode should never trigger Type B addressing mode anomaly"
    );
}

#[test]
fn test_anomaly_type_c_excessive_jitter() {
    // Jitter threshold = 5.0%
    let verdict = evaluate_anomaly(
        3.0,
        Some(3.0),
        Some(3.0),
        6.2, // jitter_pct > 5.0%
        false,
    );

    assert!(verdict.is_anomaly, "Should trigger Type C anomaly");
    assert_eq!(verdict.anomaly_types.len(), 1);
    assert_eq!(
        verdict.anomaly_types[0],
        AnomalyType::HostBranchPredictionThrashing
    );
    assert!(verdict.notes[0].contains("exceeds 5.0% threshold"));
}

#[test]
fn test_anomaly_multiple_simultaneous_anomalies() {
    // Both Type A (>2.5x family) and Type C (>5.0% jitter)
    let verdict = evaluate_anomaly(
        10.0, // > 2.5 * 2.0 = 5.0
        Some(2.0),
        Some(2.0),
        8.5, // > 5.0%
        false,
    );

    assert!(verdict.is_anomaly);
    assert_eq!(verdict.anomaly_types.len(), 2);
    assert!(verdict.anomaly_types.contains(&AnomalyType::HotPathStall));
    assert!(verdict
        .anomaly_types
        .contains(&AnomalyType::HostBranchPredictionThrashing));
    assert_eq!(verdict.notes.len(), 2);
}

#[test]
fn test_anomaly_type_descriptions() {
    let types = [
        AnomalyType::HotPathStall,
        AnomalyType::BusContentionOrInliningDefect,
        AnomalyType::HostBranchPredictionThrashing,
        AnomalyType::ExecutionTimeoutOrInfiniteLoop,
    ];

    for t in types {
        let desc = t.description();
        assert!(!desc.is_empty(), "Description must not be empty");
    }
}

#[test]
fn test_evaluate_historical_diff_categories() {
    // 1. Performance improvement: <= -3.0%
    let (status, delta) = evaluate_historical_diff(90.0, 100.0);
    assert_eq!(status, RegressionStatus::PerformanceImprovement);
    assert!((delta - (-10.0)).abs() < 1e-6);

    // 2. Normal variance: -3.0% .. +3.0%
    let (status, delta) = evaluate_historical_diff(102.0, 100.0);
    assert_eq!(status, RegressionStatus::NormalVariance);
    assert!((delta - 2.0).abs() < 1e-6);

    // 3. Minor regression warning: +3.0% .. +7.0%
    let (status, delta) = evaluate_historical_diff(105.5, 100.0);
    assert_eq!(status, RegressionStatus::MinorRegressionWarning);
    assert!((delta - 5.5).abs() < 1e-6);

    // 4. Severe regression failure: > +7.0%
    let (status, delta) = evaluate_historical_diff(115.0, 100.0);
    assert_eq!(status, RegressionStatus::SevereRegressionFailure);
    assert!((delta - 15.0).abs() < 1e-6);

    // 5. Zero or negative baseline handling
    let (status, delta) = evaluate_historical_diff(100.0, 0.0);
    assert_eq!(status, RegressionStatus::NormalVariance);
    assert_eq!(delta, 0.0);
}

#[test]
fn test_evaluate_historical_ratio_diff_categories() {
    // 1. Performance improvement: <= -3.0% (e.g. 1.02x NOP vs 1.15x NOP)
    let (status, delta) = evaluate_historical_ratio_diff(1.02, 1.15);
    assert_eq!(status, RegressionStatus::PerformanceImprovement);
    assert!(delta < -3.0);

    // 2. Normal variance: -3.0% .. +3.0% (e.g. 1.05x NOP vs 1.04x NOP)
    let (status, delta) = evaluate_historical_ratio_diff(1.05, 1.04);
    assert_eq!(status, RegressionStatus::NormalVariance);
    assert!(delta.abs() <= 3.0);

    // 3. Minor regression warning: +3.0% .. +7.0%
    let (status, delta) = evaluate_historical_ratio_diff(1.10, 1.05);
    assert_eq!(status, RegressionStatus::MinorRegressionWarning);
    assert!(delta > 3.0 && delta <= 7.0);

    // 4. Severe regression failure: > +7.0%
    let (status, delta) = evaluate_historical_ratio_diff(1.25, 1.05);
    assert_eq!(status, RegressionStatus::SevereRegressionFailure);
    assert!(delta > 7.0);

    // 5. Zero or negative baseline ratio handling
    let (status, delta) = evaluate_historical_ratio_diff(1.05, 0.0);
    assert_eq!(status, RegressionStatus::NormalVariance);
    assert_eq!(delta, 0.0);
}
