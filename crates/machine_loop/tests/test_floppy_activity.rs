//! Activity observes consumption, not the continuously rotating raw stream.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;
use common::MachineHarness;
use config::custom_reg;

#[test]
fn rotation_and_debug_peeks_stay_idle_but_live_byte_reads_count_for_selected_drive() {
    let mut harness = MachineHarness::new();
    let m = &mut harness.machine;
    assert!(m.floppy.set_connections([true; 4]).is_ok());
    for drive in &mut m.floppy.drives {
        assert!(drive.insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
    }
    common::cia::configure_drive_control_outputs(m);
    common::cia::write_byte(m, 0xbfd100, 0x7f);
    common::cia::write_byte(m, 0xbfd100, 0x6f); // DF1 selected, motor running.
    m.step_cycles(2000);
    assert!(m.floppy.drives[1].motor_on);
    assert_ne!(m.paula.peek_dskbytr() & 0x8000, 0);
    for _ in 0..2 {
        m.memory_bus()
            .read_custom_word_debug(custom_reg::paula::REGISTER_DSKBYTR);
        m.step_cycles(500);
    }
    assert!(m
        .floppy
        .drives
        .iter()
        .all(|drive| drive.activity_sequence == 0));
    let byte = m
        .memory_bus()
        .read_custom_word(custom_reg::paula::REGISTER_DSKBYTR);
    assert_ne!(byte & 0x8000, 0);
    m.step_cck();
    assert_eq!(m.floppy.drives[1].activity_sequence, 1);
    assert!(m
        .floppy
        .drives
        .iter()
        .enumerate()
        .all(|(index, drive)| index == 1 || drive.activity_sequence == 0));
    // An immediate second read acknowledges no fresh byte and adds no event.
    assert_eq!(
        m.memory_bus()
            .read_custom_word(custom_reg::paula::REGISTER_DSKBYTR)
            & 0x8000,
        0
    );
    m.step_cck();
    assert_eq!(m.floppy.drives[1].activity_sequence, 1);
}

#[test]
fn cpu_byte_consumption_is_visible_at_the_same_cck_boundary() {
    let mut harness = MachineHarness::new();
    let m = &mut harness.machine;
    assert!(m.floppy.drives[0].insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
    common::cia::configure_drive_control_outputs(m);
    common::cia::write_byte(m, 0xbfd100, 0x7f);
    common::cia::write_byte(m, 0xbfd100, 0x77);
    m.step_cycles(3);
    harness.load_cpu_code(0x1000, &[0x3039, 0x00df, 0xf01a, 0x60fe]); // MOVE.W DSKBYTR,D0
    let m = &mut harness.machine;
    // Present one byte without changing the live CPU/bus consumption path.
    m.paula.dskbytr = 0x8001;
    for _ in 0..100 {
        m.step_cck();
        if m.paula.dskbytr & 0x8000 == 0 {
            assert_eq!(m.floppy.drives[0].activity_sequence, 1);
            assert!(!m.paula.disk.activity_pending);
            return;
        }
    }
    panic!("CPU did not read the disk byte");
}

#[test]
fn serialized_write_words_report_activity_without_claiming_media_persistence() {
    let mut harness = MachineHarness::new();
    let m = &mut harness.machine;
    assert!(m.floppy.drives[0].insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
    common::cia::configure_drive_control_outputs(m);
    common::cia::write_byte(m, 0xbfd100, 0x7f);
    common::cia::write_byte(m, 0xbfd100, 0x77);
    m.step_cycles(3);
    assert!(m
        .memory_bus()
        .write_custom_word(custom_reg::agnus::REGISTER_DSKPTL, 0x2000, "DMA"));
    assert!(m
        .memory_bus()
        .write_custom_word(custom_reg::agnus::REGISTER_DMACON, 0x8210, "DMA"));
    assert!(m.paula.set_dsklen(0xc002));
    m.step_cycles(3);
    assert!(m.paula.set_dsklen(0xc002));
    m.step_cycles(3000);
    assert_eq!(m.floppy.drives[0].activity_sequence, 2);
    assert!(!m.paula.pins.disk_write_enable);
    m.step_cycles(1000);
    assert_eq!(m.floppy.drives[0].activity_sequence, 2);
}

#[test]
fn head_steps_survive_round_trip_and_counter_wrap_without_motor_or_media() {
    // Both steps happen between host publications and finish at the starting cylinder.
    let mut controller = floppy::FloppyController::default();
    controller.drives[0].activity_sequence = u64::MAX;
    for controls in [0xf5, 0xf4, 0xf7, 0xf6] {
        controller.pins.control_input = controls;
        controller.step_cck();
    }
    assert_eq!(controller.drives[0].cylinder, 0);
    assert_eq!(controller.drives[0].activity_sequence, 1);
    for _ in 0..100 {
        controller.step_cck();
    }
    assert_eq!(controller.drives[0].activity_sequence, 1);
    controller.reset();
    assert_eq!(controller.drives[0].activity_sequence, 1);
}
