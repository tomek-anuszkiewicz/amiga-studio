//! Unit tests for Amiga 500 memory bank presets and topology builders

use config::{A500Config, A500Preset, VideoStandard};
use physical_memory::presets::{
    build_bank_map, build_preset_bank_map, get_preset_bank_map, BANK_MAP_BARE, BANK_MAP_EXPANDED,
    BANK_MAP_STANDARD,
};
use physical_memory::MemoryBank;

#[test]
fn test_bare_512k_preset_topology() {
    let map = build_preset_bank_map(A500Preset::Bare512k);

    // Chip RAM at banks 0x00..=0x07 (512 KB)
    for b in 0x00..=0x07 {
        assert_eq!(map[b].bank, MemoryBank::ChipRam);
    }

    // Unmapped banks before CIA (0x08..=0xBE)
    assert_eq!(map[0x08].bank, MemoryBank::OpenBus);
    assert_eq!(map[0x20].bank, MemoryBank::OpenBus); // Fast RAM unmapped in bare 512k

    // CIA at bank 0xBF
    assert_eq!(map[0xBF].bank, MemoryBank::Cia);

    // Slow RAM unmapped in bare 512k (0xC0..=0xC7)
    assert_eq!(map[0xC0].bank, MemoryBank::OpenBus);

    // RTC unmapped in bare 512k
    assert_eq!(map[0xDC].bank, MemoryBank::OpenBus);

    // Custom chips at 0xDF
    assert_eq!(map[0xDF].bank, MemoryBank::CustomChips);

    // Kickstart ROM at 0xF8..=0xFF (512 KB address space)
    for b in 0xF8..=0xFF {
        assert_eq!(map[b].bank, MemoryBank::KickstartRom);
    }
}

#[test]
fn test_standard_1mb_preset_topology() {
    let map = build_preset_bank_map(A500Preset::Standard1Mb);

    // Chip RAM at banks 0x00..=0x07 (512 KB)
    for b in 0x00..=0x07 {
        assert_eq!(map[b].bank, MemoryBank::ChipRam);
    }

    // Slow RAM at 0xC0..=0xC7 (512 KB trapdoor expansion)
    for b in 0xC0..=0xC7 {
        assert_eq!(map[b].bank, MemoryBank::SlowRam);
    }

    // RTC at bank 0xDC
    assert_eq!(map[0xDC].bank, MemoryBank::Rtc);

    // Fast RAM remains open bus in standard 1MB
    assert_eq!(map[0x20].bank, MemoryBank::OpenBus);
}

#[test]
fn test_expanded_power_user_preset_topology() {
    let map = build_preset_bank_map(A500Preset::ExpandedPowerUser);

    // Fast RAM at 0x20..=0x5F (4 MB auto-config)
    for b in 0x20..=0x5F {
        assert_eq!(map[b].bank, MemoryBank::FastRam);
    }

    // Slow RAM active
    assert_eq!(map[0xC0].bank, MemoryBank::SlowRam);

    // RTC active
    assert_eq!(map[0xDC].bank, MemoryBank::Rtc);
}

#[test]
fn test_static_preset_dispatch_references() {
    assert_eq!(get_preset_bank_map(A500Preset::Bare512k), &BANK_MAP_BARE);
    assert_eq!(
        get_preset_bank_map(A500Preset::Standard1Mb),
        &BANK_MAP_STANDARD
    );
    assert_eq!(
        get_preset_bank_map(A500Preset::ExpandedPowerUser),
        &BANK_MAP_EXPANDED
    );

    let config = A500Config::from_preset(A500Preset::Standard1Mb, VideoStandard::Pal);
    let map = build_bank_map(&config);
    assert_eq!(map[0xC0].bank, MemoryBank::SlowRam);
}
