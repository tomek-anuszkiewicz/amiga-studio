use floppy::{FloppyController, FORMATTED_DISK_BYTES};

fn spin_to_index(controller: &mut FloppyController, standard: config::VideoStandard) {
    controller.pins.control_input = 0x7f;
    controller.step_cck();
    controller.pins.control_input = 0x77;
    let hz = standard.color_clock_hz();
    for _ in 0..hz.div_ceil(2) {
        controller.step_cck();
    }
    assert!(controller.drives[0].is_ready());
    let found_index = (0..hz / 5).any(|_| {
        let high = controller.pins.index_high;
        controller.step_cck();
        high && !controller.pins.index_high
    });
    assert!(found_index, "no INDEX crossing at full speed");
}

#[test]
fn independent_zero_sector_decodes_and_rejects_corrupt_checksums() {
    // Literal AmigaDOS sector: zero payload, FF00000B header, masked header XOR 4.
    // This fixture never calls the encoder being checked.
    let mut raw = vec![0xaa; 1088];
    raw[4..16].copy_from_slice(&[
        0x44, 0x89, 0x44, 0x89, 0x55, 0x2a, 0xaa, 0xa5, 0x55, 0x2a, 0xaa, 0xa9,
    ]);
    raw[16] = 0x2a;
    raw[55] = 0xa4;
    assert_eq!(floppy::decode_amiga_sector(&raw), Ok((0, 0, [0; 512])));
    let mut header = raw.clone();
    header[11] ^= 1;
    assert_eq!(
        floppy::decode_amiga_sector(&header),
        Err(floppy::MfmError::HeaderChecksumMismatch)
    );
    let mut data = raw.clone();
    data[64] ^= 1;
    assert_eq!(
        floppy::decode_amiga_sector(&data),
        Err(floppy::MfmError::DataChecksumMismatch)
    );
    let mut invalid = raw.clone();
    invalid[8] ^= 0x40;
    assert_eq!(
        floppy::decode_amiga_sector(&invalid),
        Err(floppy::MfmError::InvalidFormat)
    );
    assert_eq!(
        floppy::decode_amiga_sector(&raw[..100]),
        Err(floppy::MfmError::BufferTooShort)
    );
    // The leading preamble moves sync to byte four: a nearly complete body is short too.
    assert_eq!(
        floppy::decode_amiga_sector(&raw[..1087]),
        Err(floppy::MfmError::BufferTooShort)
    );
    raw[4..8].fill(0xaa);
    assert_eq!(
        floppy::decode_amiga_sector(&raw),
        Err(floppy::MfmError::SyncNotFound)
    );
}

#[test]
fn sector_and_gap_clock_joins_follow_a_final_payload_one() {
    let mut adf = vec![0; FORMATTED_DISK_BYTES];
    for sector in adf.chunks_exact_mut(512) {
        sector[511] = 1;
    }
    let standard = config::VideoStandard::Pal;
    let mut controller = FloppyController::new(standard);
    assert!(controller.drives[0].insert_disk(&adf));
    spin_to_index(&mut controller, standard);
    let hz = u64::from(standard.color_clock_hz());
    let mut bits = Vec::new();
    for tick in 1..=hz / 5 {
        controller.step_cck();
        if tick * 500_000 / hz != (tick - 1) * 500_000 / hz {
            bits.push(!controller.pins.read_data_high);
        }
    }
    let bytes: Vec<u8> = bits
        .chunks_exact(8)
        .map(|b| b.iter().fold(0, |v, one| (v << 1) | u8::from(*one)))
        .collect();
    assert_eq!(bytes.len(), 12_500);
    for sector in 1..=11 {
        assert_eq!(bytes[sector * 1088 - 1], 0xa9);
        assert_eq!(
            bytes[sector * 1088],
            0x2a,
            "preceding one suppresses next clock"
        );
    }
    assert_eq!(bytes[0], 0xaa, "gap-to-track wrap follows zero");
}

#[test]
fn electronic_reset_retains_mounted_media_and_head_position() {
    let mut controller = FloppyController::new(config::VideoStandard::Pal);
    assert!(controller.drives[0].insert_disk(&vec![0; FORMATTED_DISK_BYTES]));
    let inserted = controller.drives[0].clone();
    assert!(!controller.drives[0].insert_disk(&[0; 12]));
    assert_eq!(
        controller.drives[0], inserted,
        "invalid insertion retains usable media"
    );
    controller.drives[0].cylinder = 17;
    controller.drives[0].write_protected = true;
    controller.reset();
    assert!(controller.drives[0].disk_inserted);
    assert_eq!(controller.drives[0].cylinder, 17);
    assert!(controller.drives[0].write_protected);
}

#[test]
fn exchanging_prepared_disks_moves_write_protection_without_moving_the_head() {
    let mut controller = FloppyController::new(config::VideoStandard::Pal);
    controller
        .set_connections([true, true, false, false])
        .unwrap();
    assert!(controller.drives[0].insert_disk(&vec![0; FORMATTED_DISK_BYTES]));
    controller.drives[0].write_protected = true;
    controller.drives[0].cylinder = 17;
    controller.drives[1].cylinder = 4;

    let disk = controller.drives[0].replace_disk(None);
    assert!(disk.is_some());
    assert!(controller.drives[1].replace_disk(disk).is_none());
    assert!(!controller.drives[0].disk_inserted);
    assert!(controller.drives[1].disk_inserted && controller.drives[1].write_protected);
    assert_eq!(controller.drives[0].cylinder, 17);
    assert_eq!(controller.drives[1].cylinder, 4);
}

#[test]
fn circular_track_has_exact_bit_rate_polarity_and_index_without_rounding_drift() {
    for standard in [config::VideoStandard::Pal, config::VideoStandard::Ntsc] {
        let mut controller = FloppyController::new(standard);
        assert!(controller.drives[0].insert_disk(&vec![0; FORMATTED_DISK_BYTES]));
        spin_to_index(&mut controller, standard);
        let hz = u64::from(standard.color_clock_hz());
        let mut bits = Vec::new();
        let mut indices = Vec::new();
        let mut releases = Vec::new();
        let mut cells = 0;
        for tick in 1..=hz * 2 / 5 {
            let high = controller.pins.index_high;
            controller.step_cck();
            let elapsed_cells = tick * 500_000 / hz;
            if elapsed_cells != cells {
                cells = elapsed_cells;
                bits.push(!controller.pins.read_data_high);
            } else {
                assert!(controller.pins.read_data_high, "pulse is one sampled CCK");
            }
            if high && !controller.pins.index_high {
                indices.push(tick);
            } else if !high && controller.pins.index_high {
                releases.push(tick);
            }
        }
        assert_eq!(bits.len(), 200_000);
        assert_eq!(indices, [hz / 5, hz * 2 / 5]);
        let width = (hz * 4).div_ceil(1000);
        assert_eq!(releases, [width, hz / 5 + width]);
        assert_eq!(&bits[..100_000], &bits[100_000..]);
        let raw: Vec<u8> = bits[..100_000]
            .chunks_exact(8)
            .map(|b| b.iter().fold(0, |v, one| (v << 1) | u8::from(*one)))
            .collect();
        assert_eq!(
            &raw[..16],
            &[
                0xaa, 0xaa, 0xaa, 0xaa, 0x44, 0x89, 0x44, 0x89, 0x55, 0x2a, 0xaa, 0xa5, 0x55, 0x2a,
                0xaa, 0xa9
            ]
        );
        assert!(raw[11 * 1088..].iter().all(|byte| *byte == 0xaa));
        for sector in 0..11 {
            let start = sector * 1088;
            assert_eq!(&raw[start + 4..start + 8], &[0x44, 0x89, 0x44, 0x89]);
            // Each normal clock is the NOR of adjacent data bits, including joins.
            for bit in ((start + 8) * 8..(start + 1088) * 8).step_by(2) {
                assert_eq!(bits[bit], !(bits[bit - 1] || bits[bit + 1]));
            }
        }
    }
}

#[test]
fn deselection_and_side_changes_keep_spindle_phase_and_eject_releases_output() {
    let mut reference = FloppyController::new(config::VideoStandard::Pal);
    assert!(reference.drives[0].insert_disk(&vec![0; FORMATTED_DISK_BYTES]));
    spin_to_index(&mut reference, config::VideoStandard::Pal);
    for _ in 0..137 {
        reference.step_cck();
    }
    let mut switched = reference.clone();
    switched.pins.control_input = 0x7f;
    for _ in 0..91 {
        reference.step_cck();
        switched.step_cck();
        assert!(switched.pins.read_data_high);
        assert!(switched.drives[0].motor_on);
    }
    switched.pins.control_input = 0x73;
    switched.step_cck();
    reference.step_cck();
    switched.pins.control_input = 0x77;
    for _ in 0..355 {
        reference.step_cck();
        switched.step_cck();
    }
    for _ in 0..250 {
        reference.step_cck();
        switched.step_cck();
        assert_eq!(switched.pins.read_data_high, reference.pins.read_data_high);
    }
    while switched.pins.read_data_high {
        switched.step_cck();
        reference.step_cck();
    }
    switched.drives[0].eject_disk();
    switched.step_cck();
    reference.step_cck();
    assert!(switched.pins.read_data_high);
    assert!(switched.pins.index_high);
    assert!(!switched.drives[0].disk_inserted);
    assert!(!switched.drives[0].insert_disk(&[0; 12]));
    assert!(!switched.drives[0].disk_inserted);
    for _ in 0..3_500 {
        switched.step_cck();
        reference.step_cck();
    }
    assert!(switched.drives[0].insert_disk(&vec![0; FORMATTED_DISK_BYTES]));
    assert!(switched.drives[0].is_ready());
    assert!(
        switched.drives[0] == reference.drives[0],
        "reinsertion changed the speed or angular position"
    );
}

#[test]
fn sector_has_preamble_and_independent_masked_checksums() {
    let mut data = [0u8; 512];
    data[..4].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
    let sector = floppy::encode_amiga_sector(0, 0, &data);
    assert_eq!(sector.len(), 1088);
    assert_eq!(
        &sector[..8],
        &[0xaa, 0xaa, 0xaa, 0xaa, 0x44, 0x89, 0x44, 0x89]
    );
    // FF00000B odd/even data positions, independent of inserted clock bits.
    let longs: Vec<u32> = sector
        .chunks_exact(4)
        .map(|b| u32::from_be_bytes(b.try_into().unwrap()) & 0x5555_5555)
        .collect();
    assert_eq!(longs[2], 0x5500_0005);
    assert_eq!(longs[3], 0x5500_0001);
    // Stored checksums contain the XOR of encoded data positions, never clock bits.
    assert_eq!((longs[12] << 1) | longs[13], 0x0000_0004);
    assert_eq!((longs[14] << 1) | longs[15], 0x1104_5544);
}

#[test]
fn partial_write_crosses_index_and_preserves_every_other_angular_sample() {
    let standard = config::VideoStandard::Pal;
    let lap = standard.color_clock_hz() / 5;
    let mut original = FloppyController::new(standard);
    assert!(original.set_connections([true, true, false, false]).is_ok());
    let adf = vec![0; FORMATTED_DISK_BYTES];
    for drive in &mut original.drives[..2] {
        assert!(drive.insert_disk(&adf));
    }
    spin_to_index(&mut original, standard);
    for _ in 0..lap - 31 {
        original.step_cck();
    }
    let mut writable = original.clone();
    let mut protected = original.clone();
    protected.drives[0].write_protected = true;
    let mut empty = original.clone();
    empty.drives[0].eject_disk();
    // The gate opens and closes inside two different bytes around sector zero.
    let interval = 9..53;
    let pulses = [9, 22, 37, 51];
    let mut expected = Vec::new();
    for tick in 0..lap {
        original.step_cck();
        expected.push(!original.pins.read_data_high);
        for drive in [&mut writable, &mut protected, &mut empty] {
            drive.pins.write_enable_high = !interval.contains(&tick);
            drive.pins.write_data_high = !pulses.contains(&tick);
            drive.step_cck();
        }
        if tick == 30 {
            assert!(!writable.pins.index_high);
        }
    }
    assert!(empty.drives[0].insert_disk(&adf));
    assert_eq!(empty.drives[0], original.drives[0]);
    for (tick, baseline) in expected.into_iter().enumerate() {
        let tick = tick as u32;
        let replaced = if interval.contains(&tick) {
            pulses.contains(&tick)
        } else {
            baseline
        };
        writable.step_cck();
        protected.step_cck();
        assert_eq!(
            !writable.pins.read_data_high, replaced,
            "write sample {tick}"
        );
        assert_eq!(
            !protected.pins.read_data_high, baseline,
            "protected sample {tick}"
        );
    }
    assert_eq!(writable.drives[1], original.drives[1]);
    writable.pins.control_input = 0x73;
    original.pins.control_input = 0x73;
    for _ in 0..lap {
        writable.step_cck();
        original.step_cck();
        assert_eq!(writable.pins.read_data_high, original.pins.read_data_high);
    }
}

#[test]
fn each_step_travels_for_three_ms_and_the_last_head_settles_for_fifteen_ms() {
    let standard = config::VideoStandard::Pal;
    let mut start = FloppyController::new(standard);
    assert!(start.drives[0].insert_disk(&vec![0; FORMATTED_DISK_BYTES]));
    spin_to_index(&mut start, standard);
    start.pins.control_input = 0x75; // Set inward direction separately.
    for _ in 0..10 {
        start.step_cck();
    }
    let travel = 10_641;
    let settle = 53_204;
    for intervals in [
        &[][..],
        &[12_000, 12_000, 12_000],
        &[12_000, 18_000, 12_000],
    ] {
        let mut drive = start.clone();
        drive.pins.control_input = 0x74;
        for _ in 0..10 {
            drive.step_cck();
        }
        let mut elapsed = 0;
        for step in 0..=intervals.len() {
            drive.pins.control_input = 0x75; // Release STEP to start this movement.
            let duration = intervals.get(step).copied().unwrap_or(travel + settle);
            for tick in 1..=duration {
                drive.step_cck();
                elapsed += 1;
                assert_eq!(
                    drive.drives[0].cylinder as usize,
                    step + usize::from(tick >= travel)
                );
                assert!(drive.drives[0].is_ready());
                if tick < travel + settle {
                    assert!(drive.pins.read_data_high);
                }
                if step < intervals.len() && tick == duration - 10 {
                    drive.pins.control_input = 0x74;
                }
            }
        }
        assert_eq!(elapsed, intervals.iter().sum::<u32>() + travel + settle);
        assert!((0..40).any(|_| {
            drive.step_cck();
            !drive.pins.read_data_high
        }));
    }
}

#[test]
fn index_and_fractional_spindle_phase_continue_through_a_seek() {
    let standard = config::VideoStandard::Pal;
    let mut reference = FloppyController::new(standard);
    assert!(reference.drives[0].insert_disk(&vec![0; FORMATTED_DISK_BYTES]));
    spin_to_index(&mut reference, standard);
    reference.pins.control_input = 0x75; // Inward direction, STEP released.
    let ticks_to_index = 5_000;
    for _ in 0..standard.color_clock_hz() / 5 - ticks_to_index {
        reference.step_cck();
    }
    let mut stepped = reference.clone();
    stepped.pins.control_input = 0x74;
    let index_width = (standard.color_clock_hz() * 4).div_ceil(1_000);
    for tick in 1..=64_000 {
        if tick == 11 {
            stepped.pins.control_input = 0x75;
        }
        reference.step_cck();
        stepped.step_cck();
        let inside_index = (ticks_to_index..ticks_to_index + index_width).contains(&tick);
        assert_eq!(!stepped.pins.index_high, inside_index);
        assert_eq!(stepped.pins.index_high, reference.pins.index_high);
        if tick == ticks_to_index {
            assert_eq!(stepped.drives[0].cylinder, 0, "INDEX starts during travel");
        }
        if tick == ticks_to_index + index_width {
            assert_eq!(stepped.drives[0].cylinder, 1, "INDEX ends after arrival");
        }
    }
    // Only cylinder and CHANGE should differ once settling is complete.
    assert_eq!(reference.drives[0].cylinder, 0);
    assert!(reference.drives[0].disk_change_flip_flop);
    reference.drives[0].cylinder = 1;
    reference.drives[0].disk_change_flip_flop = false;
    assert!(
        stepped.drives[0] == reference.drives[0],
        "seek changed the final spindle phase or medium"
    );
}

#[test]
fn side_and_write_recovery_delay_only_the_read_signal() {
    let standard = config::VideoStandard::Pal;
    let mut drive = FloppyController::new(standard);
    assert!(drive.drives[0].insert_disk(&vec![0; FORMATTED_DISK_BYTES]));
    spin_to_index(&mut drive, standard);
    drive.pins.control_input = 0x73;
    for _ in 1..355 {
        // The switch is sampled on the first CCK.
        drive.step_cck();
        assert!(drive.pins.read_data_high);
        assert!(drive.drives[0].is_ready());
    }
    assert!((0..40).any(|_| {
        drive.step_cck();
        !drive.pins.read_data_high
    }));
    drive.pins.write_enable_high = false;
    for _ in 0..60 {
        drive.step_cck();
    }
    drive.pins.write_enable_high = true;
    for _ in 0..4_257 {
        // 1.2 ms from gate release.
        drive.step_cck();
        assert!(drive.pins.read_data_high);
        assert!(drive.drives[0].is_ready());
    }
    assert!((0..40).any(|_| {
        drive.step_cck();
        !drive.pins.read_data_high
    }));
}
