#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{A500Config, A500Preset, VideoStandard};
use machine_loop::A500Machine;
use save_state::{SaveState, SaveStateError, SAVE_STATE_VERSION};

#[test]
fn test_save_state_metadata_and_header() {
    let machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Standard1Mb,
        VideoStandard::Pal,
    ));
    let saved = SaveState::capture(&machine);
    let state = &saved.state;

    assert_eq!(saved.header.version, SAVE_STATE_VERSION);
    assert!(!saved.header.screenshot_png.is_empty());
    // PNG magic header: \x89PNG\r\n\x1a\n
    assert_eq!(&saved.header.screenshot_png[0..4], b"\x89PNG");
    assert_eq!(state.config.chip_ram(), config::ChipRamSize::Kb512);
    assert_eq!(state.config.slow_ram(), config::SlowRamSize::Kb512);
    assert_eq!(state.config.fast_ram(), config::FastRamSize::None);
    assert_eq!(state.cck, 0);
}
#[test]
fn test_save_state_json_roundtrip() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    machine.step_cycles(200);

    let state = SaveState::capture(&machine);
    let json_str = state.to_json().expect("JSON serialization failed");
    let restored = SaveState::from_json(&json_str).expect("JSON deserialization failed");

    assert_eq!(state.header, restored.header);
    assert_eq!(state.state.cck, restored.state.cck);
    assert_eq!(state.state.cpu, restored.state.cpu);
    assert_eq!(state.state.agnus, restored.state.agnus);
    assert_eq!(state.state.denise, restored.state.denise);
    assert_eq!(state.state.cia_a, restored.state.cia_a);
    assert_eq!(state.state.cia_b, restored.state.cia_b);
    assert_eq!(state.state.game_ports, restored.state.game_ports);
    assert_eq!(
        state.state.paula.serial_port,
        restored.state.paula.serial_port
    );
    assert_eq!(state, restored);
}
#[test]
fn test_save_state_bytes_roundtrip() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    machine.step_cycles(500);

    let state = SaveState::capture(&machine);
    let json_bytes = serde_json::to_vec(&state).expect("Serialization failed");

    let restored = SaveState::from_bytes(&json_bytes).expect("Deserialization failed");
    assert_eq!(state, restored);
}
#[test]
fn test_save_state_file_persistence() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    machine.step_cycles(150);

    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("test_save_state_amiga.json");

    SaveState::capture(&machine)
        .save_to_file(&temp_path)
        .expect("Failed to write save state file");

    let mut restored_machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    let saved = SaveState::load_from_file(&temp_path).expect("Failed to read save state file");
    restored_machine.load_state(&saved.state);

    assert_eq!(machine.cck, restored_machine.cck);
    assert_eq!(machine.cpu.state, restored_machine.cpu.state);

    let _ = std::fs::remove_file(&temp_path);
}

#[test]
fn unsupported_file_version_is_rejected_before_restore() {
    let machine = A500Machine::new(A500Config::default());
    let mut saved = SaveState::capture(&machine);
    saved.header.version = SAVE_STATE_VERSION + 1;
    let json = saved.to_json().unwrap();
    assert!(matches!(
        SaveState::from_json(&json),
        Err(SaveStateError::IncompatibleVersion { .. })
    ));
    assert!(matches!(
        SaveState::from_bytes(json.as_bytes()),
        Err(SaveStateError::IncompatibleVersion { .. })
    ));
    assert_eq!(machine.save_state(), *saved.state);
}

#[test]
fn file_roundtrip_resumes_an_undelivered_rga_event() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.cpu.state.set_stopped(true);
    machine.agnus.vpos = 100;
    machine.step_cck();
    assert_eq!(
        machine.denise.pins.incoming.as_ref().unwrap().rga_strobe,
        Some(config::custom_reg::denise::STRHOR)
    );
    let saved = SaveState::from_json(&SaveState::capture(&machine).to_json().unwrap()).unwrap();
    let mut restored = A500Machine::new(A500Config::default());
    restored.load_state(&saved.state);
    assert_eq!(machine.save_state(), restored.save_state());
    assert!(!restored.denise.sync.synchronized);
    for _ in 0..8 {
        machine.step_cck();
        restored.step_cck();
        assert_eq!(machine.save_state(), restored.save_state());
    }
    assert!(restored.denise.sync.synchronized);
}
