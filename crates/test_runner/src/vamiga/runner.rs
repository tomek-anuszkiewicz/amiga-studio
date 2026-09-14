//! vAmigaTS Test Execution Runner
//!
//! Autonomous test harness driver that boots test ADFs via direct injection,
//! steps the machine for N frames, extracts the 716 x 285 RGB24 viewport,
//! and compares rendered video against reference `.raw` frame captures.

use std::fs;
use std::path::Path;

use config::A500Config;
use machine_loop::A500Machine;

use super::injector::inject_vamiga_test;
use super::matcher::{compare_raw_frames, VamigaTestResult, VAMIGA_RAW_BYTE_SIZE};

/// Execution parameters for running a vAmigaTS test
#[derive(Debug, Clone)]
pub struct VamigaRunConfig {
    /// Number of vertical video frames to run (default: 8)
    pub frames_to_run: u32,
    /// Hardware machine configuration
    pub machine_config: A500Config,
}

impl Default for VamigaRunConfig {
    fn default() -> Self {
        Self {
            frames_to_run: 8,
            machine_config: A500Config::default(),
        }
    }
}

/// Executes a test directly from memory buffers for ADF and expected RAW reference.
pub fn run_vamiga_test_buffers(
    adf_bytes: &[u8],
    expected_raw_bytes: &[u8],
    config: &VamigaRunConfig,
) -> Result<VamigaTestResult, String> {
    let mut machine = A500Machine::new(config.machine_config.clone());
    inject_vamiga_test(&mut machine, adf_bytes)?;

    // Run for requested number of full frames
    for _ in 0..config.frames_to_run {
        machine.step_frame();
    }

    // Extract rendered 716 x 285 RGB24 viewport
    let mut actual_raw = [0u8; VAMIGA_RAW_BYTE_SIZE];
    machine
        .denise
        .frame_builder
        .extract_vamiga_raw_viewport(&mut actual_raw);

    // Compare with expected reference
    compare_raw_frames(&actual_raw, expected_raw_bytes)
}

/// Executes a test given a test directory path and test name.
pub fn run_vamiga_test_from_dir(
    test_dir: &Path,
    test_name: &str,
    config: &VamigaRunConfig,
) -> Result<VamigaTestResult, String> {
    let adf_path = test_dir.join(format!("{}.adf", test_name));
    if !adf_path.exists() {
        return Err(format!("ADF file not found: {:?}", adf_path));
    }
    let adf_bytes = fs::read(&adf_path)
        .map_err(|e| format!("Failed to read ADF file {:?}: {}", adf_path, e))?;

    // Try finding raw file: <test_name>.raw, or <test_name>_ocs.raw, or <test_name>_ecs.raw
    let raw_path = {
        let direct = test_dir.join(format!("{}.raw", test_name));
        let ocs = test_dir.join(format!("{}_ocs.raw", test_name));
        let ecs = test_dir.join(format!("{}_ecs.raw", test_name));
        if direct.exists() {
            direct
        } else if ocs.exists() {
            ocs
        } else if ecs.exists() {
            ecs
        } else {
            return Err(format!("No reference .raw file found in {:?}", test_dir));
        }
    };

    let raw_bytes = fs::read(&raw_path)
        .map_err(|e| format!("Failed to read reference raw file {:?}: {}", raw_path, e))?;

    run_vamiga_test_buffers(&adf_bytes, &raw_bytes, config)
}
