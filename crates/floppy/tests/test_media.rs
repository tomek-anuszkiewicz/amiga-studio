use floppy::{FloppyController, FORMATTED_DISK_BYTES};

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
    controller.pins.control_input = 0x7f;
    controller.step_cck();
    controller.pins.control_input = 0x77;
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
fn circular_track_has_exact_bit_rate_polarity_and_index_without_rounding_drift() {
    for standard in [config::VideoStandard::Pal, config::VideoStandard::Ntsc] {
        let mut controller = FloppyController::new(standard);
        assert!(controller.drives[0].insert_disk(&vec![0; FORMATTED_DISK_BYTES]));
        controller.pins.control_input = 0x7f;
        controller.step_cck();
        controller.pins.control_input = 0x77;
        let hz = u64::from(standard.color_clock_hz());
        let mut bits = Vec::new();
        let mut indices = Vec::new();
        let mut cells = 0;
        for tick in 1..=hz * 2 / 5 {
            controller.step_cck();
            let elapsed_cells = tick * 500_000 / hz;
            if elapsed_cells != cells {
                cells = elapsed_cells;
                bits.push(!controller.pins.read_data_high);
            } else {
                assert!(controller.pins.read_data_high, "pulse is one sampled CCK");
            }
            if !controller.pins.index_high {
                indices.push(tick);
            }
        }
        assert_eq!(bits.len(), 200_000);
        assert_eq!(indices, [hz / 5, hz * 2 / 5]);
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
    reference.pins.control_input = 0x7f;
    reference.step_cck();
    reference.pins.control_input = 0x77;
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
    for _ in 0..250 {
        reference.step_cck();
        switched.step_cck();
        assert_eq!(switched.pins.read_data_high, reference.pins.read_data_high);
    }
    while switched.pins.read_data_high {
        switched.step_cck();
    }
    switched.drives[0].eject_disk();
    switched.step_cck();
    assert!(switched.pins.read_data_high);
    assert!(switched.pins.index_high);
    assert!(!switched.drives[0].disk_inserted);
    assert!(!switched.drives[0].insert_disk(&[0; 12]));
    assert!(!switched.drives[0].disk_inserted);
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
