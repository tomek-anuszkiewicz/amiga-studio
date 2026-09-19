#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use physical_memory::{AddressBus, BusResult, PhysicalMemory};

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
fn test_chip_ram_contention_and_fast_ram_immunity() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

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

#[test]
fn test_address_bus_trait_word_and_byte_accesses() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

    let trait_bus: &mut dyn AddressBus = &mut bus;
    assert_eq!(trait_bus.write_word(0x002000, 0x1234), BusResult::Ready(()));
    assert_eq!(trait_bus.read_word(0x002000), BusResult::Ready(0x1234));
    assert_eq!(trait_bus.read_word_debug(0x002000), 0x1234);
    assert_eq!(trait_bus.write_byte(0x002000, 0x56), BusResult::Ready(()));
    assert_eq!(trait_bus.read_byte(0x002000), BusResult::Ready(0x56));
}

#[test]
fn test_address_bus_trait_arbitration_wait_states() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();
    bus.chip_ram_blocked = true;

    {
        let trait_bus: &mut dyn AddressBus = &mut bus;
        assert_eq!(trait_bus.read_word(0x001000), BusResult::WaitState);
        assert_eq!(trait_bus.write_word(0x001000, 0xCAFE), BusResult::WaitState);
        assert_eq!(trait_bus.read_byte(0x001000), BusResult::WaitState);
        assert_eq!(trait_bus.write_byte(0x001000, 0x42), BusResult::WaitState);

        // Non-intrusive debug read ignores bus lock
        assert_eq!(trait_bus.read_word_debug(0x001000), 0x0000);
    }

    // Unblocking restores Ready status
    bus.chip_ram_blocked = false;
    let trait_bus: &mut dyn AddressBus = &mut bus;
    assert_eq!(trait_bus.read_word(0x001000), BusResult::Ready(0x0000));
}
