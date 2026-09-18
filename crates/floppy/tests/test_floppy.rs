use floppy::FloppyController;

#[test]
fn test_floppy_geometry_and_stepping() {
    let mut controller = FloppyController::new();

    let df0 = &mut controller.drives[0];
    assert!(df0.is_track0());

    df0.step_pulse(true); // Step inward to cylinder 1
    assert_eq!(df0.cylinder, 1);
    assert!(!df0.is_track0());

    df0.step_pulse(false); // Step outward to cylinder 0
    assert_eq!(df0.cylinder, 0);
    assert!(df0.is_track0());
}

#[test]
fn test_dskpt_address_masking() {
    let mut controller = FloppyController::new();
    controller.set_dskpt(0x0004_2000);
    assert_eq!(controller.dskpt, 0x0004_2000);
    // Upper byte masked to 24-bit
    controller.set_dskpt(0xFF04_2000);
    assert_eq!(controller.dskpt, 0x0004_2000);
}

#[test]
fn test_disk_change_flip_flop_behavior() {
    let mut controller = FloppyController::new();
    let df0 = &mut controller.drives[0];

    // 1. Empty drive: disk changed / missing
    assert!(df0.is_disk_changed());

    // 2. Insert disk: change flip-flop remains set until head receives a step pulse!
    let dummy_adf = [0u8; 512];
    df0.insert_disk(&dummy_adf);
    assert!(df0.is_disk_changed());

    // 3. Step pulse received with disk inserted: flip-flop clears
    df0.step_pulse(true);
    assert!(!df0.is_disk_changed());
    assert_eq!(df0.cylinder, 1);

    // 4. Eject disk: flip-flop trips again
    df0.eject_disk();
    assert!(df0.is_disk_changed());
}

#[test]
fn test_ciab_port_b_motor_latching_and_stepping() {
    let mut controller = FloppyController::new();

    // Initial CIA-B PRB state: all high ($FF, no drives selected, motor off)
    assert_eq!(controller.prev_ciab_prb, 0xFF);
    assert!(!controller.drives[0].selected);
    assert!(!controller.drives[0].motor_on);

    // Select DF0: (_SEL0 bit 3 = 0) with motor ON (_MTR bit 7 = 0) -> falling edge on _SEL0 latches motor ON
    // PRB = 0b0111_0111 = 0x77 (bits: _MTR=0, _SEL3..1=1, _SEL0=0, _SIDE=1, _DIR=1, _STEP=1)
    controller.handle_ciab_port_b_write(0x77);
    assert!(controller.drives[0].selected);
    assert!(controller.drives[0].motor_on);

    // Deselect DF0: (_SEL0 = 1, _MTR = 1). Drive remembers motor was latched ON!
    // PRB = 0xFF
    controller.handle_ciab_port_b_write(0xFF);
    assert!(!controller.drives[0].selected);
    assert!(controller.drives[0].motor_on); // Motor remains latched ON!

    // Reselect DF0: and step inward: _DIR=0 (inward), pulse _STEP from 1 to 0
    // PRB = 0b0111_0100 = 0x74 (_STEP = 0 falling edge)
    controller.handle_ciab_port_b_write(0x77); // stable setup
    controller.handle_ciab_port_b_write(0x74); // _STEP falling edge, _DIR=0 (inward)
    assert_eq!(controller.drives[0].cylinder, 1);

    // Turn motor OFF on DF0: by deselecting then selecting DF0: with _MTR = 1 (bit 7 = 1)
    controller.handle_ciab_port_b_write(0xFF); // Deselect DF0
    controller.handle_ciab_port_b_write(0xF7); // _SEL0 falling with _MTR = 1
    assert!(!controller.drives[0].motor_on);
}

#[test]
fn test_ciaa_port_a_sensing_inputs() {
    let mut controller = FloppyController::new();

    // No drive selected -> all sensing lines float high ($3C: bits 2..5 = 1)
    assert_eq!(controller.sample_ciaa_port_a_inputs(), 0x3C);

    // Select DF0: with motor ON, at Track 0, disk inserted and stepped
    controller.drives[0].insert_disk(&[0u8; 100]);
    controller.drives[0].step_pulse(false); // Clear flip-flop
    controller.drives[0].set_motor(true);
    controller.drives[0].selected = true;

    // Sample inputs:
    // _RDY (bit 5) = 0 (ready)
    // _TK0 (bit 4) = 0 (track 0)
    // _WPROT (bit 3) = 1 (writable)
    // _CHNG (bit 2) = 1 (verified present)
    // Result = 0b0000_1100 = 0x0C
    assert_eq!(controller.sample_ciaa_port_a_inputs(), 0x0C);
}
