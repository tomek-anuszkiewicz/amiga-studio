#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::Cell;

use config::{A500Config, VideoStandard};
use debugger::DebuggerSession;

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

#[test]
fn accelerated_fields_and_fps_are_independent_of_presentation_rate() {
    for (standard, rate) in [(VideoStandard::Pal, 50), (VideoStandard::Ntsc, 60)] {
        for speed in [2.0, 4.0] {
            let mut session = running(standard);
            let mut playback = Playback::default();
            while playback.speed() < speed {
                playback.faster();
            }
            assert!(playback.advance(0.0, &mut session, || 0.0));
            let initial_fields = session.machine.denise.frame_builder.frame_number;
            let initial_cck = session.machine.cck;
            let mut presentations = 0;
            // Extra GUI repaints must not produce extra uploads or emulated work.
            for tick in 1..=1000 {
                presentations +=
                    usize::from(playback.advance(f64::from(tick) / 1000.0, &mut session, || 0.0));
            }
            let fields = session.machine.denise.frame_builder.frame_number - initial_fields;
            let expected_fields = f64::from(rate) * speed;
            assert!(
                (fields as f64 - expected_fields).abs() <= speed,
                "{standard:?} {speed}x: {fields}"
            );
            assert!(
                (presentations as i32 - rate).abs() <= 1,
                "{standard:?}: {presentations}"
            );
            assert!(
                (playback.fps() - expected_fields).abs() <= speed,
                "{} FPS",
                playback.fps()
            );
            let emulated_cck = session.machine.cck - initial_cck;
            let expected_cck = f64::from(standard.color_clock_hz()) * speed;
            assert!((emulated_cck as f64 - expected_cck).abs() < initial_cck as f64);
        }
    }
}

#[test]
fn overloaded_batches_keep_presentation_phase_and_count_every_field() {
    let mut session = running(VideoStandard::Pal);
    let mut playback = Playback::default();
    playback.faster();
    playback.faster();
    let mut now = 0.0;
    let mut batch_sizes = Vec::new();
    let mut presentations = 0;
    while now < 1.0 {
        let elapsed = Cell::new(0.0);
        let before = session.machine.denise.frame_builder.frame_number;
        assert!(playback.advance(now, &mut session, || {
            elapsed.set(elapsed.get() + 0.008);
            elapsed.get()
        }));
        let fields = session.machine.denise.frame_builder.frame_number - before;
        batch_sizes.push(fields);
        presentations += 1;
        assert!((2..=3).contains(&fields));
        now += elapsed.get() + 0.001; // Host rendering also consumes the interval.
    }
    assert!(batch_sizes.contains(&2) && batch_sizes.contains(&3));
    assert!(
        (49..=51).contains(&presentations),
        "{presentations} presentations in {now}s"
    );
    let fields = session.machine.denise.frame_builder.frame_number;
    playback.advance(now, &mut session, || 0.03);
    assert!((playback.fps() - fields as f64 / now).abs() < 0.001);
    assert!(playback.fps() > 100.0 && playback.fps() < 130.0);
}

#[test]
fn long_host_stalls_and_speed_changes_do_not_leave_a_catch_up_backlog() {
    let mut session = running(VideoStandard::Pal);
    let mut playback = Playback::default();
    playback.faster();
    playback.advance(0.0, &mut session, || 0.0);
    let before = session.machine.denise.frame_builder.frame_number;
    playback.advance(60.0, &mut session, || 0.0);
    assert!(session.machine.denise.frame_builder.frame_number - before <= 5);
    assert!(!playback.advance(60.0, &mut session, || 0.0));
    assert!(playback.advance(60.02, &mut session, || 0.0));
    playback.normal_speed();
    let before = session.machine.denise.frame_builder.frame_number;
    playback.advance(60.0, &mut session, || 0.0);
    assert_eq!(
        session.machine.denise.frame_builder.frame_number,
        before + 1
    );
    session.request_pause();
    playback.advance(60.0, &mut session, || 0.0);
    assert_eq!(playback.fps(), 0.0);
    session.toggle_run();
    let before = session.machine.denise.frame_builder.frame_number;
    assert!(playback.advance(90.0, &mut session, || 0.0));
    assert_eq!(
        session.machine.denise.frame_builder.frame_number,
        before + 1
    );
}

#[test]
fn unfinished_slices_keep_the_previous_image_and_stops_end_a_batch() {
    let mut session = Box::new(DebuggerSession::from_config(A500Config::standard_1mb(
        VideoStandard::Pal,
    )));
    // MOVE.W #0,$DFF02C; BRA.S $1000 keeps resetting the beam position.
    session
        .load_binary(
            0x1000,
            &[0x33, 0xfc, 0x00, 0x00, 0x00, 0xdf, 0xf0, 0x2c, 0x60, 0xf6],
            true,
        )
        .unwrap();
    session.set_debugger_enabled(true);
    session.toggle_run();
    let mut playback = Playback::default();
    playback.faster();
    for _ in 0..2 {
        assert!(!playback.advance(0.0, &mut session, || 0.0));
        assert_eq!(session.machine.denise.frame_builder.frame_number, 0);
    }
    assert_eq!(
        session.machine.cck,
        2 * debugger::session::MAX_FRAME_SLICE_CCK as u64
    );
    session.debugger.breakpoints.add_pc_breakpoint(0x1000);
    assert!(playback.advance(0.0, &mut session, || 0.0));
    assert!(!session.is_running);
    assert_eq!(session.machine.denise.frame_builder.frame_number, 0);
    assert_eq!(playback.fps(), 0.0);
}
