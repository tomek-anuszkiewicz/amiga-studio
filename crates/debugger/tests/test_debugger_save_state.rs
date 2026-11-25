#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Unit tests for DebuggerSession save state and quicksave features

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
    session.load_state(&saved_state);

    assert_eq!(session.machine.cck, 400);
    assert_eq!(session.debugger.current_cck, 400);
    assert_eq!(session.machine.cpu.state, saved_state.cpu);
    assert!(!session.is_running);
}

#[test]
fn quicksave_replaces_the_previous_state_and_restores_the_newest_capture() {
    let mut session = DebuggerSession::from_config(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    assert!(session.quicksave().is_none());
    assert!(session.quick_load().is_err());
    assert_eq!(session.machine.cck, 0);
    for _ in 0..250 {
        session.step_cck();
    }
    session.quick_save();
    let first = session.save_state();
    assert_eq!(session.quicksave(), Some(&first));
    for _ in 0..500 {
        session.step_cck();
    }
    session.quick_load().unwrap();
    assert_eq!(session.save_state(), first);
    assert_eq!(session.debugger.current_cck, 250);

    for _ in 0..500 {
        session.step_cck();
    }
    session.quick_save();
    let second = session.save_state();
    assert_eq!(second.cck, 750);
    assert_ne!(first, second);
    session.step_cck();
    session.quick_load().unwrap();
    assert_eq!(session.save_state(), second);
    assert_eq!(session.debugger.current_cck, 750);
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
    let temp_file = temp_dir.join(format!("dbg_session_test_{}.json", std::process::id()));

    std::fs::write(&temp_file, &json_str).expect("Failed to save state to file");

    let mut file_session = DebuggerSession::from_config(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    file_session
        .load_state_from_json(
            &std::fs::read_to_string(&temp_file).expect("Failed to read state file"),
        )
        .expect("Failed to load state from file");
    assert_eq!(file_session.machine.cck, 180);
    assert_eq!(file_session.debugger.current_cck, 180);

    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_debugger_session_save_state_preserves_kickstart_rom() {
    let mut session = DebuggerSession::from_config(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    let dummy_rom = vec![0x33; 256 * 1024];
    session
        .machine
        .physical_memory
        .write_bytes_debug(0xF80000, &dummy_rom)
        .unwrap();

    let saved = session.save_state();
    assert_eq!(saved.physical_memory.kickstart_rom(), dummy_rom);

    let mut new_session = DebuggerSession::from_config(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    new_session.load_state(&saved);
    assert_eq!(
        new_session.machine.physical_memory.kickstart_rom(),
        dummy_rom
    );
}
