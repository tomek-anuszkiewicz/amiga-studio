#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use physical_memory::{BusResult, KickstartSizeError, PhysicalMemory};

#[test]
fn rom_loading_rejects_other_sizes_before_writing() {
    let mut memory = PhysicalMemory::new();
    let original = memory.clone();
    for size in [0, 262143, 262145, 524288] {
        for address in [0xF80000, 0xFC0000, 0x1FC0000] {
            assert_eq!(
                memory.write_bytes_debug(address, &vec![0x55; size]),
                Err(KickstartSizeError { actual: size })
            );
            assert_eq!(memory, original);
        }
    }
    // A load beginning before the ROM window must not bypass validation.
    assert!(memory
        .write_bytes_debug(0xF7FFFF, &vec![0; 524288])
        .is_err());
    assert_eq!(memory, original);
}

#[test]
fn fixed_rom_is_mirrored_and_debug_patches_never_resize_it() {
    let mut memory = PhysicalMemory::new();
    let mut rom = vec![0; 262144];
    rom[..2].copy_from_slice(&[0x12, 0x34]);
    rom[262143] = 0x56;
    assert_eq!(memory.write_bytes_debug(0xFC0000, &rom), Ok(262144));
    for address in [0, 0x40000, 0xF80000, 0xFC0000] {
        assert_eq!(memory.read_byte(address), BusResult::Ready(0x12));
        assert_eq!(memory.read_word(address), BusResult::Ready(0x1234));
    }
    memory.write_word_debug(0xFFFFFF, 0xABCD);
    assert_eq!(memory.read_word_debug(0xFBFFFF), 0xABCD);
    assert_eq!(memory.read_byte_debug(0), 0xCD);
    memory.write_byte_debug(0xFC0001, 0xEF);
    assert_eq!(memory.read_word_debug(0xF80000), 0xCDEF);
    assert_eq!(memory.write_word(0xFC0000, 0), BusResult::Ready(()));
    assert_eq!(memory.read_word_debug(0xFC0000), 0xCDEF);
    memory.reset_warm();
    assert_eq!(memory.read_word_debug(0x40000), 0xCDEF);
    assert_eq!(memory.kickstart_rom().len(), 262144);
}
