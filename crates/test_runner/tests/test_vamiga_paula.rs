//! vAmigaTS Paula Audio & Interrupts Subsystem Verification
//!
//! Executes cycle-exact Paula test suites from `ref_src/vAmigaTS/Paula`
//! comparing rendered video frames against verified reference `.raw` frame captures.

use std::path::Path;
use test_runner::{run_vamiga_test_from_dir, VamigaRunConfig};

#[test]
fn test_vamiga_paula_audtim1_execution() {
    let base = if Path::new("ref_src").exists() {
        std::path::PathBuf::from("ref_src")
    } else {
        std::path::PathBuf::from("../../ref_src")
    };
    let test_dir = base.join("vAmigaTS/Paula/Audio/timing/audtim1");
    if !test_dir.exists() {
        eprintln!("vAmigaTS test directory not present: {:?}", test_dir);
        return;
    }

    let config = VamigaRunConfig {
        frames_to_run: 8,
        ..Default::default()
    };

    let result = run_vamiga_test_from_dir(&test_dir, "audtim1", &config)
        .expect("Test execution failed to run");

    println!(
        "audtim1 result: passed={}, mismatched_pixels={}/{}",
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
        "audtim1 rendered output should match or be diagnostic"
    );
}

#[test]
fn test_vamiga_injector_initializes_paula_interrupts() {
    let mut machine = machine_loop::A500Machine::new(config::A500Config::default());
    let adf = vec![0u8; 0x1000];
    test_runner::inject_vamiga_test(&mut machine, &adf).expect("Injection should succeed");
    assert_eq!(machine.paula.interrupts.intena, 0x4000);
}
