use memory_bus::MemoryBus;
use memory_bus::{
    A500Config, A500Preset, ChipRamSize, FastRamSize, RtcModel, SlowRamSize, VideoStandard,
};

#[test]
fn test_default_config_is_standard_1mb_pal() {
    let config = A500Config::default();
    assert_eq!(config.active_preset(), A500Preset::Standard1Mb);
    assert_eq!(config.video_standard(), VideoStandard::Pal);
    assert_eq!(config.chip_ram(), ChipRamSize::Kb512);
    assert_eq!(config.slow_ram(), SlowRamSize::Kb512);
    assert_eq!(config.fast_ram(), FastRamSize::None);
    assert_eq!(config.rtc(), RtcModel::Msm6242b);
}

#[test]
fn test_bare_512k_preset() {
    let config = A500Config::bare_512k(VideoStandard::Ntsc);
    assert_eq!(config.active_preset(), A500Preset::Bare512k);
    assert_eq!(config.video_standard(), VideoStandard::Ntsc);
    assert_eq!(config.chip_ram(), ChipRamSize::Kb512);
    assert_eq!(config.slow_ram(), SlowRamSize::None);
    assert_eq!(config.fast_ram(), FastRamSize::None);
    assert_eq!(config.rtc(), RtcModel::None);
}

#[test]
fn test_expanded_power_user_preset() {
    let config = A500Config::expanded_power_user(VideoStandard::Pal);
    assert_eq!(config.active_preset(), A500Preset::ExpandedPowerUser);
    assert_eq!(config.video_standard(), VideoStandard::Pal);
    assert_eq!(config.chip_ram(), ChipRamSize::Kb512);
    assert_eq!(config.slow_ram(), SlowRamSize::Kb512);
    assert_eq!(config.fast_ram(), FastRamSize::Mb4);
    assert_eq!(config.rtc(), RtcModel::Msm6242b);
}

#[test]
fn test_apply_preset_mutation() {
    let mut config = A500Config::bare_512k(VideoStandard::Pal);
    assert_eq!(config.active_preset(), A500Preset::Bare512k);
    assert_eq!(config.rtc(), RtcModel::None);

    // Apply standard 1MB preset
    config.apply_preset(A500Preset::Standard1Mb);
    assert_eq!(config.active_preset(), A500Preset::Standard1Mb);
    assert_eq!(config.slow_ram(), SlowRamSize::Kb512);
    assert_eq!(config.rtc(), RtcModel::Msm6242b);
}

#[test]
fn test_memory_bus_with_bare_preset_has_open_bus_rtc() {
    let config = A500Config::bare_512k(VideoStandard::Pal);
    let bus = MemoryBus::from_config(config);

    assert!(bus.slow_ram.is_none());
    assert!(bus.fast_ram.is_none());

    // Reading RTC area ($DC0000..$DC003F) on bare A500 returns open bus $FF on both even and odd bytes
    assert_eq!(bus.read_byte_debug(0xDC0000), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0001), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0004), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0005), 0xFF);
}

#[test]
fn test_memory_bus_with_standard_1mb_has_rtc_bank() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut bus = MemoryBus::from_config(config);

    assert!(bus.slow_ram.is_some());
    assert!(bus.fast_ram.is_none());
    assert_eq!(bus.bank_map[0xDC].bank, memory_bus::MemoryBank::Rtc);

    // In PhysicalMemory, RTC addresses return floating open bus $FF
    assert_eq!(bus.read_byte_debug(0xDC0000), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0001), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0005), 0xFF);

    // Writes to unhandled peripheral space are silent no-ops
    bus.write_byte_debug(0xDC0001, 0x09);
    assert_eq!(bus.read_byte_debug(0xDC0001), 0xFF);
}

#[test]
fn test_memory_bus_apply_config_dynamically() {
    let mut bus = MemoryBus::new();
    assert_eq!(bus.config.active_preset(), A500Preset::Standard1Mb);
    assert!(bus.slow_ram.is_some());

    // Switch dynamically to bare 512k
    bus.apply_config(A500Config::bare_512k(VideoStandard::Pal));
    assert_eq!(bus.config.active_preset(), A500Preset::Bare512k);
    assert!(bus.slow_ram.is_none());
    assert_eq!(bus.read_byte_debug(0xDC0001), 0xFF);
}

#[test]
fn test_256_entry_bank_map() {
    use memory_bus::MemoryBank;

    // 1. Standard 1MB config
    let mut bus = MemoryBus::new();
    // At startup, low-memory overlay is active: banks 0..=7 point to Kickstart ROM
    for b in 0..=7 {
        assert_eq!(bus.bank_map[b], MemoryBank::KickstartRom);
        assert!(!bus.bank_map[b].is_contended);
    }
    // Disengage overlay to restore physical Chip RAM map
    bus.map_chip_ram_to_low_memory();
    for b in 0..=7 {
        assert_eq!(bus.bank_map[b], MemoryBank::ChipRam);
        assert!(bus.bank_map[b].is_contended);
    }
    // Extended Chip: 8..=15 are OpenBus on 512k baseline
    for b in 8..=15 {
        assert_eq!(bus.bank_map[b], MemoryBank::OpenBus);
    }
    // CIA bank: 0xBF
    assert_eq!(bus.bank_map[0xBF], MemoryBank::Cia);
    // Slow RAM: 0xC0..=0xC7
    for b in 0xC0..=0xC7 {
        assert_eq!(bus.bank_map[b], MemoryBank::SlowRam);
    }
    // RTC bank: 0xDC
    assert_eq!(bus.bank_map[0xDC], MemoryBank::Rtc);
    // Custom chips: 0xDF
    assert_eq!(bus.bank_map[0xDF], MemoryBank::CustomChips);
    // Kickstart ROM: 0xF8..=0xFF
    for b in 0xF8..=0xFF {
        assert_eq!(bus.bank_map[b], MemoryBank::KickstartRom);
    }

    assert_eq!(bus.bank_map, memory_bus::map::BANK_MAP_STANDARD);

    // 2. Bare 512k config: SlowRam and RTC become OpenBus
    let mut bare_bus = MemoryBus::from_config(A500Config::bare_512k(VideoStandard::Pal));
    bare_bus.map_chip_ram_to_low_memory();
    for b in 0xC0..=0xC7 {
        assert_eq!(bare_bus.bank_map[b], MemoryBank::OpenBus);
    }
    assert_eq!(bare_bus.bank_map[0xDC], MemoryBank::OpenBus);
    assert_eq!(bare_bus.bank_map, memory_bus::map::BANK_MAP_BARE);

    // 3. Expanded config: Fast RAM occupies 0x20..=0x5F
    let mut exp_bus = MemoryBus::from_config(A500Config::expanded_power_user(VideoStandard::Pal));
    exp_bus.map_chip_ram_to_low_memory();
    for b in 0x20..=0x5F {
        assert_eq!(exp_bus.bank_map[b], MemoryBank::FastRam);
    }
    assert_eq!(exp_bus.bank_map, memory_bus::map::BANK_MAP_EXPANDED);
}

#[test]
fn test_bank_handler_direct_method_pointer_dispatch() {
    let mut bus = MemoryBus::new();
    bus.map_chip_ram_to_low_memory();

    // Directly invoke write and read handler pointers from bank_map
    let chip_handler = bus.bank_map[0x00];
    (chip_handler.write_byte)(&mut bus, 0x000100, 0x42);
    let val = (chip_handler.read_byte)(&bus, 0x000100);
    assert_eq!(val, 0x42);

    // Verify through normal bus read
    assert_eq!(bus.read_byte_debug(0x000100), 0x42);
}
