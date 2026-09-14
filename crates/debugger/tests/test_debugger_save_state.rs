//! Unit tests for DebuggerSession save state and quick-slot features

use config::{A500Config, A500Preset, VideoStandard};
use debugger::DebuggerSession;

#[test]
fn test_debugger_session_save_and_load() {
    let mut session = DebuggerSession::from_config(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));

    // Execute 400 CCKs
    for _ in 0..400 {
        session.step_cck();
    }
    assert_eq!(session.debugger.current_cck, 400);

    // Save state
    let saved_state = session.save_state();
    assert_eq!(saved_state.cck, 400);

    // Step further to 1000 CCKs
    for _ in 0..600 {
        session.step_cck();
    }
    assert_eq!(session.debugger.current_cck, 1000);

    // Load state back
    session
        .load_state(&saved_state)
        .expect("Failed to load save state into DebuggerSession");

    assert_eq!(session.machine.cck, 400);
    assert_eq!(session.debugger.current_cck, 400);
    assert_eq!(session.machine.cpu.state, saved_state.cpu);
    assert!(!session.is_running);
}

#[test]
fn test_debugger_session_quick_slots() {
    let mut session = DebuggerSession::from_config(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));

    // Initially all 5 quick slots are empty
    for i in 1..=5 {
        assert!(!session.has_quick_slot(i));
    }

    // Step 250 CCKs and save to Slot 1
    for _ in 0..250 {
        session.step_cck();
    }
    session
        .save_quick_slot(1)
        .expect("Failed to save to quick slot 1");
    assert!(session.has_quick_slot(1));

    // Step to 750 CCKs and save to Slot 2
    for _ in 0..500 {
        session.step_cck();
    }
    session
        .save_quick_slot(2)
        .expect("Failed to save to quick slot 2");
    assert!(session.has_quick_slot(2));

    // Restore Slot 1
    session
        .load_quick_slot(1)
        .expect("Failed to load quick slot 1");
    assert_eq!(session.machine.cck, 250);
    assert_eq!(session.debugger.current_cck, 250);

    // Restore Slot 2
    session
        .load_quick_slot(2)
        .expect("Failed to load quick slot 2");
    assert_eq!(session.machine.cck, 750);
    assert_eq!(session.debugger.current_cck, 750);

    // Loading empty Slot 3 must fail
    assert!(session.load_quick_slot(3).is_err());

    // Invalid slots (0 or 6) must fail
    assert!(session.save_quick_slot(0).is_err());
    assert!(session.save_quick_slot(6).is_err());
    assert!(session.load_quick_slot(0).is_err());
    assert!(session.load_quick_slot(6).is_err());
}

#[test]
fn test_debugger_session_json_and_file() {
    let mut session = DebuggerSession::from_config(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));

    for _ in 0..180 {
        session.step_cck();
    }

    // JSON roundtrip
    let json_str = session
        .save_state_to_json()
        .expect("Failed to serialize session to JSON");
    assert!(!json_str.is_empty());

    let mut other_session = DebuggerSession::from_config(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    other_session
        .load_state_from_json(&json_str)
        .expect("Failed to load session from JSON");
    assert_eq!(other_session.machine.cck, 180);
    assert_eq!(other_session.debugger.current_cck, 180);

    // File roundtrip
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("dbg_session_test.a500z");

    session
        .save_state_to_file(&temp_file, false)
        .expect("Failed to save state to file");

    let mut file_session = DebuggerSession::from_config(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    file_session
        .load_state_from_file(&temp_file)
        .expect("Failed to load state from file");
    assert_eq!(file_session.machine.cck, 180);
    assert_eq!(file_session.debugger.current_cck, 180);

    let _ = std::fs::remove_file(&temp_file);
}
