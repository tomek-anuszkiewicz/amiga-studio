use memory_bus::{MemoryBus, MemoryBusResult};

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
fn test_2phase_cck_arbitration_and_contention() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();
    bus.write_word_debug(0x001000, 0xCAFE);

    // 1. Unblocked Read
    let res1 = bus.read_phase1(0x001000, false, false);
    assert_eq!(res1, MemoryBusResult::Phase1Ready);
    let res2 = bus.read_phase2(0x001000);
    assert_eq!(res2, MemoryBusResult::Ready(0xCAFE));

    // 2. Blocked Read due to Agnus DMA
    bus.lock_chip_ram();
    let res_blocked = bus.read_phase1(0x001000, false, false);
    assert_eq!(res_blocked, MemoryBusResult::Blocked);

    // 3. Unlock and verify read succeeds
    bus.unlock_chip_ram();
    let res1_ok = bus.read_phase1(0x001000, false, false);
    assert_eq!(res1_ok, MemoryBusResult::Phase1Ready);
    let res2_ok = bus.read_phase2(0x001000);
    assert_eq!(res2_ok, MemoryBusResult::Ready(0xCAFE));

    // 4. Write Phase 1 & 2
    let write_res1 = bus.write_phase1(0x002000, 0xBEEF);
    assert_eq!(write_res1, MemoryBusResult::Phase1Ready);
    let write_res2 = bus.write_phase2(0x002000, 0xBEEF, false, false);
    assert_eq!(write_res2, MemoryBusResult::Ready(0));
    assert_eq!(bus.read_word_debug(0x002000), 0xBEEF);

    // 5. Blocked Write at Phase 2
    bus.lock_chip_ram();
    let write_blocked = bus.write_phase2(0x002000, 0x1234, false, false);
    assert_eq!(write_blocked, MemoryBusResult::Blocked);
    // Value remains unwritten
    assert_eq!(bus.read_word_debug(0x002000), 0xBEEF);
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
