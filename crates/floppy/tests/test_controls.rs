use floppy::FloppyController;

fn control(floppy: &mut FloppyController, pins: u8) {
    floppy.pins.control_input = pins;
    floppy.step_cck();
}

fn advance(floppy: &mut FloppyController, ticks: u32) {
    for _ in 0..ticks {
        floppy.step_cck();
    }
}

fn pulse(floppy: &mut FloppyController, released: u8) {
    control(floppy, released);
    advance(floppy, 10);
    control(floppy, released & !1);
    advance(floppy, 10);
    control(floppy, released);
}

#[test]
fn each_drive_latches_motor_on_selection_and_steps_only_on_edges() {
    for index in 0..4 {
        let mut floppy = FloppyController::new(config::VideoStandard::Pal);
        assert!(floppy.set_connections([true; 4]).is_ok());
        let select = 1 << (index + 3);
        control(&mut floppy, 0x7f); // MOTOR setup before SELECT.
        control(&mut floppy, 0x7f & !select);
        assert!(floppy.drives[index].motor_on);
        assert!(floppy.drives[index].selected);
        control(&mut floppy, 0xff & !select); // MOTOR change alone is not latched.
        assert!(floppy.drives[index].motor_on);
        pulse(&mut floppy, 0xfd & !select);
        assert_eq!(floppy.drives[index].cylinder, 0);
        advance(&mut floppy, 10_641); // 3 ms PAL travel.
        assert_eq!(floppy.drives[index].cylinder, 1);
        advance(&mut floppy, 53_204); // 15 ms settle before reversing.
        assert_eq!(floppy.drives[index].cylinder, 1);
        pulse(&mut floppy, 0xff & !select);
        advance(&mut floppy, 10_641);
        assert_eq!(floppy.drives[index].cylinder, 0);
        control(&mut floppy, 0xff);
        control(&mut floppy, 0xff & !select);
        assert!(!floppy.drives[index].motor_on);
    }
}

#[test]
fn selected_sense_lines_are_low_dominant_and_steps_clear_inserted_change() {
    let mut floppy = FloppyController::new(config::VideoStandard::Pal);
    assert!(floppy.set_connections([true; 4]).is_ok());
    assert_eq!(floppy.sample_ciaa_port_a_inputs(), 0x3c);
    // Supplied status only: no image is loaded, read or written.
    floppy.drives[0].disk_inserted = true;
    floppy.drives[0].cylinder = 2;
    floppy.drives[1].write_protected = true;
    control(&mut floppy, 0x7f);
    control(&mut floppy, 0x67); // Drives 0 and 1 selected with MOTOR on.
    advance(
        &mut floppy,
        config::VideoStandard::Pal.color_clock_hz().div_ceil(2),
    );
    assert_eq!(floppy.sample_ciaa_port_a_inputs(), 0);
    control(&mut floppy, 0x66);
    advance(&mut floppy, 10_641);
    assert!(floppy.drives[0].is_disk_changed());
    assert_eq!(floppy.drives[0].cylinder, 2);
    control(&mut floppy, 0x67);
    assert!(!floppy.drives[0].is_disk_changed());
    assert_eq!(floppy.drives[0].cylinder, 2);
    advance(&mut floppy, 10_641);
    assert!(!floppy.drives[0].disk_change_flip_flop);
    assert!(floppy.drives[1].disk_change_flip_flop);
    assert_eq!(floppy.drives[0].cylinder, 1);
    assert_eq!(floppy.drives[1].cylinder, 0);
}

#[test]
fn releasing_step_acknowledges_change_then_track_zero_follows_head_arrival() {
    let mut floppy = FloppyController::default();
    assert!(floppy.drives[0].insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
    control(&mut floppy, 0xf5); // Select DF0 and establish inward direction.
    advance(&mut floppy, 10);
    control(&mut floppy, 0xf4);
    advance(&mut floppy, 10_650); // Keep STEP low for longer than one movement.
    assert_eq!(floppy.sample_ciaa_port_a_inputs() & 0x14, 0);
    assert_eq!(floppy.drives[0].cylinder, 0);

    control(&mut floppy, 0xf5);
    assert_eq!(floppy.sample_ciaa_port_a_inputs() & 0x14, 0x04);
    advance(&mut floppy, 10_641 - 2);
    assert_eq!(floppy.sample_ciaa_port_a_inputs() & 0x14, 0x04);
    assert_eq!(floppy.drives[0].cylinder, 0);
    floppy.step_cck();
    assert_eq!(floppy.sample_ciaa_port_a_inputs() & 0x14, 0x14);
    assert_eq!(floppy.drives[0].cylinder, 1);
    advance(&mut floppy, 53_204 + 10_641);
    assert_eq!(floppy.drives[0].cylinder, 1);
}

#[test]
fn stationary_outputs_follow_media_ownership_and_empty_seek_finishes_after_deselection() {
    let mut floppy = FloppyController::default();
    floppy.pins.read_data_high = false;
    floppy.pins.index_high = false;
    floppy.step_cck();
    assert!(!floppy.pins.read_data_high && !floppy.pins.index_high);

    assert!(floppy.drives[0].insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
    floppy.step_cck();
    assert!(floppy.pins.read_data_high && floppy.pins.index_high);
    floppy.drives[0].eject_disk();
    floppy.pins.read_data_high = false;
    floppy.pins.index_high = false;
    floppy.step_cck();
    assert!(floppy.pins.read_data_high && floppy.pins.index_high);

    control(&mut floppy, 0xf5); // Select DF0 with MOTOR off and set inward direction.
    advance(&mut floppy, 10);
    floppy.pins.read_data_high = false;
    floppy.pins.index_high = false;
    control(&mut floppy, 0xf4);
    advance(&mut floppy, 10);
    control(&mut floppy, 0xf5); // Release STEP to start travel.
    control(&mut floppy, 0xfd); // Deselect during travel.
    advance(&mut floppy, 10_641 - 3);
    assert_eq!(floppy.drives[0].cylinder, 0);
    floppy.step_cck();
    assert_eq!(floppy.drives[0].cylinder, 1);
    assert!(!floppy.drives[0].motor_on && !floppy.drives[0].selected);
    assert!(!floppy.pins.read_data_high && !floppy.pins.index_high);
}

#[test]
fn insertion_with_latched_motor_starts_from_rest_like_a_cold_mounted_disk() {
    let image = vec![0; floppy::FORMATTED_DISK_BYTES];
    for standard in [config::VideoStandard::Pal, config::VideoStandard::Ntsc] {
        let mut empty = FloppyController::new(standard);
        let mut mounted = FloppyController::new(standard);
        let spin_up = standard.color_clock_hz().div_ceil(2);
        control(&mut empty, 0x7f);
        control(&mut empty, 0x77);
        assert!(empty.drives[0].motor_on);
        for _ in 0..spin_up + 1 {
            empty.step_cck();
            assert_eq!(empty.sample_ciaa_port_a_inputs() & 0x20, 0x20);
            assert!(empty.pins.index_high && empty.pins.read_data_high);
        }
        assert!(empty.drives[0].insert_disk(&image));
        assert!(!empty.drives[0].is_ready());

        assert!(mounted.drives[0].insert_disk(&image));
        control(&mut mounted, 0xf7); // Select the mounted disk with MOTOR off.
        for _ in 0..spin_up {
            mounted.step_cck();
            assert!(!mounted.drives[0].is_ready());
            assert!(mounted.pins.index_high && mounted.pins.read_data_high);
        }
        control(&mut mounted, 0x7f);
        mounted.pins.control_input = 0x77;
        let mut saw_index_before_ready = false;
        for tick in 1..=spin_up {
            let index_was_high = empty.pins.index_high;
            empty.step_cck();
            mounted.step_cck();
            assert_eq!(empty.pins.read_data_high, mounted.pins.read_data_high);
            assert_eq!(empty.pins.index_high, mounted.pins.index_high);
            saw_index_before_ready |= tick < spin_up && index_was_high && !empty.pins.index_high;
            let ready_level = if tick == spin_up { 0 } else { 0x20 };
            assert_eq!(empty.sample_ciaa_port_a_inputs() & 0x20, ready_level);
            assert_eq!(mounted.sample_ciaa_port_a_inputs() & 0x20, ready_level);
        }
        assert!(saw_index_before_ready);
        control(&mut empty, 0x7f);
        assert_eq!(empty.sample_ciaa_port_a_inputs() & 0x20, 0x20);
        control(&mut empty, 0x77);
        assert_eq!(empty.sample_ciaa_port_a_inputs() & 0x20, 0);
    }
}

#[test]
fn cold_ready_reselection_and_restart_use_the_same_moving_spindle() {
    for standard in [config::VideoStandard::Pal, config::VideoStandard::Ntsc] {
        let mut drive = FloppyController::new(standard);
        assert!(drive.drives[0].insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
        control(&mut drive, 0x7f);
        control(&mut drive, 0x77);
        let start = standard.color_clock_hz().div_ceil(2);
        assert_eq!(drive.sample_ciaa_port_a_inputs() & 0x20, 0x20);
        let mut peer = drive.clone();
        peer.pins.control_input = 0x7f;
        let mut early_transitions = 0;
        for tick in 2..start {
            drive.step_cck();
            peer.step_cck();
            early_transitions += u32::from(!drive.pins.read_data_high);
            assert!(!drive.drives[0].is_ready());
            if tick == start / 2 {
                peer.pins.control_input = 0x77;
            }
        }
        drive.step_cck();
        peer.step_cck();
        assert_eq!(drive.sample_ciaa_port_a_inputs() & 0x20, 0);
        assert!(early_transitions > 0);
        assert_eq!(drive.drives[0], peer.drives[0]);
        control(&mut drive, 0xff);
        control(&mut drive, 0x77); // Latches the previous high MOTOR.
        assert_eq!(drive.sample_ciaa_port_a_inputs() & 0x20, 0x20);
        let coast = 4_000;
        let mut pulses = 0;
        for _ in 1..coast - 1 {
            drive.step_cck();
            pulses += usize::from(!drive.pins.read_data_high);
        }
        assert!(pulses > 0);
        control(&mut drive, 0x7f);
        drive.pins.control_input = 0x77;
        advance(&mut drive, coast * 25 / 4 - 1);
        assert!(!drive.drives[0].is_ready());
        drive.step_cck();
        assert!(drive.drives[0].is_ready());
        drive.drives[0].eject_disk();
        drive.step_cck();
        assert_eq!(drive.sample_ciaa_port_a_inputs() & 0x24, 0x20);
        assert!(drive.pins.read_data_high && drive.pins.index_high);
        pulse(&mut drive, 0x77); // Empty at track zero.
        assert!(drive.drives[0].is_disk_changed());
        assert!(drive.drives[0].insert_disk(&vec![0; floppy::FORMATTED_DISK_BYTES]));
        assert!(drive.drives[0].is_ready());
        control(&mut drive, 0x76); // Outward against the track-zero stop.
        advance(&mut drive, 10);
        assert_eq!(drive.sample_ciaa_port_a_inputs() & 0x14, 0);
        control(&mut drive, 0x77);
        assert_eq!(drive.sample_ciaa_port_a_inputs() & 0x14, 0x04);
        assert!(!drive.drives[0].is_disk_changed());
        assert!(drive.drives[0].is_track0());
    }
}
