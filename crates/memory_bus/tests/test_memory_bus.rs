use memory_bus::{BusResult, MemoryBus};

#[test]
fn test_boot_overlay_and_cia_control() {
    let mut bus = MemoryBus::new();
    // Inject custom Kickstart ROM byte at offset 0
    let mut rom = vec![0x00; 256 * 1024];
    rom[0] = 0x12;
    rom[1] = 0x34;
    bus.inject_kickstart_rom(&rom);

    // Initial state: low-memory overlay active -> $000000 reads from Kickstart
    assert!(bus.is_low_memory_overlay_active());
    assert_eq!(bus.read_word_debug(0x000000), 0x1234);

    // Write to Chip RAM while overlay is active should not corrupt ROM
    bus.write_word_debug(0x000000, 0x5678);
    assert_eq!(bus.read_word_debug(0x000000), 0x1234);

    // Disengage overlay (simulate CIA-A Port A bit 0 write of 1 to $BFE001)
    bus.write_byte_debug(0xBFE001, 0x01);
    assert!(!bus.is_low_memory_overlay_active());

    // Now $000000 reads physical Chip RAM
    bus.write_word_debug(0x000000, 0xABCD);
    assert_eq!(bus.read_word_debug(0x000000), 0xABCD);

    // Re-engage overlay (write 0 to bit 0 of $BFE001)
    bus.write_byte_debug(0xBFE001, 0x00);
    assert!(bus.is_low_memory_overlay_active());
    assert_eq!(bus.read_word_debug(0x000000), 0x1234);
}

#[test]
fn test_chip_ram_contention_and_direct_rw() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    // 1. Direct word write and read
    assert_eq!(bus.write_word(0x001000, 0xCAFE), BusResult::Ready(()));
    assert_eq!(bus.read_word(0x001000), BusResult::Ready(0xCAFE));

    // 2. Direct byte write and read
    assert_eq!(bus.write_byte(0x001002, 0x42), BusResult::Ready(()));
    assert_eq!(bus.read_byte(0x001002), BusResult::Ready(0x42));

    // 3. Contention query: Chip RAM and Slow RAM
    assert_eq!(bus.read_word(0x001000), BusResult::Ready(0xCAFE));
    assert_eq!(bus.read_word(0xC00000), BusResult::Ready(0x0000));
    assert_eq!(bus.read_word(0x200000), BusResult::Ready(0xFFFF)); // Fast RAM open bus returns 0xFFFF

    bus.lock_chip_ram();
    assert!(bus.is_chip_ram_locked());
    assert_eq!(bus.read_word(0x001000), BusResult::WaitState);
    assert_eq!(bus.write_word(0x001000, 0x1234), BusResult::WaitState);
    assert_eq!(bus.read_word(0xC00000), BusResult::WaitState);
    assert_eq!(bus.write_word(0xC00000, 0x1234), BusResult::WaitState);
    assert_eq!(bus.read_word(0x200000), BusResult::Ready(0xFFFF)); // Fast RAM unaffected

    bus.unlock_chip_ram();
    assert!(!bus.is_chip_ram_locked());
    assert_eq!(bus.read_word(0x001000), BusResult::Ready(0xCAFE));
}

#[test]
fn test_floating_bus_and_tas_quirk() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    // Unmapped address ($150000) returns $FF / $FFFF
    assert_eq!(bus.read_byte_debug(0x150000), 0xFF);
    assert_eq!(bus.read_word_debug(0x150000), 0xFFFF);

    // TAS Quirk: Chip RAM write is dropped
    bus.write_byte_debug(0x000100, 0x00);
    bus.write_tas_byte(0x000100, 0x80);
    assert_eq!(bus.read_byte_debug(0x000100), 0x00); // unmodified

    // In Fast RAM, TAS write succeeds
    bus.load_test_ram(&[[0x200100, 0x00]]);
    bus.write_tas_byte(0x200100, 0x80);
    assert_eq!(bus.read_byte_debug(0x200100), 0x80);
}

#[test]
fn test_configurable_unmapped_byte_default_ff_and_test_mode() {
    // 1. Real emulator mode: unmapped memory defaults to 0xFF (open bus floating high)
    let mut real_bus = MemoryBus::new();
    assert_eq!(real_bus.unmapped_byte(), 0xFF);
    assert_eq!(real_bus.read_byte_debug(0x180000), 0xFF);
    assert_eq!(real_bus.read_word_debug(0x180000), 0xFFFF);

    // Can reconfigure open bus default if needed
    real_bus.set_unmapped_byte(0xAA);
    assert_eq!(real_bus.read_byte_debug(0x180000), 0xAA);
    assert_eq!(real_bus.read_word_debug(0x180000), 0xAAAA);

    // 2. Test harness mode: MemoryBus::new_test() defaults to 0xFF, configurable to 0x00 for flat test RAM
    let mut test_bus = MemoryBus::new_test();
    assert_eq!(test_bus.unmapped_byte(), 0xFF);
    test_bus.load_test_ram(&[[0x1000, 0x42]]);
    assert_eq!(test_bus.read_byte_debug(0x1000), 0x42);
    // Unpopulated address in test memory with default 0xFF
    assert_eq!(test_bus.read_byte_debug(0x2000), 0xFF);

    // Switch to flat RAM model (SingleStepTests)
    test_bus.set_unmapped_byte(0x00);
    assert_eq!(test_bus.read_byte_debug(0x2000), 0x00);
    assert_eq!(test_bus.read_word_debug(0x2000), 0x0000);
    assert_eq!(test_bus.read_byte_debug(0x1000), 0x42); // populated untouched
}

#[test]
fn test_bus_direct_read_write_and_byte_accesses() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    // 1. Direct word write and read
    assert_eq!(bus.write_word(0x004000, 0x1234), BusResult::Ready(()));
    assert_eq!(bus.read_word(0x004000), BusResult::Ready(0x1234));

    // 2. Direct byte reads (Upper byte at even address vs Lower byte at odd address)
    assert_eq!(bus.read_byte(0x004000), BusResult::Ready(0x12));
    assert_eq!(bus.read_byte(0x004001), BusResult::Ready(0x34));

    // 3. Individual byte writes
    assert_eq!(bus.write_byte(0x004000, 0xAB), BusResult::Ready(()));
    assert_eq!(bus.read_word(0x004000), BusResult::Ready(0xAB34));

    assert_eq!(bus.write_byte(0x004001, 0xCD), BusResult::Ready(()));
    assert_eq!(bus.read_word(0x004000), BusResult::Ready(0xABCD));

    // 4. Contention Handling on Chip RAM
    assert_eq!(bus.read_word(0x004000), BusResult::Ready(0xABCD));
    bus.lock_chip_ram();
    assert_eq!(bus.read_word(0x004000), BusResult::WaitState);
    assert_eq!(bus.write_word(0x004000, 0x9999), BusResult::WaitState);
    bus.unlock_chip_ram();
    assert_eq!(bus.read_word(0x004000), BusResult::Ready(0xABCD));
}

