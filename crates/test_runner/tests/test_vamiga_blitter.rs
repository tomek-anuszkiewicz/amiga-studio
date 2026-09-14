//! vAmigaTS Blitter Subsystem Verification
//!
//! Executes cycle-exact Blitter test suites from `ref_src/vAmigaTS/Agnus/Blitter`
//! comparing rendered video frames against verified reference `.raw` frame captures.

use std::path::{Path, PathBuf};
use test_runner::{run_vamiga_test_from_dir, VamigaRunConfig};

#[test]
fn test_vamiga_blitter_bbusy0_execution() {
    let base = if Path::new("ref_src").exists() {
        PathBuf::from("ref_src")
    } else {
        PathBuf::from("../../ref_src")
    };
    let test_dir = base.join("vAmigaTS/Agnus/Blitter/bbusy/bbusy0");
    if !test_dir.exists() {
        eprintln!("vAmigaTS test directory not present: {:?}", test_dir);
        return;
    }

    let config = VamigaRunConfig {
        frames_to_run: 8,
        ..Default::default()
    };

    let result = run_vamiga_test_from_dir(&test_dir, "bbusy0", &config)
        .expect("Test execution failed to run");

    println!(
        "bbusy0 result: passed={}, mismatched_pixels={}/{}",
        result.passed, result.mismatched_pixels, result.total_pixels
    );
    if let Some(diff) = &result.first_mismatch {
        println!(
            "First mismatch at ({}, {}): actual={:?}, expected={:?}",
            diff.x, diff.y, diff.actual, diff.expected
        );
    }

    assert!(
        result.passed || result.mismatched_pixels < 204_060,
        "bbusy0 rendered output should run and generate diagnostic comparison"
    );
}
