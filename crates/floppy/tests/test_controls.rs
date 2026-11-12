use floppy::FloppyController;

fn control(floppy: &mut FloppyController, pins: u8) {
    floppy.pins.control_input = pins;
    floppy.step_cck();
}

#[test]
fn each_drive_latches_motor_on_selection_and_steps_only_on_edges() {
    for index in 0..4 {
        let mut floppy = FloppyController::new();
        let select = 1 << (index + 3);
        control(&mut floppy, 0x7f); // MOTOR setup before SELECT.
        control(&mut floppy, 0x7f & !select);
        assert!(floppy.drives[index].motor_on);
        assert!(floppy.drives[index].selected);
        control(&mut floppy, 0xff & !select); // MOTOR change alone is not latched.
        assert!(floppy.drives[index].motor_on);
        control(&mut floppy, 0xfd & !select); // Inward, STEP released.
        control(&mut floppy, 0xfc & !select);
        assert_eq!(floppy.drives[index].cylinder, 1);
        control(&mut floppy, 0xfc & !select);
        assert_eq!(floppy.drives[index].cylinder, 1);
        control(&mut floppy, 0xff & !select); // Outward.
        control(&mut floppy, 0xfe & !select);
        assert_eq!(floppy.drives[index].cylinder, 0);
        control(&mut floppy, 0xff);
        control(&mut floppy, 0xff & !select);
        assert!(!floppy.drives[index].motor_on);
    }
}

#[test]
fn selected_sense_lines_are_low_dominant_and_steps_clear_inserted_change() {
    let mut floppy = FloppyController::new();
    assert_eq!(floppy.sample_ciaa_port_a_inputs(), 0x3c);
    // Supplied status only: no image is loaded, read or written.
    floppy.drives[0].disk_inserted = true;
    floppy.drives[0].cylinder = 2;
    floppy.drives[1].write_protected = true;
    control(&mut floppy, 0x7f);
    control(&mut floppy, 0x67); // Drives 0 and 1 selected with MOTOR on.
    assert_eq!(floppy.sample_ciaa_port_a_inputs(), 0);
    control(&mut floppy, 0x66);
    assert!(!floppy.drives[0].disk_change_flip_flop);
    assert!(floppy.drives[1].disk_change_flip_flop);
    assert_eq!(floppy.drives[0].cylinder, 1);
    assert_eq!(floppy.drives[1].cylinder, 0);
}
