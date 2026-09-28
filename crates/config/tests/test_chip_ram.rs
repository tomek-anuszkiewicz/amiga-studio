#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Unit tests for Chip RAM bus helpers (DMA word access, alignment, and wrapping)

use config::chip_ram;

#[test]
fn test_chip_ram_read_write_u16_aligned() {
    let mut ram = vec![0u8; 1024];

    chip_ram::write_u16(&mut ram, 0x100, 0x1234);
    assert_eq!(ram[0x100], 0x12);
    assert_eq!(ram[0x101], 0x34);

    let val = chip_ram::read_u16(&ram, 0x100);
    assert_eq!(val, 0x1234);
}

#[test]
fn test_chip_ram_word_alignment_enforcement() {
    let mut ram = vec![0u8; 1024];

    // Writing to an odd address must automatically clear bit 0 (word-align)
    chip_ram::write_u16(&mut ram, 0x101, 0xABCD);
    assert_eq!(ram[0x100], 0xAB);
    assert_eq!(ram[0x101], 0xCD);

    // Reading from odd address must also align to 0x100
    let val = chip_ram::read_u16(&ram, 0x101);
    assert_eq!(val, 0xABCD);
}

#[test]
fn test_chip_ram_wraparound_address_masking() {
    let mut ram = vec![0u8; 1024]; // mask is 1023 (0x3FF)

    // Write at address exceeding RAM size (0x1000 + 0x20 = 0x1020 -> wraps to 0x20)
    chip_ram::write_u16(&mut ram, 0x1020, 0xCAFE);
    assert_eq!(ram[0x020], 0xCA);
    assert_eq!(ram[0x021], 0xFE);

    let val = chip_ram::read_u16(&ram, 0x2020);
    assert_eq!(val, 0xCAFE);
}

#[test]
fn test_chip_ram_empty_slice_graceful_fallback() {
    let mut empty: Vec<u8> = Vec::new();

    // Read on empty slice returns open bus floating $FFFF
    assert_eq!(chip_ram::read_u16(&empty, 0x00), 0xFFFF);

    // Writes to empty slice must not panic
    chip_ram::write_u16(&mut empty, 0x00, 0x1234);
}
