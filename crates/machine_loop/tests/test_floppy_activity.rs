//! Chipset events retain drive attribution across publication and clock boundaries.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;
use common::MachineHarness;
use config::custom_reg;

#[test]
fn rotation_and_debug_peeks_add_no_events_but_live_byte_reads_count_for_selected_drive() {
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
    let baseline = m.paula.disk_activity.sequences;
    assert_eq!(baseline, [0, 1, 0, 0]);
    assert!(m.floppy.drives[1].motor_on);
    assert_ne!(m.paula.peek_dskbytr() & 0x8000, 0);
    for _ in 0..2 {
        m.memory_bus()
            .read_custom_word_debug(custom_reg::paula::REGISTER_DSKBYTR);
        m.step_cycles(500);
    }
    assert_eq!(m.paula.disk_activity.sequences, baseline);
    let byte = m
        .memory_bus()
        .read_custom_word(custom_reg::paula::REGISTER_DSKBYTR);
    assert_ne!(byte & 0x8000, 0);
    m.step_cck();
    assert_eq!(m.paula.disk_activity.sequences[1], baseline[1] + 1);
    assert_eq!(m.paula.disk_activity.sequences, [0, baseline[1] + 1, 0, 0]);
    // An immediate second read acknowledges no fresh byte and adds no event.
    assert_eq!(
        m.memory_bus()
            .read_custom_word(custom_reg::paula::REGISTER_DSKBYTR)
            & 0x8000,
        0
    );
    m.step_cck();
    assert_eq!(m.paula.disk_activity.sequences[1], baseline[1] + 1);
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
            assert_eq!(m.paula.disk_activity.sequences[0], 2);
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
    assert_eq!(m.paula.disk_activity.sequences[0], 3);
    assert!(!m.paula.pins.disk_write_enable);
    m.step_cycles(1000);
    assert_eq!(m.paula.disk_activity.sequences[0], 3);
}

#[test]
fn head_steps_survive_round_trip_and_counter_wrap_without_motor_or_media() {
    let mut harness = MachineHarness::new();
    let m = &mut harness.machine;
    common::cia::configure_drive_control_outputs(m);
    common::cia::write_byte(m, 0xbfd100, 0xf5);
    m.step_cycles(2);
    m.paula.disk_activity.sequences[0] = u64::MAX;
    for controls in [0xf4, 0xf7, 0xf6] {
        common::cia::write_byte(m, 0xbfd100, controls);
    }
    m.step_cycles(2);
    assert_eq!(m.floppy.drives[0].cylinder, 0);
    assert_eq!(m.paula.disk_activity.sequences, [1, 0, 0, 0]);
    m.step_cycles(100);
    assert_eq!(m.paula.disk_activity.sequences, [1, 0, 0, 0]);
    assert!(!m.floppy.drives[0].motor_on);
    assert!(!m.floppy.drives[0].disk_inserted);
}

#[test]
fn effective_controls_include_ddrb_release_and_timer_override_but_unchanged_writes_stay_silent() {
    let mut harness = MachineHarness::new();
    let m = &mut harness.machine;
    assert!(m.floppy.set_connections([true; 4]).is_ok());
    common::cia::configure_drive_control_outputs(m);
    common::cia::write_byte(m, 0xbfd100, 0xf7);
    m.step_cycles(2);
    assert_eq!(m.paula.disk_activity.sequences, [1, 0, 0, 0]);
    common::cia::write_byte(m, 0xbfd100, 0xf7);
    m.step_cycles(30);
    assert_eq!(m.paula.disk_activity.sequences, [1, 0, 0, 0]);
    common::cia::write_byte(m, 0xbfd300, 0xf7); // Release SEL0 via DDRB.
    m.step_cycles(2);
    assert_eq!(m.paula.disk_activity.sequences, [2, 0, 0, 0]);
    assert!(!m.floppy.drives[0].selected);
    common::cia::write_byte(m, 0xbfd400, 1);
    common::cia::write_byte(m, 0xbfd500, 0);
    common::cia::write_byte(m, 0xbfde00, 0x07); // Timer A overrides released PB6/SEL3.
    m.step_cycles(100);
    assert!(m.paula.disk_activity.sequences[3] > 0);
    assert_eq!(m.paula.disk_activity.sequences[1..3], [0, 0]);
}

fn consume_byte(m: &mut machine_loop::A500Machine) {
    m.paula.dskbytr = 0x8001;
    assert_ne!(
        m.memory_bus()
            .read_custom_word(custom_reg::paula::REGISTER_DSKBYTR)
            & 0x8000,
        0
    );
}

#[test]
fn transfers_follow_selection_between_events_and_reach_every_selected_drive() {
    let mut harness = MachineHarness::new();
    let m = &mut harness.machine;
    assert!(m.floppy.set_connections([true; 4]).is_ok());
    for drive in &mut m.floppy.drives {
        assert!(drive.insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
    }
    common::cia::configure_drive_control_outputs(m);
    common::cia::write_byte(m, 0xbfd100, 0x7f);
    common::cia::write_byte(m, 0xbfd100, 0x77);
    m.step_cycles(2);
    consume_byte(m);
    assert_eq!(m.paula.disk_activity.sequences, [2, 0, 0, 0]);
    common::cia::write_byte(m, 0xbfd100, 0x6f);
    m.step_cycles(2);
    consume_byte(m);
    assert_eq!(m.paula.disk_activity.sequences, [3, 2, 0, 0]);
    common::cia::write_byte(m, 0xbfd100, 0x67); // DF0 + DF1, both latched running.
    m.step_cycles(2);
    consume_byte(m);
    assert_eq!(m.paula.disk_activity.sequences, [5, 3, 0, 0]);
    assert_eq!(m.denise.frame_builder.frame_number, 0); // All events in one field.
}

#[test]
fn captured_selection_does_not_relabel_the_same_cck_cpu_read_and_snapshot_resumes_context() {
    let mut harness = MachineHarness::new();
    assert!(harness.machine.floppy.set_connections([true; 4]).is_ok());
    for drive in &mut harness.machine.floppy.drives[..2] {
        assert!(drive.insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
    }
    common::cia::configure_drive_control_outputs(&mut harness.machine);
    common::cia::write_byte(&mut harness.machine, 0xbfd100, 0x7f);
    common::cia::write_byte(&mut harness.machine, 0xbfd100, 0x77);
    harness.machine.step_cycles(2);
    let configured = harness.machine.save_state();
    let mut coincident = None;
    // Align a real CPU read with the CIA falling E edge rather than assuming
    // that a queued PRB write matures on an arbitrary CPU clock.
    for offset in 0..5 {
        harness.machine.load_state(&configured);
        for _ in 0..offset {
            harness.machine.step_subsystems_cck();
        }
        harness.load_cpu_code(0x1000, &[0x3039, 0x00df, 0xf01a, 0x60fe]);
        let initial = harness.machine.save_state();
        harness.machine.paula.dskbytr = 0x8001;
        let mut until_read = 0;
        while harness.machine.paula.dskbytr & 0x8000 != 0 {
            harness.machine.step_cck();
            until_read += 1;
            assert!(until_read < 100);
        }
        if harness.machine.cia_b.eclock_phase == 2 {
            coincident = Some((initial, until_read));
            break;
        }
    }
    let (initial, until_read) =
        coincident.expect("CPU read must align with one of the five E phases");
    harness.machine.load_state(&initial);
    harness.machine.step_cycles(until_read - 1);
    let m = &mut harness.machine;
    assert_eq!(m.cia_b.eclock_phase, 1);
    assert!(m.cia_b.set_prb(0x6f));
    m.paula.dskbytr = 0x8001;
    let before = m.paula.disk_activity.sequences;
    m.step_cck();
    assert_eq!(m.cia_b.port_b_output_pins(), 0x6f);
    assert_eq!(m.paula.dskbytr & 0x8000, 0);
    assert!(m.floppy.drives[0].selected); // New controls are still captured for n+1.
    assert!(!m.floppy.drives[1].selected);
    assert_eq!(
        m.paula.disk_activity.sequences,
        [before[0] + 2, before[1] + 1, 0, 0]
    );
    // Serialize before the pending connector delivery. Both observation context
    // and physical controls must resume identically without replaying an event.
    let state: machine_loop::A500State =
        serde_json::from_str(&serde_json::to_string(&m.save_state()).unwrap()).unwrap();
    let mut restored = m.clone();
    restored.load_state(&state);
    for machine in [&mut *m, &mut restored] {
        machine.step_subsystems_cck();
        consume_byte(machine);
    }
    assert_eq!(m.save_state(), restored.save_state());
    assert_eq!(m.paula.disk_activity.sequences[1], before[1] + 2);
}
