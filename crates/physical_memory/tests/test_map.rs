use physical_memory::{BusResult, MemoryBank, PhysicalMemory};

#[test]
fn test_bank_map_classification() {
    let bus = PhysicalMemory::new();

    // Verify bank classifications across 24-bit physical address space
    assert_eq!(bus.bank_map[0x00].bank, MemoryBank::KickstartRom); // Default overlay
    assert_eq!(bus.bank_map[0x20].bank, MemoryBank::OpenBus);
    assert_eq!(bus.bank_map[0xC0].bank, MemoryBank::SlowRam);
    assert_eq!(bus.bank_map[0xDF].bank, MemoryBank::CustomChips);
    assert_eq!(bus.bank_map[0xBF].bank, MemoryBank::Cia);
    assert_eq!(bus.bank_map[0xFC].bank, MemoryBank::KickstartRom);
}

#[test]
fn test_open_bus_floating_lines() {
    let bus = PhysicalMemory::new();

    // Standard unmapped memory addresses float high ($FF for byte, $FFFF for word)
    assert_eq!(bus.read_byte_debug(0x200000), 0xFF);
    assert_eq!(bus.read_word_debug(0x200000), 0xFFFF);
    assert_eq!(bus.read_byte_debug(0x500000), 0xFF);
    assert_eq!(bus.read_word_debug(0x500000), 0xFFFF);
}

#[test]
fn test_boot_overlay_mechanics() {
    let mut bus = PhysicalMemory::new();

    // Inject custom Kickstart ROM signature
    let mut rom = vec![0x00; 256 * 1024];
    rom[0] = 0x11;
    rom[1] = 0x22;
    bus.inject_kickstart_rom(&rom);

    // Initial state: overlay is engaged
    assert!(bus.is_low_memory_overlay_active());
    assert_eq!(bus.read_word_debug(0x000000), 0x1122);

    // Disengage overlay via map_chip_ram_to_low_memory
    bus.map_chip_ram_to_low_memory();
    assert!(!bus.is_low_memory_overlay_active());

    // Address $000000 now accesses physical Chip RAM
    let _ = bus.write_word(0x000000, 0x9988);
    assert_eq!(bus.read_word_debug(0x000000), 0x9988);

    // Re-engage overlay
    bus.map_kickstart_to_low_memory();
    assert!(bus.is_low_memory_overlay_active());
    assert_eq!(bus.read_word_debug(0x000000), 0x1122);
}

#[test]
fn test_chip_ram_bounds_and_open_bus() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

    // Write at physical Chip RAM offset $1000
    assert_eq!(bus.write_word(0x001000, 0x55AA), BusResult::Ready(()));
    assert_eq!(bus.read_word(0x001000), BusResult::Ready(0x55AA));

    // Beyond 512KB ($080000..$1FFFFF, bank 0x08+), standard unexpanded machine has open bus
    assert_eq!(bus.read_word(0x081000), BusResult::Ready(0xFFFF));
}
