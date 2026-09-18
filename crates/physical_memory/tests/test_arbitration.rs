use physical_memory::{
    arbitration::{function_code, BusAccessSize, BusResult},
    PhysicalMemory,
};

#[test]
fn test_bus_result_methods() {
    let ready: BusResult<u16> = BusResult::Ready(0x1234);
    assert!(ready.is_ready());
    assert!(!ready.is_wait());
    assert_eq!(ready.ok(), Some(0x1234));
    assert_eq!(ready.unwrap_or(0xFFFF), 0x1234);

    let wait: BusResult<u16> = BusResult::WaitState;
    assert!(!wait.is_ready());
    assert!(wait.is_wait());
    assert_eq!(wait.ok(), None);
    assert_eq!(wait.unwrap_or(0xFFFF), 0xFFFF);
}

#[test]
fn test_function_codes_and_access_size() {
    assert_eq!(function_code::USER_DATA, 1);
    assert_eq!(function_code::USER_PROGRAM, 2);
    assert_eq!(function_code::SUPERVISOR_DATA, 5);
    assert_eq!(function_code::SUPERVISOR_PROGRAM, 6);
    assert_eq!(function_code::CPU_SPACE, 7);

    let byte_sz = BusAccessSize::Byte;
    let word_sz = BusAccessSize::Word;
    assert_ne!(byte_sz, word_sz);
    assert_eq!(byte_sz, BusAccessSize::Byte);
}

#[test]
fn test_chip_ram_contention_and_fast_ram_immunity() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

    // Verify is_chip_ram_target identifying contended memory
    assert!(bus.is_chip_ram_target(0x001000)); // Chip RAM is contended
    assert!(bus.is_chip_ram_target(0xC00000)); // Slow RAM is contended on Agnus bus
    assert!(!bus.is_chip_ram_target(0x200000)); // Fast RAM is not contended

    // Initial unlocked state: Chip RAM accesses return Ready
    assert_eq!(bus.write_word(0x001000, 0xCAFE), BusResult::Ready(()));
    assert_eq!(bus.read_word(0x001000), BusResult::Ready(0xCAFE));

    // Lock Chip RAM (simulating Agnus/Blitter DMA channel activity)
    bus.chip_ram_blocked = true;
    assert!(bus.chip_ram_blocked);

    // Chip RAM and Slow RAM accesses stall with WaitState
    assert_eq!(bus.read_word(0x001000), BusResult::WaitState);
    assert_eq!(bus.write_word(0x001000, 0x1234), BusResult::WaitState);
    assert_eq!(bus.read_byte(0x001000), BusResult::WaitState);
    assert_eq!(bus.write_byte(0x001000, 0x12), BusResult::WaitState);
    assert_eq!(bus.read_word(0xC00000), BusResult::WaitState);

    // Fast RAM is completely immune to Chip RAM contention
    assert_eq!(bus.read_word(0x200000), BusResult::Ready(0xFFFF));

    // Unlock Chip RAM -> transfers resume
    bus.chip_ram_blocked = false;
    assert!(!bus.chip_ram_blocked);
    assert_eq!(bus.read_word(0x001000), BusResult::Ready(0xCAFE));
}
