use config::RtcModel;
use rtc::{RtcBattery, RtcMsm6242b};

const UTC: i64 = 631152000;

#[test]
fn guest_twelve_hour_calendar_writes_persist_as_a_host_relative_offset() {
    // 2026-11-14 at 00:34:56, 12:34:56, 03:34:56 and 15:34:56 UTC.
    for (hour_units, hour_tens, hour) in [(2, 1, 0), (2, 5, 12), (3, 0, 3), (3, 4, 15)] {
        let mut rtc = RtcMsm6242b::default();
        rtc.write_byte(0xDC0035, 1); // Hold calendar latches during guest writes.
        rtc.supply_host_utc(UTC + 100);
        rtc.write_byte(0xDC003D, 0); // 12-hour mode; HOLD remains set.
        let digits = [6, 5, 4, 3, hour_units, hour_tens, 4, 1, 1, 1, 6, 2, 6];
        for (index, digit) in digits.into_iter().enumerate() {
            rtc.write_byte(0xDC0001 + index as u32 * 4, digit);
        }
        let expected = 1794614400 + hour * 3600 + 34 * 60 + 56;
        let offset = expected - (UTC + 100);
        assert_eq!(rtc.time(), expected);
        assert_eq!(rtc.battery_data().offset_seconds, offset);
        assert_eq!(&rtc.registers[..13], &digits);
        rtc.write_byte(0xDC0035, 0); // Release HOLD.
        rtc.supply_host_utc(UTC + 107);
        assert_eq!(rtc.time(), expected + 7);
        assert_eq!(rtc.battery_data().offset_seconds, offset);
        assert_eq!(rtc.read_byte(0xDC0001), 3);
        assert_eq!(rtc.read_byte(0xDC0009), 5);

        let mut restarted = RtcMsm6242b::default();
        restarted.supply_host_utc(UTC + 1100);
        restarted.load_battery(rtc.battery_data());
        assert_eq!(restarted.time(), expected + 1000);
        assert_eq!(restarted.battery_data().offset_seconds, offset);
        // The restarted RTC defaults to 24-hour display, now at HH:51:36.
        assert_eq!(restarted.read_byte(0xDC0011), (hour % 10) as u8);
        assert_eq!(restarted.read_byte(0xDC0015), (hour / 10) as u8);
        assert_eq!(restarted.read_byte(0xDC0009), 1);
        assert_eq!(restarted.read_byte(0xDC000D), 5);
        assert_eq!(restarted.read_byte(0xDC0001), 6);
        assert_eq!(restarted.read_byte(0xDC0005), 3);
    }
}

#[test]
fn control_writes_adjust_current_time_and_hour_encoding() {
    let mut rtc = RtcMsm6242b::new(RtcModel::Msm6242b);
    rtc.write_byte(0xDC0035, 0);
    rtc.supply_host_utc(UTC + 12 * 3600 + 30);
    rtc.write_byte(0xDC0035, 8); // Round 12:00:30 up to 12:01:00.
    assert_eq!(rtc.read_byte(0xDC0001), 0);
    assert_eq!(rtc.read_byte(0xDC0009), 1);
    assert_eq!(rtc.battery_data().offset_seconds, 30);
    rtc.write_byte(0xDC003D, 0); // Encode noon in 12-hour mode.
    assert_eq!(rtc.read_byte(0xDC0011), 2);
    assert_eq!(rtc.read_byte(0xDC0015), 5);
    rtc.supply_host_utc(UTC + 12 * 3600 + 35);
    rtc.write_byte(0xDC0035, 8); // Round 12:01:05 down.
    assert_eq!(rtc.time(), UTC + 12 * 3600 + 60);
    assert_eq!(rtc.battery_data().offset_seconds, 25);
}

#[test]
fn battery_offset_uses_new_host_time_after_restart() {
    let mut rtc = RtcMsm6242b::default();
    rtc.supply_host_utc(UTC + 100);
    rtc.set_time(UTC + 40);
    let battery = rtc.battery_data();
    assert_eq!(
        battery,
        RtcBattery {
            offset_seconds: -60
        }
    );
    let mut restarted = RtcMsm6242b::default();
    restarted.supply_host_utc(UTC + 1000);
    restarted.load_battery(battery);
    assert_eq!(restarted.time(), UTC + 940);
    assert_eq!(restarted.read_byte(0xDC0001), 0);
    assert_eq!(restarted.read_byte(0xDC0005), 4);
    assert_eq!(restarted.read_byte(0xDC0009), 5);
}

#[test]
fn host_utc_calendar_rolls_over_without_timezone_conversion() {
    let mut rtc = RtcMsm6242b::default();
    rtc.write_byte(0xDC0035, 0);
    // 2026-11-14 23:59:59 UTC, Saturday.
    rtc.supply_host_utc(1794700799);
    assert_eq!(
        &rtc.registers[..13],
        &[9, 5, 9, 5, 3, 2, 4, 1, 1, 1, 6, 2, 6]
    );
    rtc.supply_host_utc(1794700800);
    assert_eq!(
        &rtc.registers[..13],
        &[0, 0, 0, 0, 0, 0, 5, 1, 1, 1, 6, 2, 0]
    );
}

#[test]
fn host_utc_calendar_preserves_february_and_leap_day() {
    // UTC midnight dates, including the final day of a 400-year March-based era.
    let cases = [
        (633830400, [0, 0, 0, 0, 0, 0, 1, 0, 2, 0, 0, 9, 4]), // 1990-02-01
        (951696000, [0, 0, 0, 0, 0, 0, 8, 2, 2, 0, 0, 0, 1]), // 2000-02-28
        (951782400, [0, 0, 0, 0, 0, 0, 9, 2, 2, 0, 0, 0, 2]), // 2000-02-29
        (951868800, [0, 0, 0, 0, 0, 0, 1, 0, 3, 0, 0, 0, 3]), // 2000-03-01
        (1772236800, [0, 0, 0, 0, 0, 0, 8, 2, 2, 0, 6, 2, 6]), // 2026-02-28
        (1772323200, [0, 0, 0, 0, 0, 0, 1, 0, 3, 0, 6, 2, 0]), // 2026-03-01
    ];
    let mut rtc = RtcMsm6242b::default();
    rtc.write_byte(0xDC0035, 0);
    for (timestamp, registers) in cases {
        rtc.supply_host_utc(timestamp);
        assert_eq!(&rtc.registers[..13], &registers, "UTC {timestamp}");
        // Writing an unchanged calendar digit must preserve the represented date.
        rtc.write_byte(0xDC0019, registers[6]);
        assert_eq!(rtc.time(), timestamp);
        assert_eq!(rtc.battery_data().offset_seconds, 0);
    }
}
