use physical_memory::{A500Config, A500Preset, BusResult, PhysicalMemory};

#[test]
fn test_boot_overlay_and_cia_control() {
    let mut bus = PhysicalMemory::new();

    // Inject custom Kickstart ROM byte at offset 0
    let mut rom = vec![0x00; 256 * 1024];
    rom[0] = 0x12;
    rom[1] = 0x34;
    bus.write_bytes_debug(0xF80000, &rom);

    // Initial state: low-memory overlay active -> $000000 reads from Kickstart
    assert!(bus.is_low_memory_overlay_active());
    assert_eq!(bus.read_word_debug(0x000000), 0x1234);

    // Write to Chip RAM while overlay is active should not corrupt ROM
    bus.write_word_debug(0x000000, 0x5678);
    assert_eq!(bus.read_word_debug(0x000000), 0x1234);

    // Disengage overlay via map_chip_ram_to_low_memory
    bus.map_chip_ram_to_low_memory();
    assert!(!bus.is_low_memory_overlay_active());

    // Now $000000 reads physical Chip RAM
    bus.write_word_debug(0x000000, 0xABCD);
    assert_eq!(bus.read_word_debug(0x000000), 0xABCD);

    // Re-engage overlay via map_kickstart_to_low_memory
    bus.map_kickstart_to_low_memory();
    assert!(bus.is_low_memory_overlay_active());
    assert_eq!(bus.read_word_debug(0x000000), 0x1234);
}

#[test]
fn test_chip_ram_contention_and_direct_rw() {
    let mut bus = PhysicalMemory::new();
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

    bus.chip_ram_blocked = true;
    assert!(bus.chip_ram_blocked);
    assert_eq!(bus.read_word(0x001000), BusResult::WaitState);
    assert_eq!(bus.write_word(0x001000, 0x1234), BusResult::WaitState);
    assert_eq!(bus.read_word(0xC00000), BusResult::WaitState);
    assert_eq!(bus.write_word(0xC00000, 0x1234), BusResult::WaitState);
    assert_eq!(bus.read_word(0x200000), BusResult::Ready(0xFFFF)); // Fast RAM unaffected

    bus.chip_ram_blocked = false;
    assert!(!bus.chip_ram_blocked);
    assert_eq!(bus.read_word(0x001000), BusResult::Ready(0xCAFE));
}

#[test]
fn test_floating_bus_and_contention_targets() {
    let mut config = A500Config::default();
    config.apply_preset(A500Preset::ExpandedPowerUser);
    let mut bus = PhysicalMemory::from_config(config);
    bus.map_chip_ram_to_low_memory();

    // Unmapped address ($150000) returns $FF / $FFFF
    assert_eq!(bus.read_byte_debug(0x150000), 0xFF);
    assert_eq!(bus.read_word_debug(0x150000), 0xFFFF);

    // Verify is_chip_ram_target
    assert!(bus.is_chip_ram_target(0x000100)); // Chip RAM
    assert!(bus.is_chip_ram_target(0xC00000)); // Slow RAM
    assert!(!bus.is_chip_ram_target(0x200100)); // Fast RAM
}

#[test]
fn test_configurable_unmapped_byte_default_ff_and_test_mode() {
    // 1. Real emulator mode: unmapped memory defaults to 0xFF (open bus floating high)
    let mut real_bus = PhysicalMemory::new();
    assert_eq!(real_bus.unmapped_byte(), 0xFF);
    assert_eq!(real_bus.read_byte_debug(0x180000), 0xFF);
    assert_eq!(real_bus.read_word_debug(0x180000), 0xFFFF);

    // Can reconfigure open bus default if needed
    real_bus.set_unmapped_byte(0xAA);
    assert_eq!(real_bus.read_byte_debug(0x180000), 0xAA);
    assert_eq!(real_bus.read_word_debug(0x180000), 0xAAAA);
}

#[test]
fn test_bus_direct_read_write_and_byte_accesses() {
    let mut bus = PhysicalMemory::new();
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
    bus.chip_ram_blocked = true;
    assert_eq!(bus.read_word(0x004000), BusResult::WaitState);
    assert_eq!(bus.write_word(0x004000, 0x9999), BusResult::WaitState);
    bus.chip_ram_blocked = false;
    assert_eq!(bus.read_word(0x004000), BusResult::Ready(0xABCD));
}

#[test]
fn test_physical_memory_reset_and_kickstart_direct_access() {
    let mut bus = PhysicalMemory::new();
    let mut rom = vec![0x00; 256 * 1024];
    rom[0] = 0x12;
    rom[1] = 0x34;
    rom[2] = 0x56;
    rom[3] = 0x78;
    bus.write_bytes_debug(0xF80000, &rom);

    // Verify direct kickstart read handlers
    assert_eq!(
        physical_memory::map::read_kickstart_rom(&bus, 0),
        BusResult::Ready(0x12)
    );
    assert_eq!(
        physical_memory::map::read_kickstart_rom(&bus, 1),
        BusResult::Ready(0x34)
    );
    assert_eq!(
        physical_memory::map::read_kickstart_rom_word(&bus, 0),
        BusResult::Ready(0x1234)
    );
    assert_eq!(
        physical_memory::map::read_kickstart_rom_word(&bus, 2),
        BusResult::Ready(0x5678)
    );

    // Modify memory and disengage overlay
    bus.map_chip_ram_to_low_memory();
    assert!(!bus.is_low_memory_overlay_active());
    let _ = bus.write_word(0x000000, 0x9999);
    bus.chip_ram_blocked = true;

    // Reset clears RAM, unblocks chip RAM, and re-engages overlay
    bus.reset();
    assert!(bus.is_low_memory_overlay_active());
    assert!(!bus.chip_ram_blocked);
    assert_eq!(bus.read_word_debug(0x000000), 0x1234);
}

#[test]
fn test_write_bytes_across_all_memory_regions() {
    let mut bus = PhysicalMemory::new();
    bus.map_chip_ram_to_low_memory();

    // 1. Write to Chip RAM
    let chip_data = [0xAA, 0xBB, 0xCC, 0xDD];
    let written = bus.write_bytes_debug(0x001000, &chip_data);
    assert_eq!(written, 4);
    assert_eq!(bus.read_word_debug(0x001000), 0xAABB);
    assert_eq!(bus.read_word_debug(0x001002), 0xCCDD);

    // 2. Write to Fast RAM (requires configuration with Fast RAM enabled)
    let mut expanded_bus = PhysicalMemory::from_config(A500Config::from_preset(
        A500Preset::ExpandedPowerUser,
        config::VideoStandard::Pal,
    ));
    let fast_data = [0x11, 0x22, 0x33, 0x44];
    let written_fast = expanded_bus.write_bytes_debug(0x200000, &fast_data);
    assert_eq!(written_fast, 4);
    assert_eq!(expanded_bus.read_word_debug(0x200000), 0x1122);
    assert_eq!(expanded_bus.read_word_debug(0x200002), 0x3344);

    // 3. Write to Slow RAM
    let slow_data = [0x55, 0x66, 0x77, 0x88];
    let written_slow = bus.write_bytes_debug(0xC00000, &slow_data);
    assert_eq!(written_slow, 4);
    assert_eq!(bus.read_word_debug(0xC00000), 0x5566);
    assert_eq!(bus.read_word_debug(0xC00002), 0x7788);

    // 4. Write to Kickstart ROM space ($F80000)
    let rom_data = [0xDE, 0xAD, 0xBE, 0xEF];
    let written_rom = bus.write_bytes_debug(0xF80000, &rom_data);
    assert_eq!(written_rom, 4);
    assert_eq!(bus.read_word_debug(0xF80000), 0xDEAD);
    assert_eq!(bus.read_word_debug(0xF80002), 0xBEEF);

    // 5. Empty slice returns 0
    assert_eq!(bus.write_bytes_debug(0x001000, &[]), 0);

    // 6. Verification that write_bytes_debug operates in debug mode, bypassing chip_ram_blocked
    bus.chip_ram_blocked = true;
    let blocked_chip_data = [0x12, 0x34];
    assert_eq!(bus.write_bytes_debug(0x002000, &blocked_chip_data), 2);
    assert_eq!(bus.read_word_debug(0x002000), 0x1234);
    bus.chip_ram_blocked = false;

    // 7. Verification that direct debug write to ROM works
    bus.write_word_debug(0xF80010, 0xFEED);
    assert_eq!(bus.read_word_debug(0xF80010), 0xFEED);
    bus.write_byte_debug(0xF80012, 0x42);
    assert_eq!(bus.read_byte_debug(0xF80012), 0x42);
}
