//! Autonomous ADF transitions through Paula, DMAL, Agnus and Chip RAM.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use common::MachineHarness;
use config::{custom_reg, A500Preset, VideoStandard};
use machine_loop::{A500Machine, A500State};

fn reading(standard: VideoStandard, side: u8, words: u16) -> A500Machine {
    let mut harness = MachineHarness::new_with_preset(A500Preset::Standard1Mb);
    if standard == VideoStandard::Ntsc {
        harness.machine = A500Machine::new(config::A500Config::standard_1mb(standard));
        harness.machine.physical_memory.map_chip_ram_to_low_memory();
    }
    // CPU contends for Chip RAM while incrementing D0.
    harness.load_cpu_code(0x10000, &[0x7000, 0x5280, 0x60fc]);
    let m = &mut harness.machine;
    assert!(m.floppy.drives[0].insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
    common::cia::configure_drive_control_outputs(m);
    assert!(m
        .memory_bus()
        .write_custom_word(custom_reg::agnus::REGISTER_DSKPTL, 0x2000));
    assert!(m.paula.set_dsksync(0x4489));
    assert!(m.paula.set_adkcon(0x8500));
    assert!(m
        .memory_bus()
        .write_custom_word(custom_reg::agnus::REGISTER_DMACON, 0x8210));
    assert!(m.paula.set_dsklen(0x8000 | words));
    m.step_cycles(3);
    assert!(!m.paula.dma_active);
    assert!(m.paula.set_dsklen(0x8000 | words));
    m.step_cycles(3);
    assert!(m.paula.dma_active);
    assert_ne!(m.paula.peek_dskbytr() & 0x4000, 0);
    common::cia::write_byte(m, 0xbfd100, 0x7f);
    common::cia::write_byte(m, 0xbfd100, if side == 0 { 0x77 } else { 0x73 });
    harness.machine
}

fn complete(machine: &mut A500Machine) -> Vec<(u64, u16)> {
    let mut transfers = Vec::new();
    let mut pointer = machine.agnus.dskpt;
    for _ in 0..800_000 {
        machine.step_cck();
        if machine.agnus.dskpt != pointer {
            assert_eq!(machine.agnus.dskpt, pointer + 2);
            transfers.push((machine.cck, machine.physical_memory.read_chip_word(pointer)));
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
            let mut m = reading(standard, side, 12);
            let transfers = complete(&mut m);
            let actual: Vec<_> = transfers.iter().map(|(_, word)| *word).collect();
            // FF track 00 0B, split odd/even; clocks carried from sync 4489.
            let even_track = if side == 0 { 0xaaa9 } else { 0x2aa9 };
            let expected = [
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
            assert_eq!(m.agnus.dskpt, 0x2018);
            assert_eq!(m.physical_memory.read_chip_word(0x2018), 0);
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
fn raw_dma_crosses_track_gap_and_wrap_without_restarting_the_spindle() {
    let mut m = reading(VideoStandard::Pal, 0, 6262);
    while m.agnus.dskpt < 0x2004 {
        m.step_cck();
    }
    // Keep the acquired alignment, then retain subsequent sync words as raw data.
    assert!(m.paula.set_adkcon(0x0400));
    complete(&mut m);
    let word = |i: u32| m.physical_memory.read_chip_word(0x2000 + i * 2);
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
    assert_eq!(m.agnus.dskpt, 0x2000 + 6262 * 2);
}

#[test]
fn guest_reset_instruction_retains_the_disk_and_mechanical_head() {
    let mut m = common::continuation::rom_machine(&[0x4e70, 0x60fe]);
    assert!(m.floppy.drives[0].insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
    m.floppy.drives[0].cylinder = 23;
    m.floppy.drives[0].motor_on = true;
    for _ in 0..500 {
        m.step_cck();
    }
    assert!(m.floppy.drives[0].disk_inserted);
    assert_eq!(m.floppy.drives[0].cylinder, 23);
    assert!(!m.floppy.drives[0].motor_on);
    assert_eq!(
        m.cpu.state.instruction_pc(),
        common::continuation::BOOT_PC + 2
    );
}

#[test]
fn reset_cancels_active_dma_but_preserves_media_without_false_completion() {
    let mut m = reading(VideoStandard::Pal, 0, 24);
    while m.agnus.dskpt < 0x2006 {
        m.step_cck();
    }
    m.reset_warm();
    m.cpu.state.set_stopped(true);
    assert!(m.floppy.drives[0].disk_inserted);
    assert!(!m.floppy.drives[0].motor_on);
    m.step_cycles(1000);
    assert_eq!(m.agnus.dskpt, 0);
    assert!(!m.paula.dma_active);
    assert_eq!(m.paula.disk.buffered_words(), 0);
    assert_eq!(m.paula.interrupts.intreq & 0x1002, 0);
}
