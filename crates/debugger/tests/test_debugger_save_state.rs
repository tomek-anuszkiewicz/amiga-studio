#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Unit tests for DebuggerSession save state and quicksave features

use config::{A500Config, A500Preset, VideoStandard};
use debugger::launch::{mount, ADF_BYTES};
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
fn debugger_json_restore_returns_swapped_disks_to_their_saved_drives() {
    let mut session = DebuggerSession::new();
    session
        .machine
        .floppy
        .set_connections([true, true, false, false])
        .unwrap();
    let first = vec![0x31; ADF_BYTES];
    let second = vec![0x72; ADF_BYTES];
    mount(&mut session, 0, Some(&first)).unwrap();
    mount(&mut session, 1, Some(&second)).unwrap();
    session.machine.step_cycles(120);
    let saved = session.save_state();
    assert_ne!(
        saved.floppy.drives[0].tracks(),
        saved.floppy.drives[1].tracks()
    );
    let json = session.save_state_to_json().unwrap();
    let saved_sources = session.file_sources().clone();

    mount(&mut session, 0, Some(&second)).unwrap();
    mount(&mut session, 1, Some(&first)).unwrap();
    session.machine.step_cycles(40);
    assert_eq!(session.file_sources().disks[0], saved_sources.disks[1]);
    assert_eq!(session.file_sources().disks[1], saved_sources.disks[0]);

    let swapped_sources = session.file_sources().clone();
    session.load_state_from_json(&json).unwrap();
    assert_eq!(session.save_state(), saved);
    assert_eq!(session.file_sources(), &saved_sources);
    assert_eq!(session.debugger.current_cck, 120);
    assert!(!session.is_running);

    let mut reopened = DebuggerSession::new();
    reopened
        .load_state_from_json_with_sources(&json, swapped_sources)
        .unwrap();
    assert_eq!(reopened.save_state(), saved);
    assert_eq!(reopened.file_sources(), &saved_sources);
}

#[test]
fn debugger_json_restore_preserves_the_session_when_a_disk_has_no_match() {
    let mut session = DebuggerSession::new();
    session
        .machine
        .floppy
        .set_connections([true, true, false, true])
        .unwrap();
    let first = vec![0x31; ADF_BYTES];
    let second = vec![0x72; ADF_BYTES];
    mount(&mut session, 0, Some(&first)).unwrap();
    mount(&mut session, 1, Some(&second)).unwrap();
    let json = session.save_state_to_json().unwrap();

    mount(&mut session, 0, None).unwrap();
    let wrong_disk = vec![0x19; ADF_BYTES];
    mount(&mut session, 1, Some(&wrong_disk)).unwrap();
    mount(&mut session, 3, Some(&first)).unwrap();
    session.machine.step_cycles(40);
    session.is_running = true;
    let previous = session.save_state();
    let previous_sources = session.file_sources().clone();

    let error = session.load_state_from_json(&json).unwrap_err();
    assert!(matches!(error, SaveStateError::MediaUnavailable(message)
        if message.contains("DF1 content does not match")));
    assert_eq!(session.save_state(), previous);
    assert_eq!(session.file_sources(), &previous_sources);
    assert!(session.is_running);
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
fn file_export_requires_the_current_rom_source_and_in_memory_quicksave_keeps_patches() {
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
    session.quick_save();
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
    session.quick_load().unwrap();
    assert_eq!(session.machine.save_state(), patched);
}
