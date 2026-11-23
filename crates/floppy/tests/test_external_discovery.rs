use floppy::FloppyController;

fn control(controller: &mut FloppyController, value: u8) {
    controller.pins.control_input = value;
    controller.step_cck();
}

fn read_external_id(controller: &mut FloppyController, select: u8) -> u32 {
    let mut identity = 0;
    for _ in 0..32 {
        control(controller, !select);
        identity = (identity << 1) | u32::from(controller.sample_ciaa_port_a_inputs() & 0x20 == 0);
        control(controller, 0xff);
    }
    identity
}

#[test]
fn stationary_external_dd_identifies_without_ever_loading_media() {
    let mut controller = FloppyController::default();
    assert!(controller
        .set_connections([true, true, false, false])
        .is_ok());

    // The motor-on selection initializes the connected drive without moving
    // the spindle because it has never held a disk.
    control(&mut controller, 0x7f);
    control(&mut controller, 0x6f); // Select DF1 with MOTOR on.
    assert!(controller.drives[1].motor_on);
    assert!(!controller.drives[1].disk_inserted);
    assert_eq!(controller.sample_ciaa_port_a_inputs() & 0x20, 0x20);
    control(&mut controller, 0x7f);
    control(&mut controller, 0xff);
    control(&mut controller, 0xef); // Select DF1 to latch MOTOR off.
    control(&mut controller, 0xff);
    assert!(!controller.drives[1].motor_on);
    assert!(!controller.drives[1].selected);

    // The Technical Reference Manual defines 32 inverted READY bits,
    // one per select/read/deselect cycle, with the motor off.
    assert_eq!(read_external_id(&mut controller, 0x10), 0xffff_ffff);
    assert!(!controller.drives[1].disk_inserted);
    assert_eq!(read_external_id(&mut controller, 0x20), 0x0000_0000);
}

#[test]
fn connected_empty_external_drives_identify_and_disconnected_drives_release_ready() {
    let mut controller = FloppyController::default();
    assert!(controller
        .set_connections([true, true, true, false])
        .is_ok());
    for (drive, inserted) in [(1, false), (2, true), (3, false)] {
        if controller.drives[drive].connected {
            assert!(controller.drives[drive].insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
        }
        let select = 1 << (drive + 3);
        // Technical Reference Manual: establish motor, select, deselect;
        // then establish motor off and select before reading the 32-bit ID.
        for value in [0x7f, 0x7f & !select] {
            control(&mut controller, value);
        }
        for _ in 0..20_000 {
            controller.step_cck();
        }
        if drive == 1 {
            controller.drives[drive].eject_disk(); // Read its ID while coasting without media.
        }
        for value in [0x7f, 0xff, !select, 0xff] {
            control(&mut controller, value);
        }
        let identity = read_external_id(&mut controller, select);
        assert_eq!(identity, if drive == 3 { 0 } else { u32::MAX });
        assert_eq!(controller.drives[drive].disk_inserted, inserted);
    }
}
