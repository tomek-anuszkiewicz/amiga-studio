#![allow(clippy::unwrap_used)]

use gui::host_video::{self, FrameInfo};

#[path = "../src/game_fps/progress.rs"]
mod progress;
#[path = "../src/game_fps/submissions.rs"]
mod submissions;

use progress::{validate_session, Boot, Progress};
use submissions::Submissions;

const GENERATION: u64 = 8;

fn image(field_number: u64) -> FrameInfo {
    FrameInfo {
        width: 320,
        height: 256,
        row_stride: 320,
        field_number,
        publication: field_number,
        cck: field_number * 70_000,
        generation: GENERATION,
        interlaced: false,
        field_odd: false,
        complete: true,
    }
}

fn progress(field_number: u64, fields: u32, work_loops: u32) -> Progress {
    Progress {
        fields,
        work_loops,
        fault: 0,
        field_number,
        cck: image(field_number).cck,
    }
}

#[test]
fn waits_for_advancing_scene_progress_and_warms_up_only_with_images_after_confirmation() {
    let mut boot = Boot::default();
    // Acknowledgement permits observation, but firmware fields and counter changes
    // without the scene signature do not arm the drawing counter.
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

    let mut counter = Submissions::new(GENERATION, confirmation.field_number);
    assert_eq!(counter.observe(true, Some(image(43)), 0.0), None);
    assert_eq!(counter.observe(true, Some(image(44)), 0.0), None);
    for field in 45..76 {
        assert_eq!(counter.observe(true, Some(image(field)), 1.0), None);
    }
    assert_eq!(counter.observe(true, Some(image(77)), 2.0), Some(10.0));
    assert_eq!(counter.observe(true, Some(image(77)), 2.5), Some(9.5));
    assert_eq!(counter.observe(true, Some(image(78)), 3.0), Some(9.0));
    assert_eq!(counter.observe(true, Some(image(79)), 12.0), Some(0.0));
    assert_eq!(
        serde_json::to_value(counter.sample).unwrap()["submissions"],
        1
    );
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
fn counts_two_distinct_completed_images_on_retained_game_passes() {
    let mut counter = Submissions::new(GENERATION, 0);
    for field in 1..32 {
        assert_eq!(counter.observe(true, Some(image(field)), 0.0), None);
    }
    // A texture update in Library supplies no draw, leaving warmup incomplete.
    assert_eq!(counter.observe(true, None, 0.0), None);
    assert_eq!(counter.observe(true, Some(image(32)), 0.0), Some(10.0));

    assert_eq!(counter.observe(true, Some(image(33)), 1.0), Some(9.0));
    assert_eq!(counter.observe(true, Some(image(33)), 2.0), Some(8.0));
    assert_eq!(counter.observe(false, Some(image(34)), 3.0), None);
    let partial = FrameInfo {
        complete: false,
        ..image(35)
    };
    assert_eq!(counter.observe(true, Some(partial), 4.0), Some(6.0));
    let stale = FrameInfo {
        generation: GENERATION - 1,
        ..image(36)
    };
    assert_eq!(counter.observe(true, Some(stale), 5.0), Some(5.0));
    assert_eq!(counter.observe(true, Some(image(32)), 6.0), Some(4.0));
    assert_eq!(counter.observe(true, Some(image(41)), 7.0), Some(3.0));

    // An empty wakeup after a stall closes the original interval.
    assert_eq!(counter.observe(true, None, 10.5), Some(0.0));
    assert_eq!(counter.observe(true, Some(image(42)), 12.0), None);
    let sample = serde_json::to_value(counter.sample).unwrap();
    assert_eq!(sample["submissions"], 2);
    assert_eq!(sample["seconds"], 10.0);
    assert_eq!(sample["deadline_delay_seconds"], 0.5);
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
