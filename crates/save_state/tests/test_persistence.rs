#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{A500Config, A500Preset, VideoStandard};
use machine_loop::A500Machine;
use save_state::{SaveState, SaveStateError, SAVE_STATE_VERSION};

#[test]
fn snapshot_rejects_kickstart_buffers_of_other_sizes() {
    let machine = A500Machine::new(A500Config::default());
    let mut value = serde_json::to_value(SaveState::capture(&machine)).unwrap();
    for size in [0, 262143, 524288] {
        value["state"]["physical_memory"]["kickstart_rom"] = serde_json::json!(vec![0u8; size]);
        let bytes = serde_json::to_vec(&value).unwrap();
        let error = SaveState::from_bytes(&bytes).unwrap_err();
        assert!(matches!(error, SaveStateError::DeserializationFailed(_)));
        assert!(error
            .to_string()
            .contains("Kickstart ROM must be exactly 256 KiB (262144 bytes)"));
        assert!(error
            .to_string()
            .contains(&format!("received {size} bytes")));
    }
}

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
    for version in [SAVE_STATE_VERSION - 1, SAVE_STATE_VERSION + 1] {
        saved.header.version = version;
        let json = saved.to_json().unwrap();
        assert!(matches!(
            SaveState::from_json(&json),
            Err(SaveStateError::IncompatibleVersion { .. })
        ));
        assert!(matches!(
            SaveState::from_bytes(json.as_bytes()),
            Err(SaveStateError::IncompatibleVersion { .. })
        ));
    }
    assert_eq!(machine.save_state(), *saved.state);
}

#[test]
fn dma_registers_roundtrip_in_their_single_owner() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.cpu.state.set_stopped(true);
    assert!(machine.agnus.set_dmacon(0x8300));
    assert!(machine.agnus.set_bplcon0(0x2004));
    assert!(machine.agnus.set_ddfstrt(0x0039));
    assert!(machine.agnus.set_ddfstop(0x00d3));
    for _ in 0..4 {
        machine.step_cck();
    }
    assert_eq!(machine.agnus.read_dmaconr() & 0x07ff, 0x0300);
    assert_eq!(machine.agnus.dma.planecount(), 2);
    let saved = SaveState::capture(&machine);
    let json = serde_json::to_value(&saved).unwrap();
    assert_eq!(json["header"]["version"], 4);
    for (name, value) in [
        ("dmacon", 0x0300),
        ("bplcon0", 0x2004),
        ("ddfstrt", 0x0038),
        ("ddfstop", 0x00d0),
    ] {
        assert!(json["state"]["agnus"].get(name).is_none());
        assert_eq!(json["state"]["agnus"]["dma"][name], value);
    }
    let restored = SaveState::from_json(&json.to_string()).unwrap();
    let mut resumed = A500Machine::new(A500Config::default());
    resumed.load_state(&restored.state);
    for _ in 0..128 {
        machine.step_cck();
        resumed.step_cck();
    }
    assert_eq!(machine.save_state(), resumed.save_state());
}

#[test]
fn file_roundtrip_resumes_an_undelivered_rga_event() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.cpu.state.set_stopped(true);
    machine.agnus.vpos = 100;
    machine.step_cck();
    assert_eq!(
        machine.denise.pins.incoming.as_ref().unwrap().rga_strobe,
        Some(config::custom_reg::denise::REGISTER_STRHOR)
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

#[test]
fn file_roundtrip_resumes_a_partial_copper_instruction_after_a_denied_fetch() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = A500Machine::new(A500Config::bare_512k(standard));
        machine.cpu.state.set_stopped(true);
        machine.physical_memory.chip_ram[..8]
            .copy_from_slice(&[0x01, 0x80, 0x0F, 0x00, 0xFF, 0xFF, 0xFF, 0xFE]);
        machine.agnus.dma.write_dmacon(0x8280);
        machine.agnus.copper.set_dma_enabled(true);
        machine.agnus.copper.restart_list1();
        machine.agnus.hpos = 0x3F;
        machine.agnus.vpos = 100;
        machine.step_cck();
        assert_eq!(machine.agnus.copper.cop_pc, 2);
        assert_eq!(machine.agnus.pending_copper_write, None);

        // A four-plane hires fetch denies the second Copper word.
        machine.agnus.dma.write_dmacon(0x8100);
        machine.agnus.dma.set_bplcon0(0xC000);
        machine.step_cck();
        assert_eq!(machine.agnus.copper.cop_pc, 2);
        let saved = SaveState::from_json(&SaveState::capture(&machine).to_json().unwrap()).unwrap();
        let mut restored = A500Machine::new(A500Config::default());
        restored.load_state(&saved.state);
        machine.agnus.dma.write_dmacon(0x0100);
        restored.agnus.dma.write_dmacon(0x0100);
        for _ in 0..12 {
            machine.step_cck();
            restored.step_cck();
            assert_eq!(machine.save_state(), restored.save_state());
        }
        assert_eq!(restored.denise.color[0], 0x0F00);
    }
}

#[test]
fn file_roundtrip_resumes_blitter_source_carry_and_pending_destination() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = A500Machine::new(A500Config::bare_512k(standard));
        machine.cpu.state.set_stopped(true);
        machine.agnus.hpos = 0x3f;
        machine.agnus.dma.write_dmacon(0x8640);
        let b = &mut machine.agnus.blitter;
        b.bltcon0 = 0x49f0;
        b.bltafwm = 0xffff;
        b.bltalwm = 0xffff;
        b.bltdpt = 16;
        b.set_dma_enabled(true);
        b.trigger_blit(0x0042);
        machine.physical_memory.chip_ram[..4].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
        for _ in 0..4 {
            machine.step_cck();
        }
        assert_eq!(machine.agnus.blitter.bltapt, 4);
        assert_eq!(machine.agnus.blitter.bltdpt, 16);
        assert!(machine.agnus.blitter.is_busy);
        let saved = SaveState::from_json(&SaveState::capture(&machine).to_json().unwrap()).unwrap();
        let mut restored = A500Machine::new(A500Config::default());
        restored.load_state(&saved.state);
        for _ in 0..8 {
            machine.step_cck();
            restored.step_cck();
            assert_eq!(machine.save_state(), restored.save_state());
        }
        assert_eq!(
            &restored.physical_memory.chip_ram[16..20],
            &[0x01, 0x23, 0x45, 0x67]
        );
        assert!(!restored.agnus.blitter.is_busy);
        assert_ne!(restored.paula.interrupts.intreq & 0x0040, 0);
    }
}

#[test]
fn file_roundtrip_preserves_drawn_pixels_and_resumes_active_display() {
    let program =
        include_bytes!("../../machine_loop/tests/programs/display/live_stripes/program.bin");
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.physical_memory.map_chip_ram_to_low_memory();
    machine.physical_memory.chip_ram[0x1000..0x1000 + program.len()].copy_from_slice(program);
    machine.cpu.state.set_sr(0x2700);
    machine.set_pc_and_prime_prefetch(0x1000);
    for _ in 0..(313 * 228) {
        machine.step_cck();
        if machine.agnus.vpos == 120 && machine.agnus.hpos == 100 {
            break;
        }
    }
    assert_eq!(machine.agnus.vpos, 120);
    assert_ne!(
        machine
            .denise
            .frame_builder
            .get_pixel(300 - 144, (100 - 26) * 2),
        0xff000000
    );
    assert!(machine.denise.bpl_armed);
    assert!(machine
        .denise
        .pipeline_pixels_valid
        .iter()
        .all(|valid| *valid));
    let saved = SaveState::from_json(&SaveState::capture(&machine).to_json().unwrap()).unwrap();
    let mut restored = A500Machine::new(A500Config::default());
    restored.load_state(&saved.state);
    assert_eq!(machine.denise.frame_builder, restored.denise.frame_builder);
    for _ in 0..64 {
        machine.step_cck();
        restored.step_cck();
        assert_eq!(machine.denise, restored.denise);
        assert_eq!(machine.agnus, restored.agnus);
    }
    assert_ne!(
        restored
            .denise
            .frame_builder
            .get_pixel(600 - 144, (120 - 26) * 2),
        0xff000000
    );
}

#[test]
fn file_roundtrip_resumes_uart_transmit_and_partial_receive_shifters() {
    let mut machine = A500Machine::new(A500Config::default());
    let serial = &mut machine.paula.serial_port;
    serial.write_serper(3);
    serial.write_serdat(0x0155);
    // Start bit and three data bits of $A6, with the transmitter running too.
    for high in [false, false, true, true] {
        for _ in 0..4 {
            serial.step_cck(high, false);
        }
    }
    let saved = SaveState::from_json(&SaveState::capture(&machine).to_json().unwrap()).unwrap();
    let mut restored = A500Machine::new(A500Config::default());
    restored.load_state(&saved.state);
    let mut received = 0;
    for high in [false, false, true, false, true, true] {
        for _ in 0..4 {
            let request = machine.paula.serial_port.step_cck(high, false);
            received |= request;
            assert_eq!(restored.paula.serial_port.step_cck(high, false), request);
            assert_eq!(machine.paula.serial_port, restored.paula.serial_port);
        }
    }
    assert_eq!(received & 0x0800, 0x0800);
    assert_eq!(
        restored.paula.serial_port.read_status(true) & 0x03ff,
        0x01a6
    );
    // The first CCK loaded TX; finish the final stop-bit interval as well.
    assert_eq!(machine.paula.serial_port.step_cck(true, true), 0);
    assert_eq!(restored.paula.serial_port.step_cck(true, true), 0);
    assert_eq!(machine.paula.serial_port, restored.paula.serial_port);
    assert_eq!(
        restored.paula.serial_port.read_status(true) & 0x3000,
        0x3000
    );
}

#[test]
fn file_roundtrip_resumes_cia_serial_bits_timer_outputs_and_pc_pulses() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.cia_a.ta_latch = 1;
    machine.cia_a.ta_counter = 1;
    assert!(machine.cia_a.set_cra(0x47));
    for _ in 0..2 {
        machine.cia_a.step_cck();
    }
    assert!(machine.cia_a.set_sdr(0xa6));
    for _ in 0..33 {
        machine.cia_a.step_cck();
    }
    // Three bits received and a Port B access still in the PC pipeline.
    for high in [true, false, true] {
        machine.cia_b.receive_serial_pins(high, false);
        machine.cia_b.step_cck();
        machine.cia_b.receive_serial_pins(high, true);
        machine.cia_b.step_cck();
    }
    machine.cia_b.prb();
    let saved = SaveState::from_json(&SaveState::capture(&machine).to_json().unwrap()).unwrap();
    let mut restored = A500Machine::new(A500Config::default());
    restored.load_state(&saved.state);
    for high in [false, false, true, true, false] {
        for cnt in [false, true] {
            for chip in [&mut machine.cia_b, &mut restored.cia_b] {
                chip.receive_serial_pins(high, cnt);
                chip.step_cck();
            }
            assert_eq!(machine.cia_b, restored.cia_b);
        }
    }
    assert_eq!(restored.cia_b.sdr(), 0xa6);
    assert_eq!(restored.cia_b.icr() & 8, 8);
    for _ in 0..200 {
        machine.cia_a.step_cck();
        restored.cia_a.step_cck();
        assert_eq!(machine.cia_a, restored.cia_a);
    }
    assert_eq!(restored.cia_a.icr() & 8, 8);
    restored.cia_a.reset();
    assert!(restored.cia_a.pins.sp_output_high);
    assert!(restored.cia_a.pins.cnt_output_high);
    assert!(restored.cia_a.pins.pc_high);
}

#[test]
fn file_roundtrip_resumes_disk_write_fifo_and_partial_bit_cell() {
    let mut machine = A500Machine::new(A500Config::default());
    let paula = &mut machine.paula;
    assert!(paula.set_dmacon(0x8210));
    assert!(paula.set_adkcon(0x8100));
    assert!(paula.set_dsklen(0xc003));
    paula.step_cck();
    assert!(paula.set_dsklen(0xc003));
    paula.step_cck();
    for word in [0xa55a, 0x8001, 0xffff] {
        assert!(paula.set_dskdat(word));
    }
    paula.step_cck();
    for _ in 0..19 {
        paula.step_cck();
    }
    assert_eq!(paula.disk.buffered_words(), 2);
    assert!(paula.pins.disk_write_enable);
    let saved = SaveState::from_json(&SaveState::capture(&machine).to_json().unwrap()).unwrap();
    let mut restored = A500Machine::new(A500Config::default());
    restored.load_state(&saved.state);
    let mut transitions = 0;
    for _ in 0..350 {
        machine.paula.step_cck();
        restored.paula.step_cck();
        assert_eq!(machine.paula, restored.paula);
        transitions += usize::from(!restored.paula.pins.disk_write_high);
    }
    assert_eq!(
        transitions, 24,
        "26 set bits minus two emitted before the snapshot"
    );
    assert_eq!(restored.paula.disk.buffered_words(), 0);
    assert!(!restored.paula.pins.disk_write_enable);
    restored.paula.reset();
    assert_eq!(restored.paula, machine_loop::paula::Paula::new());
}
