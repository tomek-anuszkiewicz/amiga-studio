use config::RtcModel;
use rtc::RtcMsm6242b;

const CCK_PER_SECOND: u64 = 3_546_895;

#[test]
fn control_writes_adjust_current_time_and_hour_encoding() {
    let mut rtc = RtcMsm6242b::new(RtcModel::Msm6242b);
    rtc.write_byte(0xDC0035, 0);
    let cck = 12 * 3600 * CCK_PER_SECOND + 30 * CCK_PER_SECOND;
    rtc.step_cck(cck);
    rtc.write_byte(0xDC0035, 8); // Round 12:00:30 up to 12:01:00.
    assert_eq!(rtc.read_byte(0xDC0001), 0);
    assert_eq!(rtc.read_byte(0xDC0009), 1);
    rtc.write_byte(0xDC003D, 0); // Encode noon in 12-hour mode.
    assert_eq!(rtc.read_byte(0xDC0011), 2);
    assert_eq!(rtc.read_byte(0xDC0015), 5); // PM | tens digit 1.
}

#[test]
fn setting_time_preserves_fractional_cycles() {
    let mut rtc = RtcMsm6242b::new(RtcModel::Msm6242b);
    rtc.write_byte(0xDC0035, 0);
    rtc.step_cck(2 * CCK_PER_SECOND - 1);
    rtc.set_time(631152008);
    rtc.step_cck(1);
    assert_eq!(rtc.read_byte(0xDC0001), 9);
    assert_eq!(rtc.cck_accumulator, 0);
}

#[test]
fn clock_preserves_fraction_across_large_elapsed_intervals() {
    let mut rtc = RtcMsm6242b::new(RtcModel::Msm6242b);
    rtc.write_byte(0xDC0035, 0);
    rtc.step_cck(123_456 * CCK_PER_SECOND + 17);
    rtc.set_time(631152008);
    rtc.step_cck(CCK_PER_SECOND - 18);
    assert_eq!(rtc.read_byte(0xDC0001), 8);
    rtc.step_cck(1);
    assert_eq!(rtc.read_byte(0xDC0001), 9);
    assert_eq!(rtc.cck_accumulator, 0);
}
