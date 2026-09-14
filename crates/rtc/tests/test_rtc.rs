//! Unit tests for OKI MSM6242B Real-Time Clock logic in isolation

use config::RtcModel;
use rtc::RtcMsm6242b;

#[test]
fn test_rtc_none_model_reads_open_bus() {
    let rtc = RtcMsm6242b::new(RtcModel::None);
    for reg in 0..16 {
        assert_eq!(rtc.read_byte(0xDC0001 + reg * 4), 0xFF);
        assert_eq!(rtc.read_byte(0xDC0000 + reg * 4), 0xFF);
    }
}

#[test]
fn test_rtc_even_address_reads_open_bus() {
    let rtc = RtcMsm6242b::new(RtcModel::Msm6242b);
    for reg in 0..16 {
        assert_eq!(rtc.read_byte(0xDC0000 + reg * 4), 0xFF);
        assert_eq!(rtc.read_byte(0xDC0002 + reg * 4), 0xFF);
    }
}

#[test]
fn test_rtc_timestamp_bcd_encoding() {
    let mut rtc = RtcMsm6242b::new(RtcModel::Msm6242b);
    // 1993-03-15 14:27:08 UTC = 732205628 (Monday)
    rtc.set_time(732205628);

    assert_eq!(rtc.read_byte(0xDC0001), 8); // 1s sec
    assert_eq!(rtc.read_byte(0xDC0005), 0); // 10s sec
    assert_eq!(rtc.read_byte(0xDC0009), 7); // 1s min
    assert_eq!(rtc.read_byte(0xDC000D), 2); // 10s min
    assert_eq!(rtc.read_byte(0xDC0011), 4); // 1s hr
    assert_eq!(rtc.read_byte(0xDC0015), 1); // 10s hr
    assert_eq!(rtc.read_byte(0xDC0019), 5); // 1s day
    assert_eq!(rtc.read_byte(0xDC001D), 1); // 10s day
    assert_eq!(rtc.read_byte(0xDC0021), 3); // 1s mon
    assert_eq!(rtc.read_byte(0xDC0025), 0); // 10s mon
    assert_eq!(rtc.read_byte(0xDC0029), 3); // 1s yr
    assert_eq!(rtc.read_byte(0xDC002D), 9); // 10s yr
    assert_eq!(rtc.read_byte(0xDC0031), 1); // day of week (Monday = 1)
}

#[test]
fn test_rtc_12_hour_am_pm_mode() {
    let mut rtc = RtcMsm6242b::new(RtcModel::Msm6242b);
    // 14:27:08 (2:27:08 PM)
    rtc.set_time(732205628);

    // Switch to 12-hour mode by clearing Bit 2 of Control Register F ($DC003D)
    let ctrl_f = rtc.read_byte(0xDC003D);
    rtc.write_byte(0xDC003D, ctrl_f & !0x04);

    // Reg 4 = 2, Reg 5 = PM bit (0x04)
    assert_eq!(rtc.read_byte(0xDC0011), 2);
    assert_eq!(rtc.read_byte(0xDC0015), 0x04);

    // Midnight 00:15:00 should be 12:15 AM
    rtc.set_time(732154500);
    assert_eq!(rtc.read_byte(0xDC0011), 2);
    assert_eq!(rtc.read_byte(0xDC0015), 1); // 12 AM (PM bit = 0)
}

#[test]
fn test_rtc_hold_latch_freezing() {
    let mut rtc = RtcMsm6242b::new(RtcModel::Msm6242b);
    rtc.set_time(732196028); // sec = 08

    // Engage HOLD (Bit 0 of Control Register D, $DC0035)
    rtc.write_byte(0xDC0035, 0x01);

    // Step clock by 5 seconds
    rtc.step_cck(5 * 3_546_895);

    // Reads while HOLD is set remain frozen
    assert_eq!(rtc.read_byte(0xDC0001), 8);
    assert_eq!(rtc.read_byte(0xDC0005), 0);

    // Release HOLD
    rtc.write_byte(0xDC0035, 0x00);

    // Reads now reflect updated time (08 + 5 = 13)
    assert_eq!(rtc.read_byte(0xDC0001), 3);
    assert_eq!(rtc.read_byte(0xDC0005), 1);
}

#[test]
fn test_rtc_step_cck_timing() {
    let mut rtc = RtcMsm6242b::new(RtcModel::Msm6242b);
    rtc.write_byte(0xDC0035, 0x00); // Clear HOLD
    rtc.set_time(732196028);

    // 1 CCK short of 1 full second
    rtc.step_cck(3_546_894);
    assert_eq!(rtc.read_byte(0xDC0001), 8);

    // Final CCK
    rtc.step_cck(1);
    assert_eq!(rtc.read_byte(0xDC0001), 9);
}

#[test]
fn test_rtc_sync_registers_and_time_methods() {
    let mut rtc = RtcMsm6242b::new(RtcModel::Msm6242b);
    rtc.simulated_time = 732205628;
    rtc.sync_time_to_registers();

    // Verify registers updated from time
    assert_eq!(rtc.registers[0], 8); // sec 1s
    assert_eq!(rtc.registers[1], 0); // sec 10s

    // Modify registers directly and sync back to time
    rtc.registers[0] = 9;
    rtc.sync_registers_to_time();
    assert_eq!(rtc.simulated_time, 732205629);
}
