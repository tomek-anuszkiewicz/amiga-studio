#![allow(clippy::unwrap_used)]

mod common;
use common::TestContext as _;
use gui::host_commands::{self, HostCommand, HostRequest, HostValue};
use gui::host_owner::MachineOwner;
use gui::{DebuggerSession, ScreenScale};
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
#[path = "../src/game_fps/focus.rs"]
mod focus;
#[path = "../src/game_fps/measurement.rs"]
mod measurement;
#[path = "../src/game_fps/progress.rs"]
mod progress;

use measurement::Sampler;
use progress::{
    validate_game_view, validate_running, validate_session, validate_speed, Boot, Display,
    Progress, SpeedChange, BOOT_SPEED, DOUBLE_SPEED, MEASUREMENT_SPEED,
};

const GENERATION: u64 = 8;

fn running_owner() -> MachineOwner {
    let mut session = Box::new(DebuggerSession::new());
    session.toggle_run();
    MachineOwner::new(session)
}

fn progress(field_number: u64, fields: u32, work_loops: u32) -> Progress {
    Progress {
        fields,
        work_loops,
        fault: 0,
        field_number,
        cck: field_number * 70_000,
    }
}

#[test]
fn waits_for_advancing_scene_progress_and_warms_up_after_two_faster_replies() {
    let mut boot = Boot::default();
    // Acknowledgement permits observation, but firmware fields and counter changes
    // without the scene signature do not confirm readiness.
    validate_session(Some(GENERATION), GENERATION, true).unwrap();
    for firmware_field in 1..=40 {
        let observation = progress(
            firmware_field,
            firmware_field as u32,
            firmware_field as u32 * 11,
        );
        assert_eq!(boot.observe(false, observation).unwrap(), None);
    }
    let first = progress(41, 0, 0);
    assert_eq!(boot.observe(true, first).unwrap(), None);
    assert_eq!(boot.observe(true, progress(42, 0, 0)).unwrap(), None);
    assert_eq!(boot.observe(true, progress(43, 0, 1)).unwrap(), None);
    let confirmation = progress(44, 1, 300);
    assert_eq!(
        boot.observe(true, confirmation).unwrap(),
        Some((first, confirmation))
    );

    let mut owner = running_owner();
    let generation = owner.status().generation;
    assert_eq!(owner.status().speed, 1.0);
    for (i, (previous, target, expected)) in [
        (BOOT_SPEED, DOUBLE_SPEED, 2.0),
        (DOUBLE_SPEED, MEASUREMENT_SPEED, 4.0),
    ]
    .into_iter()
    .enumerate()
    {
        let mut change = SpeedChange::new(previous, target);
        assert!(!change.confirmed(owner.status().speed).unwrap());
        assert!(change.acknowledge(owner.status()).is_err());
        let reply = owner.execute(HostRequest {
            id: i as u64 + 1,
            generation,
            command: HostCommand::Faster,
        });
        assert!(matches!(reply.value, Ok(HostValue::Done)));
        assert_eq!(reply.status.speed, expected);
        assert_eq!(reply.status.generation, generation);
        // A status publication alone may arrive before the command reply.
        assert!(!change.confirmed(reply.status.speed).unwrap());
        change.acknowledge(reply.status).unwrap();
        assert!(change.confirmed(owner.status().speed).unwrap());
        assert!(change.confirmed(previous).is_err());
    }

    let mut sampler = Sampler::new(100);
    assert_eq!(sampler.observe(131, 10.0).unwrap(), None);
    assert_eq!(
        sampler.observe(132, 11.0).unwrap(),
        Some(Duration::from_secs(10))
    );
    assert!(sampler.sample.is_none());
    assert_eq!(sampler.observe(332, 21.0).unwrap(), None);
    let sample = serde_json::to_value(sampler.sample).unwrap();
    assert_eq!(sample["warmup_start_field"], 100);
    assert_eq!(sample["start_field"], 132);
}

#[test]
fn requires_two_matching_clear_observations_and_valid_final_progress() {
    let mut boot = Boot::default();
    let first = progress(40, 20, 100);
    let later = progress(41, 21, 110);
    let fault = Progress {
        fault: 0xbad,
        ..later
    };

    assert_eq!(boot.observe(true, first).unwrap(), None);
    assert_eq!(boot.observe(false, later).unwrap(), None);
    assert_eq!(boot.observe(true, later).unwrap(), None);
    assert_eq!(boot.observe(true, first).unwrap(), None);
    assert!(boot.observe(true, fault).is_err());
    assert!(later.advances(first));
    assert!(!fault.advances(first));
    assert!(!first.advances(later));
    assert!(!Progress {
        cck: first.cck,
        ..later
    }
    .advances(first));
    assert!(!Progress {
        field_number: first.field_number,
        ..later
    }
    .advances(first));
    assert!(!Progress {
        fields: first.fields,
        ..later
    }
    .advances(first));
    assert!(!Progress {
        work_loops: first.work_loops,
        ..later
    }
    .advances(first));
}

#[test]
fn counts_fields_across_sparse_status_reads_and_times_the_final_stall() {
    let mut sampler = Sampler::new(960);
    assert_eq!(sampler.observe(966, 49.0).unwrap(), None);
    assert_eq!(
        sampler.observe(1_000, 51.0).unwrap(),
        Some(Duration::from_secs(10))
    );
    for (field, now, seconds_until_end) in [
        (1_000, 52.0, 9.0),
        (1_030, 54.0, 7.0),
        (1_120, 58.0, 3.0),
        (1_120, 60.5, 0.5),
    ] {
        assert_eq!(
            sampler.observe(field, now).unwrap(),
            Some(Duration::from_secs_f64(seconds_until_end))
        );
        assert!(sampler.sample.is_none());
    }
    assert_eq!(sampler.observe(1_120, 61.5).unwrap(), None);
    let sample = sampler.sample;
    assert_eq!(
        serde_json::to_value(sample).unwrap(),
        serde_json::json!({
            "warmup_start_field": 960,
            "start_field": 1_000,
            "end_field": 1_120,
            "seconds": 10.5,
            "speed": 4.0,
        })
    );
    assert_eq!(sampler.observe(1_150, 65.0).unwrap(), None);
    assert_eq!(sampler.sample, sample);
}

#[test]
fn rejects_a_regressing_owner_field_count_in_warmup_or_timing() {
    for last_field in [120, 180] {
        let mut sampler = Sampler::new(100);
        sampler.observe(last_field, 1.0).unwrap();
        assert_eq!(
            sampler.observe(last_field - 1, 2.0),
            Err("The owner's completed-field counter went backwards".into())
        );
        assert!(sampler.sample.is_none());
    }
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn native_launch_waits_for_continuous_focus_after_a_startup_loss() {
    use progress::StartupFocus;
    use std::time::{Duration, Instant};

    let mut focus = StartupFocus::default();
    let start = Instant::now();
    let after_ms = |ms| start + Duration::from_millis(ms);

    assert!(!focus.observe(false, after_ms(0)));
    assert!(!focus.observe(true, after_ms(20)));
    assert!(!focus.observe(true, after_ms(200)));
    assert!(!focus.observe(false, after_ms(210)));
    assert!(!focus.observe(true, after_ms(220)));
    assert!(!focus.observe(true, after_ms(519)));
    assert!(focus.observe(true, after_ms(520)));
}

#[test]
fn rejects_lost_focus_and_machine_replacement() {
    assert!(validate_session(Some(GENERATION), GENERATION, true).is_ok());
    assert_eq!(
        validate_session(Some(GENERATION), GENERATION, false),
        Err("The Game window lost focus".into())
    );
    assert_eq!(
        validate_session(Some(GENERATION), GENERATION + 1, true),
        Err("Machine generation changed during the benchmark".into())
    );
    assert!(validate_session(Some(GENERATION), GENERATION - 1, true).is_err());
}

#[test]
fn rejects_leaving_game_pausing_or_dropping_from_measurement_speed() {
    let status = running_owner().status();
    assert!(validate_game_view(true, false).is_ok());
    assert!(validate_running(status).is_ok());
    assert!(validate_speed(4.0, 4.0).is_ok());
    assert_eq!(
        validate_game_view(false, false),
        Err("The session left Game view".into())
    );
    assert_eq!(
        validate_game_view(true, true),
        Err("The Game session was paused".into())
    );
    assert!(validate_running(host_commands::HostStatus {
        continuous_run: false,
        ..status
    })
    .is_err());
    assert!(validate_running(host_commands::HostStatus {
        pause_pending: true,
        ..status
    })
    .is_err());
    assert!(validate_running(host_commands::HostStatus {
        halted: true,
        ..status
    })
    .is_err());
    assert_eq!(
        validate_speed(2.0, 4.0),
        Err("Expected Game speed 4×, observed 2×".into())
    );
}

#[test]
fn rejects_viewport_resize_dpi_change_and_game_image_scale_change() {
    let ctx = egui::Context::default();
    let display = |width, dpi, scale| {
        let mut raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 720.0),
            )),
            ..Default::default()
        };
        raw.viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .native_pixels_per_point = Some(dpi);
        let _ = ctx.run_test_ui(raw, |_| {});
        Display::current(&ctx, scale)
    };
    let original = display(1280.0, 1.0, ScreenScale::Fit);
    assert!(original
        .validate(&display(1280.0, 1.0, ScreenScale::Fit))
        .is_ok());
    assert!(original
        .validate(&display(1200.0, 1.0, ScreenScale::Fit))
        .is_err());
    assert!(original
        .validate(&display(1280.0, 1.5, ScreenScale::Fit))
        .is_err());
    assert!(original
        .validate(&display(1280.0, 1.0, ScreenScale::One))
        .is_err());
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn records_transient_focus_loss_between_gui_passes() {
    let ctx = egui::Context::default();
    let mut raw = egui::RawInput::default();
    raw.viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .focused = Some(true);
    focus::watch_focus(&ctx);
    let _ = ctx.run_test_ui(raw.clone(), |_| assert!(focus::focused(&ctx)));
    focus::record_focus_loss(&ctx);
    let _ = ctx.run_test_ui(raw.clone(), |_| assert!(!focus::focused(&ctx)));
    focus::stop_watching_focus(&ctx);
    let _ = ctx.run_test_ui(raw, |_| assert!(focus::focused(&ctx)));
}
