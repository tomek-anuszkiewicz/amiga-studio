#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{A500Config, VideoStandard};
use debugger::DebuggerSession;
use gui::host_input;

// Exercise the frontend scheduler with a controlled host clock, without exposing it publicly.
#[path = "../src/playback.rs"]
#[allow(dead_code)]
mod playback;

use playback::Playback;

fn running(standard: VideoStandard) -> Box<DebuggerSession> {
    let mut session = Box::new(DebuggerSession::from_config(A500Config::standard_1mb(
        standard,
    )));
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    session.set_debugger_enabled(false);
    session.toggle_run();
    session
}

// Drive the same subfield deadlines used by the worker, independently of repaint.
fn drive_until(
    playback: &mut Playback,
    session: &mut DebuggerSession,
    clock: &mut f64,
    until: f64,
    slice_cost: f64,
) -> Vec<u64> {
    let mut publications = Vec::new();
    let mut iterations = 0;
    while *clock <= until {
        let before = session.machine.cck;
        if playback.advance(*clock, session, || slice_cost) {
            publications.push(session.machine.denise.frame_builder.frame_number);
        }
        if !session.is_running {
            break;
        }
        *clock += slice_cost;
        let wait = playback.worker_delay(*clock).as_secs_f64();
        *clock += if session.machine.cck == before {
            wait.max(1e-9)
        } else {
            wait
        };
        iterations += 1;
        assert!(iterations < 100_000, "Worker deadline failed to advance");
    }
    publications
}

#[test]
fn accelerated_fields_and_fps_are_independent_of_presentation_rate() {
    for (standard, rate) in [(VideoStandard::Pal, 50), (VideoStandard::Ntsc, 60)] {
        for speed in [2.0, 4.0] {
            let mut session = running(standard);
            let mut playback = Playback::default();
            while playback.speed() < speed {
                playback.faster();
            }
            let mut clock = 0.0;
            let publications = drive_until(&mut playback, &mut session, &mut clock, 1.0, 0.0);
            let fields = session.machine.denise.frame_builder.frame_number;
            let expected_fields = f64::from(rate) * speed;
            assert!(
                (fields as f64 - expected_fields).abs() <= speed,
                "{standard:?} {speed}x: {fields}"
            );
            assert!(
                (publications.len() as i32 - rate).abs() <= 1,
                "{standard:?}: {}",
                publications.len()
            );
            let expected_cck = f64::from(standard.color_clock_hz()) * speed;
            let field_ccks = u64::from(session.machine.agnus.field_ccks());
            assert!((session.machine.cck as f64 - expected_cck).abs() < field_ccks as f64);
            drive_until(&mut playback, &mut session, &mut clock, 1.03, 0.0);
            assert!(
                (playback.fps() - expected_fields).abs() <= speed,
                "{} FPS",
                playback.fps()
            );
        }
    }
}

#[test]
fn overloaded_batches_keep_presentation_phase_and_count_every_field() {
    let mut session = running(VideoStandard::Pal);
    let mut playback = Playback::default();
    playback.faster();
    playback.faster();
    let mut clock = 0.0;
    // Three slices costing 8/3 ms each model the former 8 ms complete-field cost.
    let publications = drive_until(&mut playback, &mut session, &mut clock, 0.99, 0.008 / 3.0);
    let batch_sizes: Vec<_> = publications
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect();
    assert!(
        batch_sizes.iter().all(|count| (2..=3).contains(count)),
        "{batch_sizes:?}"
    );
    assert!(
        batch_sizes.contains(&2) && batch_sizes.contains(&3),
        "{batch_sizes:?}"
    );
    assert!(
        (49..=51).contains(&publications.len()),
        "{} publications",
        publications.len()
    );
    let fields = session.machine.denise.frame_builder.frame_number;
    playback.advance(1.0, &mut session, || 0.008 / 3.0);
    assert!((playback.fps() - fields as f64).abs() < 0.001);
    assert!(
        playback.fps() > 100.0 && playback.fps() < 130.0,
        "{} FPS",
        playback.fps()
    );
}

#[test]
fn long_host_stalls_and_speed_changes_do_not_leave_a_catch_up_backlog() {
    let mut session = running(VideoStandard::Pal);
    let mut playback = Playback::default();
    playback.faster();
    let mut clock = 0.0;
    drive_until(&mut playback, &mut session, &mut clock, 0.03, 0.0);
    let before = session.machine.denise.frame_builder.frame_number;
    clock = 60.0;
    drive_until(&mut playback, &mut session, &mut clock, 60.0, 0.0);
    assert!(session.machine.denise.frame_builder.frame_number - before <= 5);
    let cck = session.machine.cck;
    assert!(!playback.advance(60.0, &mut session, || 0.0));
    assert_eq!(session.machine.cck, cck);
    let publications = drive_until(&mut playback, &mut session, &mut clock, 60.03, 0.0);
    assert!(!publications.is_empty());
    playback.normal_speed();
    let before = session.machine.denise.frame_builder.frame_number;
    drive_until(&mut playback, &mut session, &mut clock, 60.06, 0.0);
    assert!((1..=2).contains(&(session.machine.denise.frame_builder.frame_number - before)));
    session.request_pause();
    drive_until(&mut playback, &mut session, &mut clock, 60.1, 0.0);
    assert!(!session.is_running);
    assert_eq!(playback.fps(), 0.0);
    session.toggle_run();
    clock = 90.0;
    let before = session.machine.denise.frame_builder.frame_number;
    drive_until(&mut playback, &mut session, &mut clock, 90.019, 0.0);
    assert_eq!(
        session.machine.denise.frame_builder.frame_number,
        before + 1
    );
}

#[test]
fn unfinished_slices_keep_the_previous_image_and_stops_end_a_batch() {
    let mut session = running(VideoStandard::Pal);
    session.is_running = false;
    session
        .load_binary(
            0x1000,
            &[0x33, 0xfc, 0, 0, 0, 0xdf, 0xf0, 0x2c, 0x60, 0xf6],
            true,
        )
        .unwrap();
    session.set_debugger_enabled(true);
    session.toggle_run();
    let mut playback = Playback::default();
    playback.faster();
    let span = u64::from(session.machine.agnus.field_ccks());
    assert!(!playback.advance(0.0, &mut session, || 0.0));
    let origin = playback.last_input_cck;
    assert_eq!(session.machine.cck, origin + span / 3);
    // Duration rounds to nanoseconds; wake just after the floating-point deadline.
    let next = playback.worker_delay(0.0).as_secs_f64() + 1e-9;
    assert!(!playback.advance(next, &mut session, || 0.0));
    assert_eq!(session.machine.cck, origin + span * 2 / 3);
    assert_eq!(session.machine.denise.frame_builder.frame_number, 0);
    session.debugger.breakpoints.add_pc_breakpoint(0x1000);
    let now = next + playback.worker_delay(next).as_secs_f64() + 1e-9;
    assert!(playback.advance(now, &mut session, || 0.0));
    assert!(!session.is_running);
    assert_eq!(session.machine.denise.frame_builder.frame_number, 0);
    assert_eq!(playback.fps(), 0.0);
}

#[test]
fn run_uses_host_time_and_measures_fields_independently_of_gui_repaints() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut session = running(standard);
        let mut playback = Playback::default();
        assert!(!playback.advance(0.0, &mut session, || 0.0));
        let first_cck = session.machine.cck;
        assert_eq!(session.machine.denise.frame_builder.frame_number, 0);
        for _ in 0..3 {
            assert!(!playback.advance(0.0, &mut session, || 0.0));
            assert_eq!(session.machine.cck, first_cck);
        }
        let mut clock = playback.worker_delay(0.0).as_secs_f64();
        let publications = drive_until(&mut playback, &mut session, &mut clock, 1.0, 0.0);
        assert!(publications.windows(2).all(|pair| pair[1] == pair[0] + 1));
        let expected_fps = if standard == VideoStandard::Pal {
            50.0
        } else {
            60.0
        };
        assert!((publications.len() as f64 - expected_fps).abs() <= 1.0);
        let expected_cck = f64::from(standard.color_clock_hz());
        assert!(
            (session.machine.cck as f64 - expected_cck).abs()
                < f64::from(session.machine.agnus.field_ccks())
        );
        drive_until(&mut playback, &mut session, &mut clock, 1.03, 0.0);
        assert!(
            (playback.fps() - expected_fps).abs() <= 1.0,
            "{standard:?}: {} FPS",
            playback.fps()
        );
        session.request_pause();
        let paused_cck = session.machine.cck;
        drive_until(&mut playback, &mut session, &mut clock, 1.1, 0.0);
        assert!(!session.is_running);
        assert_eq!(playback.fps(), 0.0);
        assert!(session.machine.cck - paused_cck <= u64::from(session.machine.agnus.field_ccks()));
    }
}

#[test]
fn speed_controls_change_emulated_time_without_skipping_fields() {
    for speed in [0.25, 0.5, 2.0, 4.0] {
        let mut session = running(VideoStandard::Pal);
        let mut playback = Playback::default();
        while playback.speed() > speed {
            playback.slower();
        }
        while playback.speed() < speed {
            playback.faster();
        }
        let mut clock = 0.0;
        drive_until(&mut playback, &mut session, &mut clock, 0.1, 0.0);
        let expected = f64::from(VideoStandard::Pal.color_clock_hz()) * speed * 0.1;
        assert!(
            (session.machine.cck as f64 - expected).abs()
                < f64::from(session.machine.agnus.field_ccks()) / 3.0 + 1.0,
            "speed {speed}"
        );
    }
}

#[test]
fn accelerated_fps_counts_emulated_fields_in_both_views() {
    for view in [gui::ViewMode::ScreenOnly, gui::ViewMode::Developer] {
        let mut session = running(VideoStandard::Pal);
        session.is_running = false;
        session.set_debugger_enabled(view == gui::ViewMode::Developer);
        session.toggle_run();
        let mut playback = Playback::default();
        playback.faster();
        let mut clock = 0.0;
        drive_until(&mut playback, &mut session, &mut clock, 0.04, 0.0);
        let mut publications = drive_until(&mut playback, &mut session, &mut clock, 0.99, 0.0);
        let measured_fields = session.machine.denise.frame_builder.frame_number;
        clock = 1.0;
        if playback.advance(clock, &mut session, || 0.0) {
            publications.push(session.machine.denise.frame_builder.frame_number);
        }
        assert_eq!(playback.fps(), measured_fields as f64, "{view:?}");
        publications.extend(drive_until(
            &mut playback,
            &mut session,
            &mut clock,
            1.04,
            0.0,
        ));
        assert_eq!(publications.len(), 50);
    }
}
