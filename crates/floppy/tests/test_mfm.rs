#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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

    // Set DSKPT to 0x1000 in Chip RAM
    controller.dskpt = 0x1000;

    // Arm and start DMA: transfer 100 words with WORDSYNC ($0400) and sync word $4489
    let mut dsklen = 0x8000 | 100;
    let mut dma_active = true;

    // Step DMA until transfer completes
    let mut cycles = 0;
    while dma_active && cycles < 1000 {
        controller.step_cck_ram(&mut chip_ram, 0x0400, 0x4489, &mut dsklen, &mut dma_active);
        cycles += 1;
    }

    assert!(!dma_active, "DMA should have completed");
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
    controller.dskpt = 0x0200;

    // Read 2 words
    let mut dsklen = 0x8002;
    let mut dma_active = true;

    controller.step_cck_ram(&mut chip_ram, 0x0000, 0x4489, &mut dsklen, &mut dma_active);

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
fn test_dskbytr_peek_and_read_data_bits() {
    let mut controller = FloppyController::new();
    controller.dskbytr = 0x90A5; // DSKBYT | WORDEQUAL | byte 0xA5

    assert_eq!(controller.peek_dskbytr(), 0x90A5);
    assert_eq!(controller.read_dskbytr(), 0x90A5);
    // Bit 15 cleared after read
    assert_eq!(controller.peek_dskbytr(), 0x10A5);
}
