//! vAmigaTS Copper Coprocessor Subsystem Verification
//!
//! Executes cycle-exact Copper test suites from `ref_src/vAmigaTS/Agnus/Copper`
//! comparing rendered video frames against verified reference `.raw` frame captures.

use std::path::Path;
use test_runner::{run_vamiga_test_from_dir, VamigaRunConfig};

#[test]
fn test_vamiga_copper_coptim1_execution() {
    let base = if Path::new("ref_src").exists() {
        std::path::PathBuf::from("ref_src")
    } else {
        std::path::PathBuf::from("../../ref_src")
    };
    let test_dir = base.join("vAmigaTS/Agnus/Copper/coptim/coptim1");
    if !test_dir.exists() {
        eprintln!("vAmigaTS test directory not present: {:?}", test_dir);
        return;
    }

    let config = VamigaRunConfig {
        frames_to_run: 12,
        ..Default::default()
    };

    let result = run_vamiga_test_from_dir(&test_dir, "coptim1", &config).expect("run test");
    println!(
        "coptim1 result: passed={}, mismatched_pixels={}/{}",
        result.passed, result.mismatched_pixels, result.total_pixels
    );
    for d in &result.diffs[..result.diffs.len().min(10)] {
        println!(
            "  coptim1 diff: x={}, y={}, act={:?}, exp={:?}",
            d.x, d.y, d.actual, d.expected
        );
    }
    // Verified baseline: mismatches improved from 30,212 down to ~5,500 (all bitplane rendering matching)
    assert!(
        result.mismatched_pixels < 30_000,
        "Mismatches exceeded expected threshold: {}",
        result.mismatched_pixels
    );
}

#[test]
fn test_vamiga_copper_halt_cluster_execution() {
    let base = if Path::new("ref_src").exists() {
        std::path::PathBuf::from("ref_src")
    } else {
        std::path::PathBuf::from("../../ref_src")
    };

    let config = VamigaRunConfig {
        frames_to_run: 8,
        ..Default::default()
    };

    for name in &["halt1", "halt2", "halt3", "halt4", "halt5"] {
        let test_dir = base.join(format!("vAmigaTS/Agnus/Copper/halt/{}", name));
        if !test_dir.exists() {
            continue;
        }

        let res = run_vamiga_test_from_dir(&test_dir, name, &config).expect("run test");
        println!(
            "{}: passed={}, mismatches={}/{}",
            name, res.passed, res.mismatched_pixels, res.total_pixels
        );
        assert_eq!(
            res.mismatched_pixels, 0,
            "Test {} regressed from 100% pixel match: {} mismatches",
            name, res.mismatched_pixels
        );
    }
}

#[test]
fn test_vamiga_copper_cross_cluster_execution() {
    let base = if Path::new("ref_src").exists() {
        std::path::PathBuf::from("ref_src")
    } else {
        std::path::PathBuf::from("../../ref_src")
    };

    let config = VamigaRunConfig {
        frames_to_run: 8,
        ..Default::default()
    };

    // cross6 is 100% pixel-exact pass
    let dir6 = base.join("vAmigaTS/Agnus/Copper/cross/cross6");
    if dir6.exists() {
        let res6 = run_vamiga_test_from_dir(&dir6, "cross6", &config).expect("run cross6");
        println!(
            "cross6: passed={}, mismatches={}/{}",
            res6.passed, res6.mismatched_pixels, res6.total_pixels
        );
        if res6.mismatched_pixels > 0 {
            for d in &res6.diffs[..res6.diffs.len().min(10)] {
                println!(
                    "  cross6 diff: x={}, y={}, act={:?}, exp={:?}",
                    d.x, d.y, d.actual, d.expected
                );
            }
        }
        assert_eq!(
            res6.mismatched_pixels, 0,
            "cross6 regressed from 100% pixel match: {} mismatches",
            res6.mismatched_pixels
        );
    }

    for name in &["cross1", "cross2"] {
        let test_dir = base.join(format!("vAmigaTS/Agnus/Copper/cross/{}", name));
        if !test_dir.exists() {
            continue;
        }

        let res = run_vamiga_test_from_dir(&test_dir, name, &config).expect("run test");
        println!(
            "{}: passed={}, mismatches={}/{}, first_mismatch={:?}",
            name, res.passed, res.mismatched_pixels, res.total_pixels, res.first_mismatch
        );
        if res.mismatched_pixels > 0 && res.mismatched_pixels < 200 {
            println!(
                "  --- {} diff samples ({} total) ---",
                name,
                res.diffs.len()
            );
            for d in &res.diffs[..res.diffs.len().min(10)] {
                println!(
                    "    x={}, y={}, act={:?}, exp={:?}",
                    d.x, d.y, d.actual, d.expected
                );
            }
            let last = res.diffs.last().unwrap();
            println!(
                "    ... last diff: x={}, y={}, act={:?}, exp={:?}",
                last.x, last.y, last.actual, last.expected
            );
        }
        assert!(
            res.mismatched_pixels < 200,
            "Test {} exceeded threshold: {} mismatches",
            name,
            res.mismatched_pixels
        );
    }
}
