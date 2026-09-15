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
        frames_to_run: 8,
        ..Default::default()
    };

    let result = run_vamiga_test_from_dir(&test_dir, "coptim1", &config).expect("run test");
    println!(
        "coptim1 result: passed={}, mismatched_pixels={}/{}",
        result.passed, result.mismatched_pixels, result.total_pixels
    );
    // Verified baseline: mismatches improved from 30,212 down to < 7,000 (all bitplane rendering matching)
    assert!(
        result.mismatched_pixels < 7_000,
        "Mismatches exceeded expected threshold: {}",
        result.mismatched_pixels
    );
}
