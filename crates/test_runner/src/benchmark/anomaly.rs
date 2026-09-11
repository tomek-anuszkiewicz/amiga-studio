//! Anomaly Detection & Regression Diffing Engine
//!
//! Evaluates normalized cycle ratio (R_norm = Host ns / Amiga CCK), detects Type A/B/C anomalies,
//! and compares against historical baseline files per Obsidian/Amiga/Design/CPU Instruction Benchmarking.md.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    /// Type A: Intra-Family Execution Spike (R_norm > 2.5x family baseline)
    HotPathStall,
    /// Type B: Addressing Mode Inefficiency (R_norm > 3.0x register baseline)
    BusContentionOrInliningDefect,
    /// Type C: Excessive Jitter (> 5.0% across passes)
    HostBranchPredictionThrashing,
    /// Execution Timeout: Pass exceeded cycle safety limit (possible infinite loop)
    ExecutionTimeoutOrInfiniteLoop,
}

impl AnomalyType {
    pub const fn description(&self) -> &'static str {
        match self {
            Self::HotPathStall => {
                "Type A: Hot Path Stalling / Cold Dispatch Spike (>2.5x baseline)"
            }
            Self::BusContentionOrInliningDefect => {
                "Type B: Addressing Mode Inefficiency / Memory Inlining Defect (>3.0x reg baseline)"
            }
            Self::HostBranchPredictionThrashing => {
                "Type C: Excessive Jitter / Host Branch Predictor Thrashing (>5.0% CV)"
            }
            Self::ExecutionTimeoutOrInfiniteLoop => {
                "Execution Timeout: Pass exceeded 10x expected cycles (possible infinite loop)"
            }
        }
    }
}

/// Anomaly evaluation verdict for a benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyVerdict {
    pub is_anomaly: bool,
    pub anomaly_types: Vec<AnomalyType>,
    pub delta_vs_baseline_pct: Option<f64>,
    pub notes: Vec<String>,
}

impl Default for AnomalyVerdict {
    fn default() -> Self {
        Self {
            is_anomaly: false,
            anomaly_types: Vec::new(),
            delta_vs_baseline_pct: None,
            notes: Vec::new(),
        }
    }
}

/// Classifies performance anomalies for a given instruction result
pub fn evaluate_anomaly(
    host_ns_per_cck: f64,
    family_baseline_ns_per_cck: Option<f64>,
    register_baseline_ns_per_cck: Option<f64>,
    jitter_pct: f64,
    is_memory_mode: bool,
) -> AnomalyVerdict {
    let mut verdict = AnomalyVerdict::default();

    // 1. Type A: Intra-Family Execution Spike (>2.5x family baseline)
    if let Some(family_base) = family_baseline_ns_per_cck {
        if family_base > 0.0 && host_ns_per_cck > (family_base * 2.5) {
            verdict.is_anomaly = true;
            verdict.anomaly_types.push(AnomalyType::HotPathStall);
            verdict.notes.push(format!(
                "Host ns/CCK ({:.3}) exceeds 2.5x family baseline ({:.3})",
                host_ns_per_cck, family_base
            ));
        }
    }

    // 2. Type B: Addressing Mode Inefficiency (>3.0x register baseline)
    if is_memory_mode {
        if let Some(reg_base) = register_baseline_ns_per_cck {
            if reg_base > 0.0 && host_ns_per_cck > (reg_base * 3.0) {
                verdict.is_anomaly = true;
                verdict
                    .anomaly_types
                    .push(AnomalyType::BusContentionOrInliningDefect);
                verdict.notes.push(format!(
                    "Memory mode host ns/CCK ({:.3}) exceeds 3.0x register baseline ({:.3})",
                    host_ns_per_cck, reg_base
                ));
            }
        }
    }

    // 3. Type C: Excessive Jitter (>5.0% across passes)
    if jitter_pct > 5.0 {
        verdict.is_anomaly = true;
        verdict
            .anomaly_types
            .push(AnomalyType::HostBranchPredictionThrashing);
        verdict.notes.push(format!(
            "Pass jitter ({:.2}%) exceeds 5.0% threshold",
            jitter_pct
        ));
    }

    verdict
}

/// Historical regression status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegressionStatus {
    NormalVariance,
    MinorRegressionWarning,
    SevereRegressionFailure,
    PerformanceImprovement,
}

/// Compares current execution latency with historical baseline
pub fn evaluate_historical_diff(current_ms: f64, baseline_ms: f64) -> (RegressionStatus, f64) {
    if baseline_ms <= 0.0 {
        return (RegressionStatus::NormalVariance, 0.0);
    }
    let delta_pct = ((current_ms - baseline_ms) / baseline_ms) * 100.0;
    let status = if delta_pct <= -3.0 {
        RegressionStatus::PerformanceImprovement
    } else if delta_pct <= 3.0 {
        RegressionStatus::NormalVariance
    } else if delta_pct <= 7.0 {
        RegressionStatus::MinorRegressionWarning
    } else {
        RegressionStatus::SevereRegressionFailure
    };
    (status, delta_pct)
}

/// Compares current execution latency ratio with historical baseline ratio (e.g. NOP-normalized R_nop).
///
/// Because host CPU clock speed differences scale both the target instruction and the NOP baseline
/// equally, the ratio T(Op) / T(NOP) is hardware-invariant across different host processor architectures.
#[inline]
pub fn evaluate_historical_ratio_diff(
    current_ratio: f64,
    baseline_ratio: f64,
) -> (RegressionStatus, f64) {
    evaluate_historical_diff(current_ratio, baseline_ratio)
}
