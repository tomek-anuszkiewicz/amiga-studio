#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Unit tests for DebuggerSession save state and quick-slot features

use config::{A500Config, A500Preset, VideoStandard};
use debugger::{DebuggerSession, MediaSources, SaveStateError, SourceImage};

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
    assert_eq!(session.quick_slots[0].as_deref().unwrap().cck, 250);

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

#[test]
fn file_export_requires_the_current_rom_source_and_in_memory_slots_keep_patches() {
    let mut session = DebuggerSession::new();
    let mut rom = vec![0x4e; physical_memory::KICKSTART_ROM_SIZE];
    rom[..4].copy_from_slice(&0x70000u32.to_be_bytes());
    rom[4..8].copy_from_slice(&0xfc0100u32.to_be_bytes());
    rom[0x100..0x102].copy_from_slice(&[0x60, 0xfe]);
    session.load_rom(&rom).unwrap();
    session.machine.step_cycles(300);
    let original = session.machine.save_state();
    let json = session.save_state_to_json().unwrap();
    let mut reopened = DebuggerSession::new();
    let before = reopened.machine.save_state();
    let error = reopened.load_state_from_json(&json).unwrap_err();
    assert!(matches!(error, SaveStateError::MediaUnavailable(_)));
    assert_eq!(reopened.machine.save_state(), before);
    reopened
        .load_state_from_json_with_sources(&json, session.file_sources().clone())
        .unwrap();
    assert_eq!(reopened.machine.save_state(), original);

    session
        .machine
        .physical_memory
        .write_byte_debug(config::address_map::KICKSTART_BASE + 0x200, 0x63);
    rom[0x200] = 0x63;
    let patched = session.machine.save_state();
    session.save_quick_slot(1).unwrap();
    let error = session.save_state_to_json().unwrap_err();
    assert!(error
        .to_string()
        .contains("Kickstart ROM source is unavailable"));
    assert_eq!(session.machine.save_state(), patched);

    let sources = MediaSources {
        rom: Some(SourceImage {
            name: Some("Patched Kickstart".into()),
            bytes: rom,
        }),
        ..MediaSources::default()
    };
    let json = session.save_state_to_json_with_sources(&sources).unwrap();
    let previous = reopened.machine.save_state();
    let error = reopened.load_state_from_json(&json).unwrap_err();
    assert!(error.to_string().contains("content does not match"));
    assert_eq!(reopened.machine.save_state(), previous);
    reopened
        .load_state_from_json_with_sources(&json, sources)
        .unwrap();
    assert_eq!(reopened.machine.save_state(), patched);
    session.machine.step_cycles(13);
    session
        .machine
        .physical_memory
        .write_byte_debug(config::address_map::KICKSTART_BASE + 0x200, 0xaa);
    session.load_quick_slot(1).unwrap();
    assert_eq!(session.machine.save_state(), patched);
}
