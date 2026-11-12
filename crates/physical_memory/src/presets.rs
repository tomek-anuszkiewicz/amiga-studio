//! Precalculated memory bank dispatch tables for Amiga 500 system presets
//!
//! Maps physical address space topologies for Bare 512 KB, Standard 1 MB (+ Slow RAM + RTC),
//! and Expanded Power User (1 MB + 4 MB Fast RAM + RTC) configurations at compile time.

use super::map::{
    BankHandler, CHIP_RAM_HANDLER, CIA_HANDLER, CUSTOM_CHIPS_HANDLER, FAST_RAM_HANDLER,
    KICKSTART_ROM_HANDLER, OPEN_BUS_HANDLER, RTC_HANDLER, SLOW_RAM_HANDLER,
};
use config::address_map::{CIA_BANK_FIRST, CIA_BANK_LAST, CUSTOM_BANK_FIRST, CUSTOM_BANK_LAST};
use config::{A500Config, A500Preset};

const RTC_BANK: usize = 0xDC;

/// Precalculates the 256-entry 64 KB memory bank dispatch table for a given preset at compile time
pub const fn build_preset_bank_map(
    preset: A500Preset,
) -> [BankHandler; config::address_map::MEMORY_BANK_COUNT] {
    let mut map = [OPEN_BUS_HANDLER; config::address_map::MEMORY_BANK_COUNT];

    // 1. Chip RAM: 512 KB occupies banks 0x00..=0x07 (8 banks of 64 KB)
    let mut b = config::address_map::CHIP_BANK_FIRST as usize;
    while b <= config::address_map::CHIP_BANK_LAST as usize {
        map[b] = CHIP_RAM_HANDLER;
        b += 1;
    }

    // 2. Fast RAM: 4 MB occupies banks 0x20..=0x5F (64 banks of 64 KB)
    if matches!(preset, A500Preset::ExpandedPowerUser) {
        let mut b = config::address_map::FAST_BANK_FIRST as usize;
        while b <= config::address_map::FAST_BANK_LAST as usize {
            map[b] = FAST_RAM_HANDLER;
            b += 1;
        }
    }

    // 3. CIA qualification: A23..A21 = 101, independent low A12/A13 selects.
    let mut b = CIA_BANK_FIRST as usize;
    while b <= CIA_BANK_LAST as usize {
        map[b] = CIA_HANDLER;
        b += 1;
    }

    // 4. Slow RAM: 512 KB occupies banks 0xC0..=0xC7 (8 banks of 64 KB)
    if matches!(
        preset,
        A500Preset::Standard1Mb | A500Preset::ExpandedPowerUser
    ) {
        let mut b = config::address_map::SLOW_BANK_FIRST as usize;
        while b <= config::address_map::SLOW_BANK_LAST as usize {
            map[b] = SLOW_RAM_HANDLER;
            b += 1;
        }
    }

    // 5. RTC: bank 0xDC (at $DC0000..=$DC003F)
    if matches!(
        preset,
        A500Preset::Standard1Mb | A500Preset::ExpandedPowerUser
    ) {
        map[RTC_BANK] = RTC_HANDLER;
    }

    // 6. Both custom banks share the low 512-byte register window.
    let mut b = CUSTOM_BANK_FIRST as usize;
    while b <= CUSTOM_BANK_LAST as usize {
        map[b] = CUSTOM_CHIPS_HANDLER;
        b += 1;
    }

    // 7. Kickstart ROM: 512 KB occupies banks 0xF8..=0xFF (8 banks of 64 KB)
    let mut b = config::address_map::KICKSTART_BANK_FIRST as usize;
    while b <= config::address_map::KICKSTART_BANK_LAST as usize {
        map[b] = KICKSTART_ROM_HANDLER;
        b += 1;
    }

    map
}

/// Static compile-time bank dispatch table for Preset 1 (Bare Stock 512 KB)
pub static BANK_MAP_BARE: [BankHandler; config::address_map::MEMORY_BANK_COUNT] =
    build_preset_bank_map(A500Preset::Bare512k);

/// Static compile-time bank dispatch table for Preset 2 (Standard 1 MB + RTC)
pub static BANK_MAP_STANDARD: [BankHandler; config::address_map::MEMORY_BANK_COUNT] =
    build_preset_bank_map(A500Preset::Standard1Mb);

/// Static compile-time bank dispatch table for Preset 3 (Expanded Power User: 1 MB + 4 MB Fast + RTC)
pub static BANK_MAP_EXPANDED: [BankHandler; config::address_map::MEMORY_BANK_COUNT] =
    build_preset_bank_map(A500Preset::ExpandedPowerUser);

/// Returns a reference to the static compile-time bank dispatch table for the given preset
#[inline(always)]
pub const fn get_preset_bank_map(
    preset: A500Preset,
) -> &'static [BankHandler; config::address_map::MEMORY_BANK_COUNT] {
    match preset {
        A500Preset::Bare512k => &BANK_MAP_BARE,
        A500Preset::Standard1Mb => &BANK_MAP_STANDARD,
        A500Preset::ExpandedPowerUser => &BANK_MAP_EXPANDED,
    }
}

/// Returns the 256-entry 64 KB memory bank dispatch table based on active configuration
#[inline(always)]
pub fn build_bank_map(
    config: &A500Config,
) -> [BankHandler; config::address_map::MEMORY_BANK_COUNT] {
    *get_preset_bank_map(config.active_preset())
}
