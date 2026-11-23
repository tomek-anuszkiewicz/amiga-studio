use floppy::FloppyController;

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
        for control in [0x7f, 0x7f & !select] {
            controller.pins.control_input = control;
            controller.step_cck();
        }
        for _ in 0..20_000 {
            controller.step_cck();
        }
        if drive == 1 {
            controller.drives[drive].eject_disk(); // Read its ID while coasting without media.
        }
        for control in [0x7f, 0xff, 0xff & !select, 0xff] {
            controller.pins.control_input = control;
            controller.step_cck();
        }
        let mut identity = 0u32;
        for _ in 0..32 {
            controller.pins.control_input = 0xff & !select;
            controller.step_cck();
            identity =
                (identity << 1) | u32::from(controller.sample_ciaa_port_a_inputs() & 0x20 == 0);
            controller.pins.control_input = 0xff;
            controller.step_cck();
        }
        assert_eq!(identity, if drive == 3 { 0 } else { u32::MAX });
        assert_eq!(controller.drives[drive].disk_inserted, inserted);
    }
}
