#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for the OKI MSM6242B Real-Time Clock (RTC) subsystem via A500Machine
//!
//! Verifies odd-byte address decoding, BCD conversion, 24/12h mode, HOLD latching,
//! host UTC sampling, snapshot independence, and preset mapping per Obsidian/Amiga/Design/RTC.md.

use config::{A500Config, RtcModel, VideoStandard};
use machine_loop::BusResult;
use machine_loop::{A500Machine, AddressBus};

const UTC: i64 = 631152000;

fn running_clock() -> A500Machine {
    let mut machine = A500Machine::new(A500Config::standard_1mb(VideoStandard::Pal));
    // 1990-01-01 00:00:08, with live register updates enabled.
    machine.rtc.set_time(631152008);
    assert_eq!(
        machine.memory_bus().write_byte(0xDC0035, 0),
        BusResult::Ready(())
    );
    machine
}

#[test]
fn rtc_uses_host_time_and_reads_do_not_change_state() {
    let mut machine = running_clock();
    let before = machine.rtc.clone();
    machine.step_subsystems_cck();
    machine.step_cck();
    machine.step_cycles(1000);
    assert_eq!(machine.rtc, before);
    machine.rtc.supply_host_utc(UTC + 1);
    assert_eq!(machine.rtc.time(), UTC + 9);
    let before = machine.rtc.clone();
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0041),
        BusResult::Ready(0xFF)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0000),
        BusResult::Ready(0xFF)
    );
    assert_eq!(
        machine.memory_bus().write_byte(0xDC0000, 3),
        BusResult::Ready(())
    );
    assert_eq!(machine.memory_bus().read_word_debug(0xDC0000), 0xFF09);
    assert_eq!(
        machine.memory_bus().read_word(0xDC0000),
        BusResult::Ready(0xFF09)
    );
    assert_eq!(machine.rtc, before);
}

#[test]
fn guest_cpu_rtc_read_uses_latest_host_sample() {
    let mut machine = running_clock();
    machine.physical_memory.map_chip_ram_to_low_memory();
    // MOVE.B $DC0001,D0; STOP #$2700.
    machine.physical_memory.chip_ram[0x1000..0x100A]
        .copy_from_slice(&[0x10, 0x39, 0x00, 0xDC, 0x00, 0x01, 0x4E, 0x72, 0x27, 0x00]);
    machine.cpu.state.set_sr(0x2700);
    machine.rtc.supply_host_utc(UTC + 1);
    machine.set_pc_and_prime_prefetch(0x1000);
    machine.step_instruction();
    assert!(!machine.cpu.state.is_halted());
    assert_eq!(machine.cpu.state.d_byte(0), 9);
    assert_eq!(machine.rtc.time(), UTC + 9);
}

#[test]
fn rtc_hold_captures_time_at_the_write_and_releases_elapsed_time() {
    let mut machine = running_clock();
    machine.rtc.supply_host_utc(UTC + 2);
    assert_eq!(
        machine.memory_bus().write_word(0xDC0034, 0xFF01),
        BusResult::Ready(())
    );
    machine.rtc.supply_host_utc(UTC + 5);
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(0)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0005),
        BusResult::Ready(1)
    );
    assert_eq!(
        machine.memory_bus().write_byte(0xDC0035, 0),
        BusResult::Ready(())
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(3)
    );
}

#[test]
fn rtc_stop_excludes_stopped_host_interval() {
    let mut machine = running_clock();
    machine.rtc.supply_host_utc(UTC + 1);
    assert_eq!(
        machine.memory_bus().write_byte(0xDC003D, 0x06),
        BusResult::Ready(())
    );
    machine.rtc.supply_host_utc(UTC + 11);
    assert_eq!(
        machine.memory_bus().write_byte(0xDC003D, 0x04),
        BusResult::Ready(())
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(9)
    );
    machine.rtc.supply_host_utc(UTC + 12);
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(0)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0005),
        BusResult::Ready(1)
    );
}

#[test]
fn rtc_writes_update_offset_and_preserve_latches_until_next_second() {
    let mut machine = running_clock();
    machine.rtc.supply_host_utc(UTC + 62);
    assert_eq!(
        machine.memory_bus().write_byte(0xDC0001, 5),
        BusResult::Ready(())
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0009),
        BusResult::Ready(1)
    );
    assert_eq!(machine.rtc.time(), UTC + 75);
    assert_eq!(machine.rtc.battery_data().offset_seconds, 13);
    assert_eq!(
        machine.memory_bus().write_byte(0xDC0031, 6),
        BusResult::Ready(())
    );
    machine.rtc.supply_host_utc(UTC + 62);
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0031),
        BusResult::Ready(6)
    );
    machine.rtc.supply_host_utc(UTC + 63);
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0031),
        BusResult::Ready(1)
    );
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(6)
    );
}

#[test]
fn snapshots_exclude_rtc_and_restore_leaves_live_clock_intact() {
    for control in [0x04, 0x06] {
        let mut machine = running_clock();
        let state = machine.save_state();
        let json = serde_json::to_value(&state).unwrap();
        assert!(json.get("rtc").is_none());
        assert!(json.get("offset_seconds").is_none());
        let decoded = serde_json::from_value(json).unwrap();
        machine.rtc.supply_host_utc(UTC + 100);
        machine.rtc.set_time(UTC + 250);
        machine.rtc.write_byte(0xDC003D, control);
        let rtc_before = machine.rtc.clone();
        machine.load_state(&decoded);
        assert_eq!(machine.rtc, rtc_before);
        assert_eq!(machine.save_state(), state);
        machine.rtc.supply_host_utc(UTC + 110);
        assert_eq!(
            machine.rtc.time(),
            UTC + if control == 0x04 { 260 } else { 250 }
        );
    }
}

#[test]
fn rtc_resets_preserve_clock_and_battery() {
    for warm in [false, true] {
        let mut machine = running_clock();
        machine.rtc.supply_host_utc(UTC + 1);
        machine.cck = 100;
        let rtc_before = machine.rtc.clone();
        if warm {
            machine.reset_warm();
        } else {
            machine.reset();
        }
        assert_eq!(machine.cck, 0);
        assert_eq!(machine.rtc, rtc_before);
        machine.rtc.supply_host_utc(UTC + 2);
        assert_eq!(
            machine.memory_bus().read_byte(0xDC0001),
            BusResult::Ready(0)
        );
        assert_eq!(
            machine.memory_bus().read_byte(0xDC0005),
            BusResult::Ready(1)
        );
    }
}

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
    assert!(matches!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(0..=0x0F)
    ));
    assert!(matches!(
        machine.memory_bus().read_byte(0xDC0005),
        BusResult::Ready(0..=0x0F)
    ));
    assert!(matches!(
        machine.memory_bus().read_byte(0xDC0009),
        BusResult::Ready(0..=0x0F)
    ));

    // Even byte writes do not alter odd byte register state
    let BusResult::Ready(before) = machine.memory_bus().read_byte(0xDC0001) else {
        panic!("Expected a completed bus read");
    };
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
    let BusResult::Ready(ctrl_f) = machine.memory_bus().read_byte(0xDC003D) else {
        panic!("Expected a completed bus read");
    };
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

    // Supply host time five seconds later.
    machine.rtc.supply_host_utc(UTC + 5);

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
fn restored_configuration_changes_presence_without_replacing_clock() {
    let mut machine = running_clock();
    let saved = machine.save_state();
    let bare = A500Machine::new(A500Config::bare_512k(VideoStandard::Ntsc)).save_state();
    machine.rtc.supply_host_utc(UTC + 50);
    let time = machine.rtc.time();
    let battery = machine.rtc.battery_data();
    machine.load_state(&bare);
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(0xFF)
    );
    machine.load_state(&saved);
    assert_eq!(machine.rtc.time(), time);
    assert_eq!(machine.rtc.battery_data(), battery);
    assert_eq!(
        machine.memory_bus().read_byte(0xDC0001),
        BusResult::Ready(8)
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
