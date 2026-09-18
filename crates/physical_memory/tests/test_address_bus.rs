use physical_memory::{AddressBus, BusResult, PhysicalMemory};

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
