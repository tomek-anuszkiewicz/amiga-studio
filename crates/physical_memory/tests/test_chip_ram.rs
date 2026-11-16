#![allow(clippy::unwrap_used)]

//! DMA word storage contracts, migrated from the retired raw-slice helpers.
use config::chip_ram::{ChipRamAccess, ChipRamOwner};
use physical_memory::PhysicalMemory;

#[test]
fn test_chip_ram_read_write_u16_aligned() {
    let mut ram = PhysicalMemory::new();
    ram.begin_cck(1, 0);
    ram.write_chip_word(0x100, 0x1234, ChipRamOwner::Copper);
    assert_eq!(ram.read_chip_byte_debug(0x100), 0x12);
    assert_eq!(ram.read_chip_byte_debug(0x101), 0x34);
    ram.begin_cck(2, 0);
    assert_eq!(ram.read_chip_word(0x100, ChipRamOwner::Copper), 0x1234);
}

#[test]
fn test_chip_ram_word_alignment_enforcement() {
    let mut ram = PhysicalMemory::new();
    ram.begin_cck(1, 0);
    ram.write_chip_word(0x101, 0xabcd, ChipRamOwner::Blitter);
    assert_eq!(ram.read_chip_byte_debug(0x100), 0xab);
    assert_eq!(ram.read_chip_byte_debug(0x101), 0xcd);
    ram.begin_cck(2, 0);
    assert_eq!(ram.read_chip_word(0x101, ChipRamOwner::Blitter), 0xabcd);
}

#[test]
fn test_chip_ram_wraparound_address_masking() {
    let mut ram = PhysicalMemory::new();
    // The supported storage is 512 KiB; both DMA addresses wrap to $20.
    ram.begin_cck(1, 0);
    ram.write_chip_word(0x80020, 0xcafe, ChipRamOwner::Disk);
    assert_eq!(ram.read_chip_byte_debug(0x20), 0xca);
    assert_eq!(ram.read_chip_byte_debug(0x21), 0xfe);
    ram.begin_cck(2, 0);
    assert_eq!(ram.read_chip_word(0x100020, ChipRamOwner::Disk), 0xcafe);
}

#[test]
fn test_ocs_dma_cannot_address_above_lower_512k_even_with_larger_buffer() {
    // Preserve the original oversized-buffer fixture through serialization.
    // This isolates the OCS address limit from backing length; it does not add
    // a supported one-megabyte Chip RAM configuration.
    let mut image = serde_json::to_value(PhysicalMemory::new()).unwrap();
    image["chip_ram"] = serde_json::json!(vec![0u8; 1024 * 1024]);
    let mut ram: PhysicalMemory = serde_json::from_value(image).unwrap();
    ram.begin_cck(1, 0);
    ram.write_chip_word(0x80020, 0xcafe, ChipRamOwner::Audio(0));
    assert_eq!(ram.read_chip_byte_debug(0x20), 0xca);
    assert_eq!(ram.read_chip_byte_debug(0x21), 0xfe);
    assert_eq!(ram.read_chip_byte_debug(0x80020), 0);
    assert_eq!(ram.read_chip_byte_debug(0x80021), 0);
    ram.begin_cck(2, 0);
    assert_eq!(ram.read_chip_word(0x80020, ChipRamOwner::Audio(0)), 0xcafe);
}
