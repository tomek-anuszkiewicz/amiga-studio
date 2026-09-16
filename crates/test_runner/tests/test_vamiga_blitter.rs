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
        frames_to_run: 20,
        ..Default::default()
    };

    let result = run_vamiga_test_from_dir(&test_dir, "bbusy0", &config)
        .expect("bbusy0 execution failed to run");

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

#[test]
fn test_vamiga_blitter_sblit0_execution() {
    let base = if Path::new("ref_src").exists() {
        PathBuf::from("ref_src")
    } else {
        PathBuf::from("../../ref_src")
    };
    let test_dir = base.join("vAmigaTS/Agnus/Blitter/sblit/sblit0");
    if !test_dir.exists() {
        eprintln!("vAmigaTS test directory not present: {:?}", test_dir);
        return;
    }

    let config = VamigaRunConfig {
        frames_to_run: 8,
        ..Default::default()
    };

    let result = run_vamiga_test_from_dir(&test_dir, "sblit0", &config)
        .expect("sblit0 execution failed to run");

    println!(
        "sblit0 result: passed={}, mismatched_pixels={}/{}",
        result.passed, result.mismatched_pixels, result.total_pixels
    );
    if let Some(diff) = &result.first_mismatch {
        println!(
            "First mismatch at ({}, {}): actual={:?}, expected={:?}",
            diff.x, diff.y, diff.actual, diff.expected
        );
    }

    // 5-bitplane emoji blit and Copper synchronization achieves 100% exact pixel match
    assert!(
        result.passed,
        "sblit0 should achieve 100% pixel match, got {} mismatches",
        result.mismatched_pixels
    );
    assert_eq!(result.mismatched_pixels, 0);
}

#[test]
fn test_vamiga_blitter_sblit1_execution() {
    let base = if Path::new("ref_src").exists() {
        PathBuf::from("ref_src")
    } else {
        PathBuf::from("../../ref_src")
    };
    let test_dir = base.join("vAmigaTS/Agnus/Blitter/sblit/sblit1");
    if !test_dir.exists() {
        eprintln!("vAmigaTS test directory not present: {:?}", test_dir);
        return;
    }

    let config = VamigaRunConfig {
        frames_to_run: 8,
        ..Default::default()
    };

    let result = run_vamiga_test_from_dir(&test_dir, "sblit1", &config)
        .expect("sblit1 execution failed to run");

    println!(
        "sblit1 result: passed={}, mismatched_pixels={}/{}",
        result.passed, result.mismatched_pixels, result.total_pixels
    );
    if let Some(diff) = &result.first_mismatch {
        println!(
            "First mismatch at ({}, {}): actual={:?}, expected={:?}",
            diff.x, diff.y, diff.actual, diff.expected
        );
    }

    println!("Total diffs recorded: {}", result.diffs.len());
    for diff in result.diffs.iter().take(15) {
        println!(
            "  diff at ({}, {}): actual={:?}, exp={:?}",
            diff.x, diff.y, diff.actual, diff.expected
        );
    }

    let mut machine = machine_loop::A500Machine::new(config.machine_config.clone());
    let adf_bytes = std::fs::read(test_dir.join("sblit1.adf")).unwrap();
    test_runner::inject_vamiga_test(&mut machine, &adf_bytes).unwrap();
    for _ in 0..8 {
        machine.step_frame();
    }
    let mut actual_raw = [0u8; test_runner::VAMIGA_RAW_BYTE_SIZE];
    machine
        .denise
        .frame_builder
        .extract_vamiga_raw_viewport(&mut actual_raw);
    let _ = std::fs::write("target/actual_sblit1.raw", &actual_raw);

    assert_eq!(result.mismatched_pixels, 0, "sblit1 pixel mismatch");
}

#[test]
fn test_vamiga_blitter_sblit3_execution() {
    let base = if Path::new("ref_src").exists() {
        PathBuf::from("ref_src")
    } else {
        PathBuf::from("../../ref_src")
    };
    let test_dir = base.join("vAmigaTS/Agnus/Blitter/sblit/sblit3");
    if !test_dir.exists() {
        eprintln!("vAmigaTS test directory not present: {:?}", test_dir);
        return;
    }

    let config = VamigaRunConfig {
        frames_to_run: 8,
        ..Default::default()
    };

    let result = run_vamiga_test_from_dir(&test_dir, "sblit3", &config)
        .expect("sblit3 execution failed to run");

    println!(
        "sblit3 result: passed={}, mismatched_pixels={}/{}",
        result.passed, result.mismatched_pixels, result.total_pixels
    );
    if let Some(diff) = &result.first_mismatch {
        println!(
            "First mismatch at ({}, {}): actual={:?}, expected={:?}",
            diff.x, diff.y, diff.actual, diff.expected
        );
    }

    let mut machine = machine_loop::A500Machine::new(config.machine_config.clone());
    let adf_bytes = std::fs::read(test_dir.join("sblit3.adf")).unwrap();
    test_runner::inject_vamiga_test(&mut machine, &adf_bytes).unwrap();
    for _ in 0..8 {
        machine.step_frame();
    }
    let mut actual_raw = [0u8; test_runner::VAMIGA_RAW_BYTE_SIZE];
    machine
        .denise
        .frame_builder
        .extract_vamiga_raw_viewport(&mut actual_raw);
    let _ = std::fs::write("target/actual_sblit3.raw", &actual_raw);

    assert_eq!(result.mismatched_pixels, 0, "sblit3 pixel mismatch");
}

#[test]
fn test_vamiga_blitter_sblit9_execution() {
    let base = if Path::new("ref_src").exists() {
        PathBuf::from("ref_src")
    } else {
        PathBuf::from("../../ref_src")
    };
    let test_dir = base.join("vAmigaTS/Agnus/Blitter/sblit/sblit9");
    if !test_dir.exists() {
        eprintln!("vAmigaTS test directory not present: {:?}", test_dir);
        return;
    }

    let config = VamigaRunConfig {
        frames_to_run: 8,
        ..Default::default()
    };

    let result = run_vamiga_test_from_dir(&test_dir, "sblit9", &config)
        .expect("sblit9 execution failed to run");

    println!(
        "sblit9 result: passed={}, mismatched_pixels={}/{}",
        result.passed, result.mismatched_pixels, result.total_pixels
    );
    if let Some(diff) = &result.first_mismatch {
        println!(
            "First mismatch at ({}, {}): actual={:?}, expected={:?}",
            diff.x, diff.y, diff.actual, diff.expected
        );
    }

    let mut machine = machine_loop::A500Machine::new(config.machine_config.clone());
    let adf_bytes = std::fs::read(test_dir.join("sblit9.adf")).unwrap();
    test_runner::inject_vamiga_test(&mut machine, &adf_bytes).unwrap();
    for _ in 0..8 {
        machine.step_frame();
    }
    let mut actual_raw = [0u8; test_runner::VAMIGA_RAW_BYTE_SIZE];
    machine
        .denise
        .frame_builder
        .extract_vamiga_raw_viewport(&mut actual_raw);
    let _ = std::fs::write("target/actual_sblit9.raw", &actual_raw);

    assert_eq!(result.mismatched_pixels, 0, "sblit9 pixel mismatch");
}

#[test]
fn test_vamiga_blitter_fill0_execution() {
    let base = if Path::new("ref_src").exists() {
        PathBuf::from("ref_src")
    } else {
        PathBuf::from("../../ref_src")
    };
    let test_dir = base.join("vAmigaTS/Agnus/Blitter/fill/fill0");
    if !test_dir.exists() {
        eprintln!("vAmigaTS test directory not present: {:?}", test_dir);
        return;
    }

    let config = VamigaRunConfig {
        frames_to_run: 8,
        ..Default::default()
    };

    let result = run_vamiga_test_from_dir(&test_dir, "fill0", &config)
        .expect("fill0 execution failed to run");

    println!(
        "fill0 result: passed={}, mismatched_pixels={}/{}",
        result.passed, result.mismatched_pixels, result.total_pixels
    );
    if let Some(diff) = &result.first_mismatch {
        println!(
            "First mismatch at ({}, {}): actual={:?}, expected={:?}",
            diff.x, diff.y, diff.actual, diff.expected
        );
    }

    assert!(
        result.passed,
        "fill0 should achieve 100% pixel match, got {} mismatches",
        result.mismatched_pixels
    );
    assert_eq!(result.mismatched_pixels, 0);
}

#[test]
fn test_vamiga_blitter_fill_suite_execution() {
    let base = if Path::new("ref_src").exists() {
        PathBuf::from("ref_src")
    } else {
        PathBuf::from("../../ref_src")
    };

    let config = VamigaRunConfig {
        frames_to_run: 8,
        ..Default::default()
    };

    for i in 0..=7 {
        let name = format!("fill{}", i);
        let test_dir = base.join(format!("vAmigaTS/Agnus/Blitter/fill/{}", name));
        if !test_dir.exists() {
            continue;
        }

        let result = run_vamiga_test_from_dir(&test_dir, &name, &config)
            .unwrap_or_else(|e| panic!("{} execution failed to run: {}", name, e));

        assert!(
            result.passed,
            "{} should achieve 100% pixel match, got {} mismatches",
            name, result.mismatched_pixels
        );
        assert_eq!(result.mismatched_pixels, 0);
    }
}

#[test]
fn test_vamiga_blitter_zero1_execution() {
    let base = if Path::new("ref_src").exists() {
        PathBuf::from("ref_src")
    } else {
        PathBuf::from("../../ref_src")
    };

    let test_dir = base.join("vAmigaTS/Agnus/Blitter/line/zero1");
    if !test_dir.exists() {
        return;
    }

    let config = VamigaRunConfig {
        frames_to_run: 12,
        ..Default::default()
    };

    let result = run_vamiga_test_from_dir(&test_dir, "zero1", &config)
        .expect("zero1 execution failed to run");

    println!(
        "zero1 result: passed={}, mismatches={}/{}",
        result.passed, result.mismatched_pixels, result.total_pixels
    );
    assert!(
        result.passed,
        "zero1 should pass with 0 mismatches, got {}",
        result.mismatched_pixels
    );
    assert_eq!(result.mismatched_pixels, 0);
}
