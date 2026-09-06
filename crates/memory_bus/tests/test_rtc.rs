//! Unit and integration tests for the OKI MSM6242B Real-Time Clock (RTC) subsystem
//!
//! Verifies odd-byte address decoding, BCD conversion, 24/12h mode, HOLD latching,
//! CCK cycle stepping, and preset mapping per Obsidian/Amiga/Design/RTC.md.

use memory_bus::{A500Config, MemoryBus, RtcModel, VideoStandard};

#[test]
fn test_rtc_unmapped_on_bare_512k() {
    let config = A500Config::bare_512k(VideoStandard::Pal);
    let mut bus = MemoryBus::from_config(config);

    assert_eq!(bus.config.rtc(), RtcModel::None);

    // Unmapped RTC space ($DC0000..$DC003F) returns open bus $FF on both even and odd bytes
    assert_eq!(bus.read_byte_debug(0xDC0000), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0001), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0004), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0005), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC003C), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC003D), 0xFF);

    // Writes are silent no-ops
    bus.write_byte_debug(0xDC0001, 0x05);
    assert_eq!(bus.read_byte_debug(0xDC0001), 0xFF);
}

#[test]
fn test_rtc_odd_byte_addressing() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut bus = MemoryBus::from_config(config);

    assert_eq!(bus.config.rtc(), RtcModel::Msm6242b);

    // Even byte addresses in RTC space ($DC0000, $DC0002, ...) return open bus $FF
    assert_eq!(bus.read_byte_debug(0xDC0000), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0002), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC000E), 0xFF);

    // Odd byte addresses return active 4-bit nibbles ($00..$0F)
    assert!(bus.read_byte_debug(0xDC0001) <= 0x0F);
    assert!(bus.read_byte_debug(0xDC0005) <= 0x0F);
    assert!(bus.read_byte_debug(0xDC0009) <= 0x0F);

    // Even byte writes do not alter odd byte register state
    let before = bus.read_byte_debug(0xDC0001);
    bus.write_byte_debug(0xDC0000, 0x0A);
    assert_eq!(bus.read_byte_debug(0xDC0001), before);
}

#[test]
fn test_rtc_bcd_decomposition_and_registers() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut bus = MemoryBus::from_config(config);

    // Timestamp for 1993-03-15 14:27:08 UTC = 732205628
    // Note: Monday March 15, 1993
    bus.rtc.set_time(732205628);

    // Register 0 (1s sec) -> 8
    assert_eq!(bus.read_byte_debug(0xDC0001), 8);
    // Register 1 (10s sec) -> 0
    assert_eq!(bus.read_byte_debug(0xDC0005), 0);
    // Register 2 (1s min) -> 7
    assert_eq!(bus.read_byte_debug(0xDC0009), 7);
    // Register 3 (10s min) -> 2
    assert_eq!(bus.read_byte_debug(0xDC000D), 2);
    // Register 4 (1s hour) -> 4
    assert_eq!(bus.read_byte_debug(0xDC0011), 4);
    // Register 5 (10s hour) -> 1
    assert_eq!(bus.read_byte_debug(0xDC0015), 1);
    // Register 6 (1s day) -> 5
    assert_eq!(bus.read_byte_debug(0xDC0019), 5);
    // Register 7 (10s day) -> 1
    assert_eq!(bus.read_byte_debug(0xDC001D), 1);
    // Register 8 (1s month) -> 3
    assert_eq!(bus.read_byte_debug(0xDC0021), 3);
    // Register 9 (10s month) -> 0
    assert_eq!(bus.read_byte_debug(0xDC0025), 0);
    // Register A (1s year) -> 3
    assert_eq!(bus.read_byte_debug(0xDC0029), 3);
    // Register B (10s year) -> 9 (for 1993)
    assert_eq!(bus.read_byte_debug(0xDC002D), 9);
    // Register C (day of week: 1 = Monday)
    assert_eq!(bus.read_byte_debug(0xDC0031), 1);
}

#[test]
fn test_rtc_12_hour_mode() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut bus = MemoryBus::from_config(config);

    // 14:27:08 (2:27:08 PM)
    bus.rtc.set_time(732205628);

    // Switch to 12-hour mode by clearing Bit 2 of Control Register F ($DC003D)
    let ctrl_f = bus.read_byte_debug(0xDC003D);
    bus.write_byte_debug(0xDC003D, ctrl_f & !0x04);

    // In 12-hour mode:
    // Reg 4 = 2 (1s hour)
    assert_eq!(bus.read_byte_debug(0xDC0011), 2);
    // Reg 5 = PM bit (bit 2 = 0x04), 10s hour is 0 -> 0x04
    assert_eq!(bus.read_byte_debug(0xDC0015), 0x04);

    // Midnight 00:15:00 should be 12:15 AM
    bus.rtc.set_time(732154500);
    // Reg 4 = 2, Reg 5 = 1 (12 AM, PM bit 0)
    assert_eq!(bus.read_byte_debug(0xDC0011), 2);
    assert_eq!(bus.read_byte_debug(0xDC0015), 1);
}

#[test]
fn test_rtc_hold_register_latching() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut bus = MemoryBus::from_config(config);

    bus.rtc.set_time(732196028); // Seconds = 08
    assert_eq!(bus.read_byte_debug(0xDC0001), 8);

    // Engage HOLD (Bit 0 of Control Register D, addr $DC0035)
    bus.write_byte_debug(0xDC0035, 0x01);

    // Step clock by 5 seconds (5 * 3,546,895 CCK cycles)
    bus.step_cck(5 * 3_546_895);

    // While HOLD is engaged, readable register latches are frozen
    assert_eq!(bus.read_byte_debug(0xDC0001), 8);
    assert_eq!(bus.read_byte_debug(0xDC0005), 0);

    // Release HOLD (clear Bit 0)
    bus.write_byte_debug(0xDC0035, 0x00);

    // Now registers reflect the advanced time (08 + 5 = 13 seconds)
    assert_eq!(bus.read_byte_debug(0xDC0001), 3); // 1s digit of 13
    assert_eq!(bus.read_byte_debug(0xDC0005), 1); // 10s digit of 13
}

#[test]
fn test_rtc_step_cck_cycle_exactness() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut bus = MemoryBus::from_config(config);

    // Clear HOLD to allow live updates
    bus.write_byte_debug(0xDC0035, 0x00);
    bus.rtc.set_time(732196028); // Seconds = 08
    assert_eq!(bus.read_byte_debug(0xDC0001), 8);

    // Step 3,546,894 cycles (1 cycle short of 1 second)
    bus.step_cck(3_546_894);
    assert_eq!(bus.read_byte_debug(0xDC0001), 8);

    // Step the final 1 cycle to complete 1 full second
    bus.step_cck(1);
    assert_eq!(bus.read_byte_debug(0xDC0001), 9);
}

#[test]
fn test_rtc_bank_boundary_open_bus() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let bus = MemoryBus::from_config(config);

    // Inside RTC window ($DC0000..$DC003F):
    // Register 15 at $DC003D is mapped
    assert_ne!(bus.read_byte_debug(0xDC003D), 0xFF);

    // Outside RTC window ($DC0040..$DCFFFF in Bank $DC):
    assert_eq!(bus.read_byte_debug(0xDC0040), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0041), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC1000), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDCFFFF), 0xFF);
}

#[test]
fn test_rtc_reexported_module_namespace() {
    // Verify that downstream callers can construct or type-check RtcMsm6242b
    // directly via memory_bus::rtc::RtcMsm6242b without depending directly on crate rtc
    let rtc_instance = memory_bus::rtc::RtcMsm6242b::new(memory_bus::RtcModel::Msm6242b);
    assert_eq!(rtc_instance.model, memory_bus::RtcModel::Msm6242b);
}
