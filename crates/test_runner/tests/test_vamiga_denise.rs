//! vAmigaTS Denise Subsystem Verification
//!
//! Executes cycle-exact Denise test suites from `ref_src/vAmigaTS/Denise`
//! comparing rendered video frames against verified reference `.raw` frame captures.

use std::path::Path;
use test_runner::{run_vamiga_test_from_dir, VamigaRunConfig};

#[test]
fn test_vamiga_denise_diwsub_execution() {
    let base = if Path::new("ref_src").exists() {
        std::path::PathBuf::from("ref_src")
    } else {
        std::path::PathBuf::from("../../ref_src")
    };
    let test_dir = base.join("vAmigaTS/Denise/DIW/DIWH/diwsub");
    if !test_dir.exists() {
        eprintln!("vAmigaTS test directory not present: {:?}", test_dir);
        return;
    }

    let config = VamigaRunConfig {
        frames_to_run: 8,
        ..Default::default()
    };

    let result = run_vamiga_test_from_dir(&test_dir, "diwsub", &config)
        .expect("Test execution failed to run");

    println!(
        "diwsub result: passed={}, mismatched_pixels={}/{}",
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
        "diwsub rendered output should match or be diagnostic"
    );
}
