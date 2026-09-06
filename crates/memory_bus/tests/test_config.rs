use memory_bus::config::{A500Config, A500Preset, ChipRamSize, FastRamSize, RtcModel, SlowRamSize, VideoStandard};
use memory_bus::MemoryBus;

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
    let mut bus = MemoryBus::from_config(config);

    assert!(bus.slow_ram.is_none());
    assert!(bus.fast_ram.is_none());

    // Reading RTC area ($DC0000) on bare A500 returns open bus $FF
    assert_eq!(bus.read_byte_debug(0xDC0000), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0004), 0xFF);
}

#[test]
fn test_memory_bus_with_standard_1mb_has_active_rtc() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut bus = MemoryBus::from_config(config);

    assert!(bus.slow_ram.is_some());
    assert!(bus.fast_ram.is_none());

    // Write to RTC register 1 (addr $DC0004)
    bus.write_byte_debug(0xDC0004, 0x07);
    assert_eq!(bus.read_byte_debug(0xDC0004), 0x07);

    // Write to RTC register 0 (addr $DC0000)
    bus.write_byte_debug(0xDC0000, 0x09);
    assert_eq!(bus.read_byte_debug(0xDC0000), 0x09);
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
    assert_eq!(bus.read_byte_debug(0xDC0000), 0xFF);
}
