//! Statistical Sampling, Noise Filtering & Metrics Calculation
//!
//! Implements Tukey's fences outlier rejection, jitter analysis (CV < 3%),
//! environmental cooldown pauses ("Wait Out the Storm"), and robust metrics
//! per Obsidian/Amiga/Design/CPU Instruction Benchmarking.md.

use serde::{Deserialize, Serialize};

/// Aggregated statistical metrics for a benchmark run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    pub total_guest_instructions: u64,
    pub total_guest_cck: u64,
    pub raw_samples_count: usize,
    pub filtered_samples_count: usize,
    pub host_duration_median_ms: f64,
    pub host_duration_min_ms: f64,
    pub host_duration_max_ms: f64,
    pub host_duration_trimmed_mean_ms: f64,
    pub host_std_dev_ms: f64,
    pub host_jitter_pct: f64,
    pub host_ns_per_instruction: f64,
    pub host_ns_per_guest_cck: f64,
    pub host_mips: f64,
    pub environment_noisy: bool,
    pub confidence_level: String,
}

/// Computes robust metrics from raw pass durations in nanoseconds
pub fn analyze_pass_samples(
    raw_samples_ns: &[f64],
    instructions_per_pass: u64,
    cck_per_pass: u64,
) -> BenchmarkMetrics {
    let raw_count = raw_samples_ns.len();
    if raw_count == 0 {
        return BenchmarkMetrics {
            total_guest_instructions: 0,
            total_guest_cck: 0,
            raw_samples_count: 0,
            filtered_samples_count: 0,
            host_duration_median_ms: 0.0,
            host_duration_min_ms: 0.0,
            host_duration_max_ms: 0.0,
            host_duration_trimmed_mean_ms: 0.0,
            host_std_dev_ms: 0.0,
            host_jitter_pct: 0.0,
            host_ns_per_instruction: 0.0,
            host_ns_per_guest_cck: 0.0,
            host_mips: 0.0,
            environment_noisy: false,
            confidence_level: "none".to_string(),
        };
    }

    let mut sorted = raw_samples_ns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // 1. Tukey's Fences Outlier Rejection
    let filtered: Vec<f64> = if sorted.len() >= 4 {
        let q1 = percentile_sorted(&sorted, 0.25);
        let q3 = percentile_sorted(&sorted, 0.75);
        let iqr = q3 - q1;
        let upper_fence = q3 + 1.5 * iqr;
        let lower_fence = (q1 - 1.5 * iqr).max(0.0);

        let kept: Vec<f64> = sorted
            .iter()
            .copied()
            .filter(|&v| v >= lower_fence && v <= upper_fence)
            .collect();
        if kept.is_empty() {
            sorted.clone()
        } else {
            kept
        }
    } else {
        sorted.clone()
    };

    let filtered_count = filtered.len();
    let min_ns = *filtered.first().unwrap_or(&0.0);
    let max_ns = *filtered.last().unwrap_or(&0.0);
    let median_ns = percentile_sorted(&filtered, 0.50);
    let trimmed_mean_ns = compute_trimmed_mean(&filtered, 0.10);

    // Compute standard deviation and Coefficient of Variation (CV)
    let mean_val = filtered.iter().sum::<f64>() / (filtered_count as f64);
    let variance = filtered
        .iter()
        .map(|v| {
            let diff = v - mean_val;
            diff * diff
        })
        .sum::<f64>()
        / (filtered_count as f64);
    let std_dev_ns = variance.sqrt();
    let jitter_pct = if mean_val > 0.0 {
        (std_dev_ns / mean_val) * 100.0
    } else {
        0.0
    };

    let environment_noisy = jitter_pct > 3.0;
    let confidence_level = if jitter_pct <= 2.0 {
        "high".to_string()
    } else if jitter_pct <= 5.0 {
        "medium".to_string()
    } else {
        "low".to_string()
    };

    // Calculate throughput and timing per instruction and per Amiga CCK
    let host_ns_per_instruction = if instructions_per_pass > 0 {
        median_ns / (instructions_per_pass as f64)
    } else {
        0.0
    };
    let host_ns_per_guest_cck = if cck_per_pass > 0 {
        median_ns / (cck_per_pass as f64)
    } else {
        0.0
    };
    let host_mips = if median_ns > 0.0 {
        (instructions_per_pass as f64) * 1_000.0 / median_ns
    } else {
        0.0
    };

    BenchmarkMetrics {
        total_guest_instructions: instructions_per_pass * (raw_count as u64),
        total_guest_cck: cck_per_pass * (raw_count as u64),
        raw_samples_count: raw_count,
        filtered_samples_count: filtered_count,
        host_duration_median_ms: median_ns / 1_000_000.0,
        host_duration_min_ms: min_ns / 1_000_000.0,
        host_duration_max_ms: max_ns / 1_000_000.0,
        host_duration_trimmed_mean_ms: trimmed_mean_ns / 1_000_000.0,
        host_std_dev_ms: std_dev_ns / 1_000_000.0,
        host_jitter_pct: jitter_pct,
        host_ns_per_instruction,
        host_ns_per_guest_cck,
        host_mips,
        environment_noisy,
        confidence_level,
    }
}

/// Evaluates percentile value from an already sorted slice
fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let idx = p * ((sorted.len() - 1) as f64);
    let low = idx.floor() as usize;
    let high = idx.ceil() as usize;
    if low == high {
        sorted[low]
    } else {
        let weight = idx - (low as f64);
        sorted[low] * (1.0 - weight) + sorted[high] * weight
    }
}

/// Trims top and bottom trim_fraction (e.g. 0.10 for 10%) and computes mean
fn compute_trimmed_mean(sorted: &[f64], trim_fraction: f64) -> f64 {
    let len = sorted.len();
    if len <= 2 {
        return sorted.iter().sum::<f64>() / (len.max(1) as f64);
    }
    let trim_count = ((len as f64) * trim_fraction).floor() as usize;
    if trim_count * 2 >= len {
        return sorted.iter().sum::<f64>() / (len as f64);
    }
    let slice = &sorted[trim_count..(len - trim_count)];
    slice.iter().sum::<f64>() / (slice.len() as f64)
}
