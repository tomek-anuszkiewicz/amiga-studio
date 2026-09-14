//! Machine Loop Profile Stats Unit Tests

use config::A500Config;
use machine_loop::{A500Machine, SubsystemProfileStats};
use std::time::Duration;

#[test]
fn test_profile_stats_calculations() {
    let mut stats = SubsystemProfileStats::new();
    stats.total_duration = Duration::from_millis(500); // 0.5 seconds
    stats.cpu_duration = Duration::from_millis(200);
    stats.agnus_duration = Duration::from_millis(100);
    stats.denise_duration = Duration::from_millis(100);
    stats.paula_duration = Duration::from_millis(50);
    stats.cia_duration = Duration::from_millis(50);
    stats.frames_executed = 50;
    stats.cck_executed = 3_540_000;

    // 50 frames in 0.5s -> 100 FPS
    assert!((stats.fps() - 100.0).abs() < 1e-4);
    // 100 FPS / 50 FPS PAL = 2.0x
    assert!((stats.speedup_factor() - 2.0).abs() < 1e-4);
    // 3.54M CCKs / 0.5s = 7.08 MHz
    assert!((stats.cck_mhz() - 7.08).abs() < 1e-4);
}

#[test]
fn test_step_frame_profiled_execution() {
    let mut machine = A500Machine::new(A500Config::default());
    let mut stats = SubsystemProfileStats::new();

    // Step 2 full frames with profiling enabled
    machine.step_frame_profiled(&mut stats);
    machine.step_frame_profiled(&mut stats);

    assert_eq!(stats.frames_executed, 2);
    assert!(
        stats.cck_executed > 140_000,
        "2 frames should step > 140,000 CCKs"
    );
    assert!(stats.total_duration > Duration::ZERO);
    assert!(stats.cpu_duration > Duration::ZERO);
    assert!(stats.agnus_duration > Duration::ZERO);
    assert!(stats.denise_duration > Duration::ZERO);
    assert!(stats.fps() > 0.0);
}
