use floppy::{
    decode_amiga_sector, decode_mfm_long, encode_amiga_sector, encode_mfm_long, FloppyController,
    MfmError, FORMATTED_DISK_BYTES, RAW_MFM_SECTOR_BYTES, SECTORS_PER_TRACK, SECTOR_DATA_BYTES,
};

#[test]
fn test_mfm_longword_encode_decode_roundtrip() {
    let test_values = [
        0x1234_5678,
        0xAAAA_AAAA,
        0x5555_5555,
        0xFFFF_FFFF,
        0x0000_0000,
        0xDEAD_BEEF,
        0xCAFE_BABE,
    ];

    for &val in &test_values {
        let (odd, even) = encode_mfm_long(val);
        let decoded = decode_mfm_long(odd, even);
        assert_eq!(
            decoded, val,
            "MFM longword roundtrip failed for 0x{:08X}",
            val
        );
    }
}

#[test]
fn test_amiga_sector_encode_decode_roundtrip() {
    let mut payload = [0u8; 512];
    for i in 0..512 {
        payload[i] = (i & 0xFF) as u8;
    }

    let track = 20;
    let sector = 7;
    let raw = encode_amiga_sector(track, sector, &payload);

    assert_eq!(raw.len(), RAW_MFM_SECTOR_BYTES);
    // Magic sync words at bytes 0..3: $4489 $4489
    assert_eq!(raw[0], 0x44);
    assert_eq!(raw[1], 0x89);
    assert_eq!(raw[2], 0x44);
    assert_eq!(raw[3], 0x89);

    let decoded = decode_amiga_sector(&raw).expect("Sector decoding should succeed");
    assert_eq!(decoded.0, track, "Decoded track should match");
    assert_eq!(decoded.1, sector, "Decoded sector should match");
    assert_eq!(
        decoded.2, payload,
        "Decoded 512-byte payload should match original"
    );
}

#[test]
fn test_amiga_sector_checksum_validation() {
    let payload = [0x55u8; 512];
    let mut raw = encode_amiga_sector(5, 3, &payload);

    // Corrupt one byte in the data section (offset 600)
    raw[600] ^= 0xFF;
    let result = decode_amiga_sector(&raw);
    assert_eq!(
        result,
        Err(MfmError::DataChecksumMismatch),
        "Corrupted data should fail data checksum"
    );

    // Corrupt one byte in the header section (offset 10)
    let mut raw_header_corrupt = encode_amiga_sector(5, 3, &payload);
    raw_header_corrupt[10] ^= 0x01;
    let result_hdr = decode_amiga_sector(&raw_header_corrupt);
    assert_eq!(
        result_hdr,
        Err(MfmError::HeaderChecksumMismatch),
        "Corrupted header should fail header checksum"
    );
}

#[test]
fn test_amiga_track_dma_stream_and_sync() {
    let mut controller = FloppyController::new();
    let mut chip_ram = vec![0u8; 0x20000];

    // Create an 880 KB ADF image with recognizable sector data
    let mut adf_image = vec![0u8; FORMATTED_DISK_BYTES];
    for track in 0..160 {
        for sector in 0..SECTORS_PER_TRACK {
            let offset = (track * SECTORS_PER_TRACK + sector) * SECTOR_DATA_BYTES;
            adf_image[offset] = track as u8;
            adf_image[offset + 1] = sector as u8;
        }
    }

    // Insert disk into DF0:
    controller.drives[0].insert_disk(&adf_image);

    // Select DF0: via CIA-B Port B write (_SEL0 = 0, _MTR = 0)
    controller.handle_ciab_port_b_write(0b0111_0111); // Bit 3 (_SEL0) low, bit 7 (_MTR) low
    assert!(controller.drives[0].selected);
    assert!(controller.drives[0].motor_on);

    // Enable DMA in DMACON
    controller.set_dma_enabled(true);

    // Set ADKCON with WORDSYNC (bit 10 = $0400)
    controller.set_adkcon(0x0400);

    // Set DSKPT to 0x1000 in Chip RAM
    controller.set_dskpt(0x1000);

    // Arm and start DMA via 2-write DSKLEN sequence: transfer 100 words
    controller.set_dsklen(0x8000 | 100); // Write 1: arm
    assert!(controller.is_dma_armed());
    assert!(!controller.is_dma_active());

    controller.set_dsklen(0x8000 | 100); // Write 2: activate
    assert!(controller.is_dma_active());

    // Step DMA until transfer completes
    let mut cycles = 0;
    while controller.is_dma_active() && cycles < 1000 {
        controller.step_cck_ram(&mut chip_ram);
        cycles += 1;
    }

    assert!(!controller.is_dma_active(), "DMA should have completed");
    assert!(
        controller.wordsync_matched,
        "Sync word $4489 should have been matched"
    );
    assert!(
        controller.poll_dsksyn_irq(),
        "DSKSYN interrupt should have triggered"
    );
    assert!(
        controller.poll_dskblk_irq(),
        "DSKBLK completion interrupt should have triggered"
    );

    // Verify DSKPT advanced by 200 bytes (100 words)
    assert_eq!(controller.dskpt, 0x1000 + 200);

    // First word after sync in Chip RAM should be the MFM header word
    let word0 = u16::from_be_bytes([chip_ram[0x1000], chip_ram[0x1001]]);
    assert_ne!(word0, 0, "Chip RAM should contain streamed MFM data");
}

#[test]
fn test_dskbytr_clear_on_read() {
    let mut controller = FloppyController::new();
    let mut chip_ram = vec![0u8; 0x1000];

    let adf_image = vec![0x77u8; FORMATTED_DISK_BYTES];
    controller.drives[0].insert_disk(&adf_image);
    controller.handle_ciab_port_b_write(0b0111_0111);
    controller.set_dma_enabled(true);
    controller.set_dskpt(0x0200);

    // Read 2 words
    controller.set_dsklen(0x8002);
    controller.set_dsklen(0x8002);

    controller.step_cck_ram(&mut chip_ram);

    // DSKBYTR should have bit 15 (DSKBYT) set
    assert_ne!(
        controller.peek_dskbytr() & 0x8000,
        0,
        "Bit 15 should be set in peek"
    );
    assert_ne!(
        controller.peek_dskbytr() & 0x8000,
        0,
        "Peek should not clear bit 15"
    );

    let read_val = controller.read_dskbytr();
    assert_ne!(read_val & 0x8000, 0, "Read should return bit 15 set");

    // Read should have cleared bit 15
    assert_eq!(
        controller.peek_dskbytr() & 0x8000,
        0,
        "Bit 15 should be cleared after read"
    );
}

#[test]
fn test_dskbytr_composite_dmaon_and_diskwrite() {
    let mut controller = FloppyController::new();

    // Initially DMA disabled and read mode: bits 14 and 13 should be 0
    assert_eq!(controller.peek_dskbytr() & 0x6000, 0);

    // Enable DMA
    controller.set_dma_enabled(true);
    assert_eq!(controller.peek_dskbytr() & 0x4000, 0x4000); // DMAON

    // Set write mode in DSKLEN (bit 14)
    controller.set_dsklen(0x4000);
    assert_eq!(controller.peek_dskbytr() & 0x6000, 0x6000); // DMAON | DISKWRITE
}
