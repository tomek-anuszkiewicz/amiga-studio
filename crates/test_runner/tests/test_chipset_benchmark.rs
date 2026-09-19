#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Chipset Benchmark & Baseline Regression Sentinel Tests

use std::path::PathBuf;
use test_runner::benchmark::chipset::{
    ChipsetBenchmarkBaseline, ChipsetBenchmarkResult, REGRESSION_THRESHOLD_PERCENT,
};
use test_runner::vamiga::catalog::VamigaCatalog;

fn resolve_vamiga_root() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("ref_src").join("vAmigaTS"));

    if let Some(c) = candidate {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

#[test]
fn test_chipset_benchmark_serialization_roundtrip() {
    let mut methods = std::collections::BTreeMap::new();
    let mut copper_submethods = std::collections::BTreeMap::new();
    copper_submethods.insert("Copper::step_cck".to_string(), 21.6);
    copper_submethods.insert("DmaScheduler::arbitrate".to_string(), 11.2);

    methods.insert(
        "Agnus::step_cck_ram".to_string(),
        test_runner::benchmark::chipset::MethodProfileEntry {
            percent: 32.8,
            module: Some("agnus".to_string()),
            submethods: Some(copper_submethods),
        },
    );
    methods.insert(
        "Cpu::step_cck".to_string(),
        test_runner::benchmark::chipset::MethodProfileEntry {
            percent: 31.4,
            module: Some("cpu".to_string()),
            submethods: None,
        },
    );

    let result = ChipsetBenchmarkResult {
        name: "coptim1".to_string(),
        category: "Copper Coprocessor".to_string(),
        frames: 50,
        cck_count: 3_540_000,
        elapsed_ms: 330.5,
        fps: 151.3,
        cck_mhz: 10.71,
        methods: Some(methods),
    };

    let baseline = ChipsetBenchmarkBaseline {
        version: 1,
        recorded_at: "2026-09-14".to_string(),
        results: vec![result.clone()],
    };

    let json = serde_json::to_string_pretty(&baseline).expect("Serialization failed");
    let deserialized: ChipsetBenchmarkBaseline =
        serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(baseline, deserialized);
    assert_eq!(deserialized.results.len(), 1);
    assert_eq!(deserialized.results[0].name, "coptim1");
    assert!((deserialized.results[0].fps - 151.3).abs() < 1e-4);
    let m = deserialized.results[0]
        .methods
        .as_ref()
        .expect("Methods missing");
    assert_eq!(m.len(), 2);
    assert_eq!(m["Cpu::step_cck"].percent, 31.4);
    assert_eq!(m["Agnus::step_cck_ram"].module.as_deref(), Some("agnus"));
}

#[test]
fn test_chipset_benchmark_regression_evaluation() {
    let golden_fps = 100.0;

    // Faster run (+10%)
    let fast_fps = 110.0;
    let fast_delta = ((fast_fps - golden_fps) / golden_fps) * 100.0;
    assert!(fast_delta >= REGRESSION_THRESHOLD_PERCENT);

    // Minor jitter (-2%) -> still passing
    let slight_drop_fps = 98.0;
    let slight_delta = ((slight_drop_fps - golden_fps) / golden_fps) * 100.0;
    assert!(slight_delta >= REGRESSION_THRESHOLD_PERCENT);

    // Severe regression (-8%) -> flagged
    let slow_fps = 92.0;
    let slow_delta = ((slow_fps - golden_fps) / golden_fps) * 100.0;
    assert!(slow_delta < REGRESSION_THRESHOLD_PERCENT);
}

#[test]
fn test_chipset_benchmark_runner_execution() {
    let root = match resolve_vamiga_root() {
        Some(r) => r,
        None => return,
    };

    let catalog = VamigaCatalog::discover(&root);
    let desc = match catalog.find_test("coptim1") {
        Some(d) => d,
        None => return,
    };

    let adf_bytes = std::fs::read(&desc.adf_path).expect("Failed to read ADF");
    let result = test_runner::benchmark::chipset::benchmark_test_workload(
        "coptim1", "Copper", &adf_bytes, 2,
    )
    .expect("Benchmark workload execution failed");

    assert_eq!(result.name, "coptim1");
    assert_eq!(result.frames, 2);
    assert!(result.cck_count > 140_000);
    assert!(result.elapsed_ms > 0.0);
    assert!(result.fps > 0.0);
}
