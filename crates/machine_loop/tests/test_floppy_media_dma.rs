//! Autonomous ADF transitions through Paula, DMAL, Agnus and Chip RAM.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use common::MachineHarness;
use config::{custom_reg, A500Preset, VideoStandard};
use machine_loop::{A500Machine, A500State};

fn mounted(standard: VideoStandard, side: u8, adf: &[u8]) -> A500Machine {
    let mut harness = MachineHarness::new_with_preset(A500Preset::Standard1Mb);
    if standard == VideoStandard::Ntsc {
        harness.machine = A500Machine::new(config::A500Config::standard_1mb(standard));
        harness.machine.physical_memory.map_chip_ram_to_low_memory();
    }
    // CPU contends for Chip RAM while incrementing D0.
    harness.load_cpu_code(0x10000, &[0x7000, 0x5280, 0x60fc]);
    let m = &mut harness.machine;
    assert!(m.floppy.drives[0].insert_disk(adf));
    common::cia::configure_drive_control_outputs(m);
    common::cia::write_byte(m, 0xbfd100, 0x7f);
    common::cia::write_byte(m, 0xbfd100, if side == 0 { 0x77 } else { 0x73 });
    m.step_cycles(2);
    // Prepare a steady drive aligned to sector zero before timing the RAM transfer.
    for _ in 0..standard.color_clock_hz().div_ceil(2) {
        m.floppy.step_cck();
    }
    assert!(m.floppy.drives[0].is_ready());
    align_to_index(m, standard);
    harness.machine
}

fn align_to_index(m: &mut A500Machine, standard: VideoStandard) {
    let mut ticks = 0;
    loop {
        let released = m.floppy.pins.index_high;
        m.floppy.step_cck();
        ticks += 1;
        if released && !m.floppy.pins.index_high {
            break;
        }
        assert!(ticks <= standard.color_clock_hz() / 5);
    }
}

fn start_dma(m: &mut A500Machine, address: u16, length: u16, sync: u16) {
    assert!(m
        .memory_bus()
        .write_custom_word(custom_reg::agnus::REGISTER_DSKPTL, address, "DMA"));
    assert!(m.paula.set_dsksync(sync));
    assert!(m.paula.set_adkcon(0x8500));
    assert!(m
        .memory_bus()
        .write_custom_word(custom_reg::agnus::REGISTER_DMACON, 0x8210, "DMA"));
    assert!(m.paula.set_dsklen(length));
    m.step_cycles(3);
    assert!(!m.paula.dma_active);
    assert!(m.paula.set_dsklen(length));
    m.step_cycles(3);
    assert!(m.paula.dma_active);
    assert_ne!(m.paula.peek_dskbytr() & 0x4000, 0);
}

fn reading(standard: VideoStandard, side: u8, words: u16) -> A500Machine {
    let mut machine = mounted(standard, side, &vec![0; floppy::FORMATTED_DISK_BYTES]);
    start_dma(&mut machine, 0x2000, 0x8000 | words, 0x4489);
    machine
}

fn complete(machine: &mut A500Machine) -> Vec<(u64, u16)> {
    let mut transfers = Vec::new();
    let mut pointer = machine.agnus.dskpt;
    for _ in 0..800_000 {
        machine.step_cck();
        if machine.agnus.dskpt != pointer {
            assert_eq!(machine.agnus.dskpt, pointer + 2);
            transfers.push((
                machine.cck,
                machine.physical_memory.read_chip_word_debug(pointer),
            ));
            pointer += 2;
        }
        if !machine.paula.dma_active {
            return transfers;
        }
    }
    panic!("autonomous DMA timed out");
}

#[test]
fn image_alone_supplies_exact_words_on_both_surfaces_and_video_clocks() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for side in [0, 1] {
            let mut m = reading(standard, side, 13);
            let transfers = complete(&mut m);
            let actual: Vec<_> = transfers.iter().map(|(_, word)| *word).collect();
            // FF track 00 0B, split odd/even; clocks carried from sync 4489.
            let even_track = if side == 0 { 0xaaa9 } else { 0x2aa9 };
            let expected = [
                0x4489,
                0x552a,
                0xaaa5,
                if side == 0 { 0x552a } else { 0x5529 },
                even_track,
                0x2aaa,
                0xaaaa,
                0xaaaa,
                0xaaaa,
                0xaaaa,
                0xaaaa,
                0xaaaa,
                0xaaaa,
            ];
            assert_eq!(actual, expected);
            assert_eq!(m.floppy.drives[0].side, side);
            assert_eq!(m.agnus.dskpt, 0x201a);
            assert_eq!(m.paula.disk_activity.sequences[0], 14);
            assert!(m.paula.disk_activity.sequences[1..]
                .iter()
                .all(|sequence| *sequence == 0));
            assert_eq!(m.physical_memory.read_chip_word_debug(0x201a), 0);
            assert_eq!(m.paula.dsklen & 0x3fff, 0);
            assert_eq!(m.paula.interrupts.intreq & 0x1002, 0x1002);
            assert!(m.cpu.state.d_long(0) > 0);
            assert_ne!(m.paula.peek_dskbytr() & 0x8000, 0);
            m.paula.read_dskbytr();
            assert_eq!(m.paula.peek_dskbytr() & 0x8000, 0);
            let pointer = m.agnus.dskpt;
            m.step_cycles(1000);
            assert_eq!(m.agnus.dskpt, pointer, "completion cannot replay a word");
        }
    }
}

#[test]
fn serialized_mid_transfer_resumes_media_and_exact_completion_cck() {
    let mut m = reading(VideoStandard::Pal, 0, 24);
    while m.agnus.dskpt < 0x2006 {
        m.step_cck();
    }
    m.step_cycles(19); // Partial cell/word and pending connector state.
    let encoded = serde_json::to_string(&m.save_state()).unwrap();
    let state: A500State = serde_json::from_str(&encoded).unwrap();
    let mut restored = A500Machine::new(config::A500Config::default());
    restored.load_state(&state);
    let original_words = complete(&mut m);
    assert_eq!(complete(&mut restored), original_words);
    assert_eq!(m.save_state(), restored.save_state());
    assert!(restored.floppy.drives[0].disk_inserted);
}

#[test]
fn chip_ram_write_pulses_survive_serialization_and_read_back_at_their_recorded_spacing() {
    let words = [
        0xaaaa, 0x5224, 0x9254, 0xaa49, 0x24a9, 0x2a52, 0x492a, 0x9292,
    ];
    let mut machine = mounted(
        VideoStandard::Pal,
        0,
        &vec![0; floppy::FORMATTED_DISK_BYTES],
    );
    for (i, word) in words.into_iter().enumerate() {
        machine
            .physical_memory
            .write_chip_word_debug(0x2000 + 2 * i as u32, word);
    }
    start_dma(&mut machine, 0x2000, 0xc008, 0x5224);
    for _ in 0..1000 {
        machine.step_cck();
        if machine.paula.pins.disk_write_enable {
            break;
        }
    }
    assert!(machine.paula.pins.disk_write_enable && machine.floppy.pins.write_enable_high);
    let json = serde_json::to_string(&machine.save_state()).unwrap();
    let mut restored = A500Machine::new(config::A500Config::default());
    restored.load_state(&serde_json::from_str(&json).unwrap());
    let mut gate_open_ticks = 0;
    for _ in 0..2000 {
        machine.step_cck();
        restored.step_cck();
        assert_eq!(machine.floppy.pins, restored.floppy.pins);
        assert_eq!(machine.agnus.dskpt, restored.agnus.dskpt);
        assert_eq!(machine.paula.dma_active, restored.paula.dma_active);
        gate_open_ticks += usize::from(!machine.floppy.pins.write_enable_high);
    }
    assert_eq!(gate_open_ticks, words.len() * 16 * 7);
    assert_eq!(machine.agnus.dskpt, 0x2010);
    assert_eq!(machine.paula.dsklen & 0x3fff, 0);
    assert_eq!(machine.paula.interrupts.intreq & 2, 2);
    assert_eq!(machine.save_state(), restored.save_state());
    assert!(restored.paula.set_intreq(0x1002));
    restored.step_cycles(3);
    assert_eq!(restored.paula.interrupts.intreq & 0x1002, 0);
    // Start at sector zero again. Only the guest write contains the C2 sync.
    align_to_index(&mut restored, VideoStandard::Pal);
    restored
        .physical_memory
        .write_chip_word_debug(0x400c, 0xdead);
    assert_eq!(restored.paula.interrupts.intreq & 2, 0);
    start_dma(&mut restored, 0x4000, 0x8006, 0x5224);
    let returned: Vec<_> = complete(&mut restored)
        .into_iter()
        .map(|(_, word)| word)
        .collect();
    assert_eq!(returned, words[2..]);
    assert_eq!(restored.agnus.dskpt, 0x400c);
    assert_eq!(
        restored.physical_memory.read_chip_word_debug(0x400c),
        0xdead
    );
    assert_eq!(restored.paula.dsklen & 0x3fff, 0);
    assert_eq!(restored.paula.interrupts.intreq & 0x1002, 0x1002);
}

#[test]
fn raw_dma_crosses_track_gap_and_wrap_without_restarting_the_spindle() {
    let mut m = reading(VideoStandard::Pal, 0, 6263);
    while m.agnus.dskpt < 0x2004 {
        m.step_cck();
    }
    // Keep the acquired alignment, then retain subsequent sync words as raw data.
    assert!(m.paula.set_adkcon(0x0400));
    complete(&mut m);
    assert_eq!(m.physical_memory.read_chip_word_debug(0x2000), 0x4489);
    let word = |i: u32| m.physical_memory.read_chip_word_debug(0x2002 + i * 2);
    assert_eq!(
        (word(0), word(1), word(2), word(3)),
        (0x552a, 0xaaa5, 0x552a, 0xaaa9)
    );
    for i in 5980..6248 {
        assert_eq!(word(i), 0xaaaa);
    }
    assert_eq!((word(6248), word(6249)), (0x4489, 0x4489));
    for i in 0..12 {
        assert_eq!(word(6250 + i), word(i));
    }
    assert_eq!(m.agnus.dskpt, 0x2000 + 6263 * 2);
}

#[test]
fn reset_cancels_active_dma_but_preserves_media_without_false_completion() {
    let mut m = reading(VideoStandard::Pal, 0, 24);
    while m.agnus.dskpt < 0x2006 {
        m.step_cck();
    }
    m.floppy.pins.incoming = Some(floppy::FloppyInputs {
        control: 0x74,
        write_data_high: false,
        write_enable_high: false,
    });
    let mut cold = m.clone();
    cold.reset();
    m.reset_warm();
    for machine in [&mut cold, &mut m] {
        machine.cpu.state.set_stopped(true);
        assert!(machine.floppy.drives[0].disk_inserted);
        assert!(!machine.floppy.drives[0].motor_on);
        machine.step_cycles(1000);
        assert_eq!(machine.agnus.dskpt, 0);
        assert!(!machine.paula.dma_active);
        assert_eq!(machine.paula.disk.buffered_words(), 0);
        assert_eq!(machine.paula.interrupts.intreq & 0x1002, 0);
        assert!(machine.floppy.pins.write_enable_high && machine.floppy.pins.index_high);
    }
    assert_eq!(cold.floppy, m.floppy);
}

#[test]
fn seek_reverse_and_side_selection_return_the_recorded_sector_on_each_surface() {
    let standard = VideoStandard::Pal;
    let mut adf = vec![0; floppy::FORMATTED_DISK_BYTES];
    for (i, track) in adf.chunks_exact_mut(11 * 512).enumerate() {
        track[..512].fill(i as u8);
    }
    let mut m = mounted(standard, 0, &adf);
    common::cia::write_byte(&mut m, 0xbfd100, 0x75);
    for cylinder in 1..=4 {
        m.step_cycles(10);
        common::cia::write_byte(&mut m, 0xbfd100, 0x74);
        m.step_cycles(10);
        common::cia::write_byte(&mut m, 0xbfd100, 0x75);
        m.step_cycles(10_643);
        assert_eq!(m.floppy.drives[0].cylinder, cylinder);
        assert!(m.floppy.drives[0].is_ready());
    }
    m.step_cycles(53_204);
    common::cia::write_byte(&mut m, 0xbfd100, 0x77);
    m.step_cycles(10);
    common::cia::write_byte(&mut m, 0xbfd100, 0x76);
    m.step_cycles(10);
    common::cia::write_byte(&mut m, 0xbfd100, 0x77);
    m.step_cycles(63_847); // 3 ms travel and 15 ms settling.
    assert_eq!(m.floppy.drives[0].cylinder, 3);
    for track in [6, 6, 7] {
        if track == 7 {
            common::cia::write_byte(&mut m, 0xbfd100, 0x73);
            m.step_cycles(356);
        }
        align_to_index(&mut m, standard);
        start_dma(&mut m, 0x2000, 0x8000 | 541, 0x4489);
        let mut sector = vec![0xaa, 0xaa, 0xaa, 0xaa, 0x44, 0x89];
        for (_, word) in complete(&mut m) {
            sector.extend(word.to_be_bytes());
        }
        assert_eq!(sector.len(), 1088);
        assert_eq!(
            floppy::decode_amiga_sector(&sector),
            Ok((track, 0, [track; 512]))
        );
    }
}

#[test]
fn drive_index_latches_only_one_ciab_flag_request_per_revolution() {
    let standard = VideoStandard::Pal;
    let mut m = mounted(standard, 0, &vec![0; floppy::FORMATTED_DISK_BYTES]);
    assert!(!m.floppy.pins.index_high);
    m.step_cycles(3);
    assert_eq!(m.cia_b.icr() & 0x10, 0x10);
    assert_eq!(m.cia_a.icr() & 0x10, 0);
    let width = (standard.color_clock_hz() * 4).div_ceil(1000);
    m.step_cycles(u64::from(width - 4));
    assert!(!m.floppy.pins.index_high);
    m.step_cycles(2);
    assert!(m.floppy.pins.index_high);
    assert!(m.cia_b.pins.flag_high);
    assert_eq!(m.cia_b.icr() & 0x10, 0);
    m.step_cycles(u64::from(standard.color_clock_hz() / 5 - width - 1));
    assert!(!m.floppy.pins.index_high);
    assert!(m.cia_b.pins.flag_high);
    m.step_cck();
    assert_eq!(m.cia_b.icr() & 0x10, 0x10);
    assert_eq!(m.cia_a.icr() & 0x10, 0);
}

fn compare_snapshot(m: &mut A500Machine, ticks: u32) {
    let encoded = serde_json::to_string(&m.save_state()).unwrap();
    let mut resumed = A500Machine::new(config::A500Config::default());
    resumed.load_state(&serde_json::from_str(&encoded).unwrap());
    for _ in 0..ticks {
        m.step_cck();
        resumed.step_cck();
        assert_eq!(m.floppy.pins, resumed.floppy.pins);
        assert_eq!(m.cia_a.pins.port_a_input, resumed.cia_a.pins.port_a_input);
        assert_eq!(
            m.floppy.drives[0].cylinder,
            resumed.floppy.drives[0].cylinder
        );
    }
    assert_eq!(m.save_state(), resumed.save_state());
}

#[test]
fn snapshots_preserve_transient_motion_head_activity_and_the_asserted_index() {
    let standard = VideoStandard::Pal;
    let mut m = mounted(standard, 0, &vec![0; floppy::FORMATTED_DISK_BYTES]);
    compare_snapshot(&mut m, 14_200); // Resume inside the INDEX aperture.
    common::cia::write_byte(&mut m, 0xbfd100, 0xff);
    common::cia::write_byte(&mut m, 0xbfd100, 0xf7);
    m.step_cycles(3721);
    compare_snapshot(&mut m, 2000); // Coasting with flux still passing the head.
    common::cia::write_byte(&mut m, 0xbfd100, 0x7f);
    common::cia::write_byte(&mut m, 0xbfd100, 0x75);
    m.step_cycles(1700);
    assert!(!m.floppy.drives[0].is_ready());
    compare_snapshot(&mut m, 42_000);
    assert!(m.floppy.drives[0].is_ready());
    common::cia::write_byte(&mut m, 0xbfd100, 0x74);
    m.step_cycles(10);
    common::cia::write_byte(&mut m, 0xbfd100, 0x75);
    m.step_cycles(397);
    compare_snapshot(&mut m, 10_641);
    assert_eq!(m.floppy.drives[0].cylinder, 1);
    compare_snapshot(&mut m, 53_204);
}

#[test]
fn cold_warm_and_guest_reset_cancel_seek_and_dma_and_retain_the_recording_and_angle() {
    let mut reference = common::continuation::rom_machine(&[0x4e70, 0x60fe]);
    reference.cpu.state.set_stopped(true);
    assert!(reference.floppy.drives[0].insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
    reference.floppy.drives[0].cylinder = 23;
    common::cia::configure_drive_control_outputs(&mut reference);
    common::cia::write_byte(&mut reference, 0xbfd100, 0x7f);
    common::cia::write_byte(&mut reference, 0xbfd100, 0x75);
    reference.step_cycles(9_567);
    for addr in (0x2000..0x2100).step_by(2) {
        reference
            .physical_memory
            .write_chip_word_debug(addr, 0xaaaa);
    }
    start_dma(&mut reference, 0x2000, 0xc080, 0x4489);
    reference.step_cycles(1_000);
    common::cia::write_byte(&mut reference, 0xbfd100, 0x74);
    reference.step_cycles(10);
    common::cia::write_byte(&mut reference, 0xbfd100, 0x75);
    reference.floppy.drives[0].write_protected = true;

    // The drive advances before the CPU in each CCK. Keep the reference
    // running up to the same physical instant at which the guest asserts RESET.
    let mut guest = reference.clone();
    guest.cpu.state.set_stopped(false);
    for _ in 0..66 {
        reference.step_cck();
        guest.step_cck();
        if guest.cpu.state.is_reset_line_asserted() {
            break;
        }
    }
    assert!(guest.cpu.state.is_reset_line_asserted());
    assert!(reference.paula.dma_active && reference.agnus.dskpt > 0x2000);
    assert!(!reference.floppy.pins.write_enable_high);
    assert!(reference
        .floppy
        .pins
        .incoming
        .is_some_and(|input| !input.write_enable_high));

    let before = serde_json::to_value(&reference.floppy.drives[0]).unwrap();
    assert_ne!(before["spindle_speed"], 0);
    assert_eq!(before["head_travel"][0], 24);
    assert_eq!(before["cylinder"], 23);
    assert_eq!(before["disk_change_flip_flop"], false);
    assert_ne!(before["spindle_fraction"], 0);
    assert_ne!(before["cell_phase"], 0);
    assert!(before["track_data"][46]["Recorded"]
        .as_array()
        .unwrap()
        .iter()
        .any(|block| block.get("Recorded").is_some()));
    let mut cold = reference.clone();
    cold.reset();
    cold.cpu.state.set_stopped(true);
    reference.reset_warm();
    reference.cpu.state.set_stopped(true);
    for machine in [&mut cold, &mut reference, &mut guest] {
        let released = floppy::FloppyPins {
            incoming: Some(floppy::FloppyInputs {
                control: 0xff,
                write_data_high: true,
                write_enable_high: true,
            }),
            ..Default::default()
        };
        assert_eq!(machine.floppy.pins, released);
        for _ in 0..12_000 {
            machine.step_cck();
            assert_eq!(machine.floppy.pins, released);
            assert!(!machine.paula.dma_active);
            assert_eq!(machine.agnus.dskpt, 0);
            assert_eq!(machine.paula.interrupts.intreq & 0x1002, 0);
        }
        assert_eq!(machine.paula.disk.buffered_words(), 0);
        let drive = &machine.floppy.drives[0];
        assert!(drive.connected && drive.disk_inserted && drive.write_protected);
        assert!(!drive.motor_on && !drive.selected && !drive.is_ready());
        let after = serde_json::to_value(drive).unwrap();
        assert!(
            before["track_data"] == after["track_data"],
            "reset changed the recorded medium"
        );
        for field in [
            "cylinder",
            "disk_change_flip_flop",
            "rotation_bit",
            "cell_phase",
            "spindle_fraction",
        ] {
            assert_eq!(before[field], after[field], "{field}");
        }
        assert_eq!(after["spindle_speed"], 0);
        assert!(after["head_travel"].is_null());
        assert_eq!(after["settle_cck"], 0);
        assert_eq!(after["read_recovery_cck"], 0);
    }
    assert_eq!(
        guest.cpu.state.instruction_pc(),
        common::continuation::BOOT_PC + 2
    );
}

#[test]
fn resetting_mid_seek_preserves_the_recording_and_fractional_angle_and_cancels_pending_work() {
    let mut controller = floppy::FloppyController::default();
    assert!(controller.drives[0].insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
    controller.pins.control_input = 0x7f;
    controller.step_cck();
    controller.pins.control_input = 0x75;
    controller.pins.write_enable_high = false;
    controller.pins.write_data_high = false;
    for _ in 0..9_567 {
        controller.step_cck();
    }
    controller.pins.control_input = 0x74;
    controller.step_cck();
    controller.pins.control_input = 0x75;
    controller.step_cck();
    controller.pins.incoming = Some(floppy::FloppyInputs {
        control: 0x74,
        write_data_high: false,
        write_enable_high: false,
    });
    controller.drives[0].write_protected = true;
    // Read the saved representation to observe fractional angle and recorded
    // media without adding debugger accessors to the drive's execution path.
    let before = serde_json::to_value(&controller.drives[0]).unwrap();
    assert!(before["head_travel"].is_array());
    assert_ne!(before["spindle_fraction"], 0);
    assert_ne!(before["cell_phase"], 0);
    assert!(before["track_data"][0]["Recorded"]
        .as_array()
        .unwrap()
        .iter()
        .any(|block| block.get("Recorded").is_some()));
    controller.reset();
    for _ in 0..12_000 {
        controller.sample_input_pins();
        controller.step_cck();
    }
    let drive = &controller.drives[0];
    let after = serde_json::to_value(drive).unwrap();
    assert!(drive.connected && drive.disk_inserted && drive.write_protected);
    for field in [
        "track_data",
        "cylinder",
        "disk_change_flip_flop",
        "spindle_fraction",
        "cell_phase",
        "rotation_bit",
    ] {
        assert_eq!(before[field], after[field], "{field}");
    }
    assert!(!drive.motor_on && !drive.selected && !drive.is_ready());
    assert_eq!(after["spindle_speed"], 0);
    assert!(after["head_travel"].is_null());
    assert_eq!(after["settle_cck"], 0);
    assert_eq!(after["read_recovery_cck"], 0);
    assert_eq!(controller.pins, floppy::FloppyPins::default());
}
