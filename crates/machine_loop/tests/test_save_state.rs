//! Integration tests for Amiga 500 Save State Serialization and Restoration

use config::{A500Config, A500Preset, VideoStandard};
use machine_loop::{
    compute_crc32, A500Machine, A500State, SaveStateError, SAVE_STATE_MAGIC, SAVE_STATE_VERSION,
};

#[test]
fn test_save_state_metadata_and_header() {
    let machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Standard1Mb,
        VideoStandard::Pal,
    ));
    let state = machine.save_state();

    assert_eq!(state.header.magic, SAVE_STATE_MAGIC);
    assert_eq!(state.header.version, SAVE_STATE_VERSION);
    assert_eq!(state.header.chip_ram_size, 512 * 1024);
    assert_eq!(state.header.slow_ram_size, 512 * 1024);
    assert_eq!(state.header.fast_ram_size, 0);
    assert!(!state.header.is_self_contained);
    assert_eq!(state.cck, 0);
}

#[test]
fn test_save_state_json_roundtrip() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    machine.step_cycles(200);

    let state = machine.save_state();
    let json_str = state.to_json().expect("JSON serialization failed");
    let restored: A500State = A500State::from_json(&json_str).expect("JSON deserialization failed");

    assert_eq!(state.header, restored.header);
    assert_eq!(state.cck, restored.cck);
    assert_eq!(state.cpu, restored.cpu);
    assert_eq!(state.agnus, restored.agnus);
    assert_eq!(state.denise, restored.denise);
    assert_eq!(state.paula, restored.paula);
    assert_eq!(state.cia_a, restored.cia_a);
    assert_eq!(state.cia_b, restored.cia_b);
    assert_eq!(state, restored);
}

#[test]
fn test_save_state_compressed_roundtrip() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    machine.step_cycles(500);

    let state = machine.save_state();
    let compressed = state
        .to_compressed_bytes()
        .expect("Gzip compression failed");
    assert!(
        compressed.len() < 50_000,
        "Compressed state should be lean (< 50KB)"
    );

    let restored = A500State::from_bytes(&compressed).expect("Decompression failed");
    assert_eq!(state, restored);
}

#[test]
fn test_save_state_deterministic_stepping_roundtrip() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Standard1Mb,
        VideoStandard::Pal,
    ));

    // 1. Advance machine by 300 CCKs
    machine.step_cycles(300);

    // 2. Capture baseline save state
    let snapshot = machine.save_state();

    // 3. Step forward another 600 CCKs to produce branch A
    machine.step_cycles(600);
    let state_a = machine.save_state();

    // 4. Restore the baseline snapshot into machine
    machine
        .load_state(&snapshot)
        .expect("Failed to restore save state");

    // 5. Step forward the exact same 600 CCKs to produce branch B
    machine.step_cycles(600);
    let state_b = machine.save_state();

    // 6. Assert bit-for-bit determinism across branches A and B
    assert_eq!(
        state_a.cck, state_b.cck,
        "Color Clock cycle count must match exactly"
    );
    assert_eq!(
        state_a.cpu, state_b.cpu,
        "CPU state must be bit-for-bit identical"
    );
    assert_eq!(
        state_a.agnus, state_b.agnus,
        "Agnus state must be bit-for-bit identical"
    );
    assert_eq!(
        state_a.denise, state_b.denise,
        "Denise state must be bit-for-bit identical"
    );
    assert_eq!(
        state_a.paula, state_b.paula,
        "Paula state must be bit-for-bit identical"
    );
    assert_eq!(
        state_a.cia_a, state_b.cia_a,
        "CIA-A state must be bit-for-bit identical"
    );
    assert_eq!(
        state_a.cia_b, state_b.cia_b,
        "CIA-B state must be bit-for-bit identical"
    );
    assert_eq!(
        state_a.physical_memory, state_b.physical_memory,
        "PhysicalMemory buffers must match"
    );
    assert_eq!(state_a, state_b);
}

#[test]
fn test_save_state_kickstart_mismatch_guard() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    // Synthetic Kickstart ROM
    machine
        .physical_memory
        .inject_kickstart_rom(&[0x11; 256 * 1024]);
    let mut state = machine.save_state();

    // Alter expected Kickstart CRC in save state
    state.header.kickstart_crc32 = 0xDEADBEEF;

    let err = machine
        .load_state(&state)
        .expect_err("Loading state with mismatched Kickstart CRC must fail");
    match err {
        SaveStateError::KickstartMismatch {
            expected_crc,
            actual_crc,
        } => {
            assert_eq!(expected_crc, 0xDEADBEEF);
            assert_ne!(actual_crc, 0xDEADBEEF);
        }
        other => panic!("Expected KickstartMismatch, got {:?}", other),
    }
}

#[test]
fn test_save_state_ram_size_mismatch_guard() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    let mut state = machine.save_state();

    // Alter expected Chip RAM size to 1MB
    state.header.chip_ram_size = 1024 * 1024;

    let err = machine
        .load_state(&state)
        .expect_err("Loading state with mismatched Chip RAM size must fail");
    match err {
        SaveStateError::MemorySizeMismatch {
            expected_chip,
            actual_chip,
        } => {
            assert_eq!(expected_chip, 1024 * 1024);
            assert_eq!(actual_chip, 512 * 1024);
        }
        other => panic!("Expected MemorySizeMismatch, got {:?}", other),
    }
}

#[test]
fn test_save_state_self_contained_roundtrip() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    let dummy_rom = vec![0x42; 256 * 1024];
    machine.physical_memory.inject_kickstart_rom(&dummy_rom);

    let state = machine.save_state_self_contained();
    assert!(state.header.is_self_contained);
    assert_eq!(state.physical_memory.kickstart_rom.len(), 256 * 1024);

    let mut target_machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    assert!(!target_machine.physical_memory.is_kickstart_loaded());

    target_machine
        .load_state(&state)
        .expect("Failed to load self-contained state");
    assert!(target_machine.physical_memory.is_kickstart_loaded());
    assert_eq!(
        compute_crc32(&target_machine.physical_memory.kickstart_rom),
        compute_crc32(&dummy_rom)
    );
}

#[test]
fn test_save_state_file_persistence() {
    let mut machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    machine.step_cycles(150);

    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("test_save_state_amiga.a500z");

    machine
        .save_state_to_file(&temp_path, false)
        .expect("Failed to write save state file");

    let mut restored_machine = A500Machine::new(A500Config::from_preset(
        A500Preset::Bare512k,
        VideoStandard::Pal,
    ));
    restored_machine
        .load_state_from_file(&temp_path)
        .expect("Failed to read save state file");

    assert_eq!(machine.cck, restored_machine.cck);
    assert_eq!(machine.cpu.state, restored_machine.cpu.state);

    let _ = std::fs::remove_file(&temp_path);
}
