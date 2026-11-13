#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{A500Config, VideoStandard};
use debugger::{session::MAX_FRAME_SLICE_CCK, DebuggerSession};

fn session(standard: VideoStandard) -> DebuggerSession {
    let mut session = DebuggerSession::from_config(A500Config::standard_1mb(standard));
    session.load_binary(0x1000, &[0x60, 0xfe], true).unwrap();
    session
}

#[test]
fn frame_step_and_pause_stop_at_the_first_receiver_completion() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut session = session(standard);
        session.step_cck();
        let mut expected = session.machine.clone();
        let initial_frame = expected.denise.frame_builder.frame_number;
        while expected.denise.frame_builder.frame_number == initial_frame {
            expected.step_cck();
            assert!(expected.cck < 2 * MAX_FRAME_SLICE_CCK as u64);
        }

        session.toggle_run();
        session.run_frame_slice(17);
        assert_eq!(
            session.machine.denise.frame_builder.frame_number,
            initial_frame
        );
        session.toggle_run();
        assert!(session.pause_pending());
        session.run_frame_slice(MAX_FRAME_SLICE_CCK);
        assert!(!session.is_running);
        assert!(!session.pause_pending());
        assert_eq!(session.machine.cck, expected.cck);
        assert_eq!(session.machine.cpu.state, expected.cpu.state);
        assert_eq!(
            session.machine.denise.frame_builder,
            expected.denise.frame_builder
        );

        // Run then Pause between fields must not execute an additional field.
        session.toggle_run();
        session.toggle_run();
        assert!(!session.is_running);
        session.run_frame_slice(MAX_FRAME_SLICE_CCK);
        assert_eq!(session.machine.cck, expected.cck);

        session.step_frame();
        session.run_frame_slice(MAX_FRAME_SLICE_CCK);
        assert!(!session.is_running);
        assert_eq!(
            session.machine.denise.frame_builder.frame_number,
            initial_frame + 2
        );
    }
}

#[test]
fn partial_instruction_resumes_once_and_breakpoint_precedes_frame_completion() {
    let mut session = session(VideoStandard::Pal);
    session
        .load_binary(0x1000, &[0x4e, 0x71, 0x60, 0xfe], true)
        .unwrap();
    session.temporal.set_recording(true);
    session.toggle_run();
    assert_eq!(session.run_frame_slice(1), 0);
    assert_eq!(session.machine.cck, 1);
    assert_eq!(session.debugger.trace.len(), 1);
    session.debugger.breakpoints.add_pc_breakpoint(0x1002);
    session.request_pause();
    session.run_frame_slice(MAX_FRAME_SLICE_CCK);
    assert!(!session.is_running);
    assert_eq!(session.machine.cpu.state.instruction_pc(), 0x1002);
    assert_eq!(session.instructions_executed, 1);
    assert_eq!(session.debugger.trace.len(), 1);
    assert_eq!(session.temporal.len(), 1);
    assert_eq!(session.machine.denise.frame_builder.frame_number, 0);

    session.step_frame();
    let cck = session.machine.cck;
    session.run_frame_slice(MAX_FRAME_SLICE_CCK);
    assert!(!session.is_running);
    assert_eq!(session.machine.cck, cck);
}

#[test]
fn cpu_stop_and_reset_cancel_pending_frame_steps() {
    let mut session = session(VideoStandard::Pal);
    session
        .load_binary(0x1000, &[0x4e, 0x72, 0x27, 0x00], true)
        .unwrap();
    session.step_frame();
    session.run_frame_slice(MAX_FRAME_SLICE_CCK);
    assert!(session.machine.cpu.state.is_stopped());
    assert!(!session.is_running);
    assert!(!session.pause_pending());
    session.step_frame();
    session.reset();
    assert!(!session.is_running);
    assert!(!session.pause_pending());
}

#[test]
fn game_mode_bypasses_inspection_and_breakpoints_without_changing_emulation() {
    let mut session = session(VideoStandard::Pal);
    session.temporal.set_recording(true);
    session.step_instruction();
    let trace = session.debugger.trace.entries();
    let history = session.temporal.len();
    let instructions = session.instructions_executed;
    let debugger_cck = session.debugger.current_cck;
    let diff = session.prev_hex_bytes;
    session.debugger.breakpoints.add_pc_breakpoint(0x1000);
    session.set_debugger_enabled(false);
    let mut expected = session.machine.clone();
    let initial_frame = expected.denise.frame_builder.frame_number;
    while expected.denise.frame_builder.frame_number == initial_frame {
        expected.step_cck();
    }
    session.toggle_run();
    session.run_frame_slice(MAX_FRAME_SLICE_CCK);
    assert!(session.is_running);
    assert_eq!(session.save_state(), expected.save_state());
    assert_eq!(session.debugger.trace.entries(), trace);
    assert_eq!(session.temporal.len(), history);
    assert_eq!(session.instructions_executed, instructions);
    assert_eq!(session.debugger.current_cck, debugger_cck);
    assert_eq!(session.prev_hex_bytes, diff);
    assert!(session.prev_cpu_state.is_none());
    session.request_pause();
    assert!(!session.is_running);
    session.set_debugger_enabled(true);
    session.toggle_run();
    session.run_frame_slice(MAX_FRAME_SLICE_CCK);
    assert!(
        !session.is_running,
        "breakpoints reconnect on returning to Debug"
    );
    assert_eq!(session.debugger.current_cck, session.machine.cck);
    assert!(session.temporal.is_recording());
}

#[test]
fn game_cpu_stop_keeps_chip_clocks_running_until_user_stops_at_a_field_boundary() {
    let mut session = session(VideoStandard::Ntsc);
    session
        .load_binary(0x1000, &[0x4e, 0x72, 0x27, 0x00], true)
        .unwrap();
    session.set_debugger_enabled(false);
    session.toggle_run();
    session.run_frame_slice(30);
    assert!(session.machine.cpu.state.is_stopped());
    assert!(session.is_running);
    session.request_pause();
    session.run_frame_slice(MAX_FRAME_SLICE_CCK);
    assert!(!session.is_running);
    assert_eq!(session.machine.denise.frame_builder.frame_number, 1);
    assert!(session.machine.cpu.state.is_stopped());
    assert!(session.debugger.trace.entries().is_empty());
}
