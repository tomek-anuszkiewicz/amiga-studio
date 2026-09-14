//! Integration tests for the OKI MSM6242B Real-Time Clock (RTC) subsystem via A500Machine
//!
//! Verifies odd-byte address decoding, BCD conversion, 24/12h mode, HOLD latching,
//! CCK cycle stepping, and preset mapping per Obsidian/Amiga/Design/RTC.md.

use config::{A500Config, RtcModel, VideoStandard};
use machine_loop::BusResult;
use machine_loop::{A500Machine, AddressBus};

#[test]
fn test_rtc_unmapped_on_bare_512k() {
    let config = A500Config::bare_512k(VideoStandard::Pal);
    let mut machine = A500Machine::new(config);

    assert_eq!(machine.config.rtc(), RtcModel::None);

    // Unmapped RTC space ($DC0000..$DC003F) returns open bus $FF on both even and odd bytes
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0000),
        BusResult::Ready(0xFF)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(0xFF)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0004),
        BusResult::Ready(0xFF)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0005),
        BusResult::Ready(0xFF)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC003C),
        BusResult::Ready(0xFF)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC003D),
        BusResult::Ready(0xFF)
    );

    // Writes are silent no-ops
    let _ = machine.memory_bus().write_byte(0xDC0001, 0x05);
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(0xFF)
    );
}

#[test]
fn test_rtc_odd_byte_addressing() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut machine = A500Machine::new(config);

    assert_eq!(machine.config.rtc(), RtcModel::Msm6242b);

    // Even byte addresses in RTC space ($DC0000, $DC0002, ...) return open bus $FF
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0000),
        BusResult::Ready(0xFF)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0002),
        BusResult::Ready(0xFF)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC000E),
        BusResult::Ready(0xFF)
    );

    // Odd byte addresses return active 4-bit nibbles ($00..$0F)
    assert!(machine.memory_bus().read_byte(0xDC0001).ok().unwrap() <= 0x0F);
    assert!(machine.memory_bus().read_byte(0xDC0005).ok().unwrap() <= 0x0F);
    assert!(machine.memory_bus().read_byte(0xDC0009).ok().unwrap() <= 0x0F);

    // Even byte writes do not alter odd byte register state
    let before = machine.memory_bus().read_byte(0xDC0001).ok().unwrap();
    let _ = machine.memory_bus().write_byte(0xDC0000, 0x0A);
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(before)
    );
}

#[test]
fn test_rtc_bcd_decomposition_and_registers() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut machine = A500Machine::new(config);

    // Timestamp for 1993-03-15 14:27:08 UTC = 732205628
    // Note: Monday March 15, 1993
    machine.rtc.set_time(732205628);

    // Register 0 (1s sec) -> 8
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(8)
    );
    // Register 1 (10s sec) -> 0
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0005),
        BusResult::Ready(0)
    );
    // Register 2 (1s min) -> 7
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0009),
        BusResult::Ready(7)
    );
    // Register 3 (10s min) -> 2
    assert_eq!(
        machine.memory_bus().read_byte(0xDC000D),
        BusResult::Ready(2)
    );
    // Register 4 (1s hour) -> 4
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0011),
        BusResult::Ready(4)
    );
    // Register 5 (10s hour) -> 1
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0015),
        BusResult::Ready(1)
    );
    // Register 6 (1s day) -> 5
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0019),
        BusResult::Ready(5)
    );
    // Register 7 (10s day) -> 1
    assert_eq!(
        machine.memory_bus().read_byte(0xDC001D),
        BusResult::Ready(1)
    );
    // Register 8 (1s month) -> 3
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0021),
        BusResult::Ready(3)
    );
    // Register 9 (10s month) -> 0
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0025),
        BusResult::Ready(0)
    );
    // Register A (1s year) -> 3
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0029),
        BusResult::Ready(3)
    );
    // Register B (10s year) -> 9 (for 1993)
    assert_eq!(
        machine.memory_bus().read_byte(0xDC002D),
        BusResult::Ready(9)
    );
    // Register C (day of week: 1 = Monday)
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0031),
        BusResult::Ready(1)
    );
}

#[test]
fn test_rtc_12_hour_mode() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut machine = A500Machine::new(config);

    // 14:27:08 (2:27:08 PM)
    machine.rtc.set_time(732205628);

    // Switch to 12-hour mode by clearing Bit 2 of Control Register F ($DC003D)
    let ctrl_f = machine.memory_bus().read_byte(0xDC003D).ok().unwrap();
    let _ = machine.memory_bus().write_byte(0xDC003D, ctrl_f & !0x04);

    // In 12-hour mode:
    // Reg 4 = 2 (1s hour)
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0011),
        BusResult::Ready(2)
    );
    // Reg 5 = PM bit (bit 2 = 0x04), 10s hour is 0 -> 0x04
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0015),
        BusResult::Ready(0x04)
    );

    // Midnight 00:15:00 should be 12:15 AM
    machine.rtc.set_time(732154500);
    // Reg 4 = 2, Reg 5 = 1 (12 AM, PM bit 0)
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0011),
        BusResult::Ready(2)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0015),
        BusResult::Ready(1)
    );
}

#[test]
fn test_rtc_hold_register_latching() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut machine = A500Machine::new(config);

    machine.rtc.set_time(732196028); // Seconds = 08
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(8)
    );

    // Engage HOLD (Bit 0 of Control Register D, addr $DC0035)
    let _ = machine.memory_bus().write_byte(0xDC0035, 0x01);

    // Step clock by 5 seconds (5 * 3,546,895 CCK cycles)
    machine.rtc.step_cck(5 * 3_546_895);

    // While HOLD is engaged, readable register latches are frozen
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(8)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0005),
        BusResult::Ready(0)
    );

    // Release HOLD (clear Bit 0)
    let _ = machine.memory_bus().write_byte(0xDC0035, 0x00);

    // Now registers reflect the advanced time (08 + 5 = 13 seconds)
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(3)
    ); // 1s digit of 13
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0005),
        BusResult::Ready(1)
    ); // 10s digit of 13
}

#[test]
fn test_rtc_step_cck_cycle_exactness() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut machine = A500Machine::new(config);

    // Clear HOLD to allow live updates
    let _ = machine.memory_bus().write_byte(0xDC0035, 0x00);
    machine.rtc.set_time(732196028); // Seconds = 08
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(8)
    );

    // Step 3,546,894 cycles (1 cycle short of 1 second)
    machine.rtc.step_cck(3_546_894);
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(8)
    );

    // Step the final 1 cycle to complete 1 full second
    machine.rtc.step_cck(1);
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(9)
    );
}

#[test]
fn test_rtc_bank_boundary_open_bus() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut machine = A500Machine::new(config);

    // Inside RTC window ($DC0000..$DC003F):
    // Register 15 at $DC003D is mapped
    assert_ne!(
        machine.memory_bus().read_byte(0xDC003D),
        BusResult::Ready(0xFF)
    );

    // Outside RTC window ($DC0040..$DCFFFF in Bank $DC):
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0040),
        BusResult::Ready(0xFF)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0041),
        BusResult::Ready(0xFF)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC1000),
        BusResult::Ready(0xFF)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDCFFFF),
        BusResult::Ready(0xFF)
    );
}
