use std::time::{Duration, UNIX_EPOCH};

#[path = "../src/host_clock.rs"]
#[allow(dead_code)]
mod host_clock;

#[test]
fn native_utc_seconds_floor_on_both_sides_of_the_epoch() {
    for (milliseconds, before, after) in [
        (0, 0, 0),
        (1, -1, 0),
        (999, -1, 0),
        (1000, -1, 1),
        (1001, -2, 1),
        (123456, -124, 123),
    ] {
        let duration = Duration::from_millis(milliseconds);
        assert_eq!(host_clock::utc_seconds(UNIX_EPOCH - duration), before);
        assert_eq!(host_clock::utc_seconds(UNIX_EPOCH + duration), after);
    }
}

#[cfg(windows)]
#[test]
fn windows_save_time_uses_the_current_regional_settings() {
    use windows::Foundation::DateTime;
    use windows::Globalization::DateTimeFormatting::DateTimeFormatter;

    // The same saved instant, expressed in Windows ticks since January 1601.
    let instant = DateTime {
        UniversalTime: 133_545_275_400_000_000,
    };
    let date = DateTimeFormatter::ShortDate()
        .unwrap()
        .Format(instant)
        .unwrap();
    let time = DateTimeFormatter::LongTime()
        .unwrap()
        .Format(instant)
        .unwrap();
    let expected = format!("{date} {time}");
    assert_eq!(
        host_clock::local_save_times(&[1_710_053_940, u64::MAX]),
        [Some(expected.trim().to_owned()), None]
    );
}

#[cfg(windows)]
#[test]
fn regional_save_time_uses_the_clock_style_and_offset_at_each_saved_instant() {
    use windows::core::HSTRING;
    use windows::Globalization::DateTimeFormatting::DateTimeFormatter;
    use windows::Globalization::{CalendarIdentifiers, ClockIdentifiers};

    // New York changed from 01:59 to 03:01 in March 2024. In 2006, the March instant
    // still used standard time because the spring transition took place in April.
    let instants = [1_710_053_940, 1_710_054_060, 1_142_193_600];
    let zone = HSTRING::from("America/New_York");
    let calendar = CalendarIdentifiers::Gregorian().unwrap();
    // Pin the date and time patterns to exercise both preferences on any Windows profile.
    let format = |language, region, clock: HSTRING, date_pattern, time_pattern| {
        let languages = vec![HSTRING::from(language)].into();
        let formatter = |style| {
            DateTimeFormatter::CreateDateTimeFormatterContext(
                &HSTRING::from(style),
                Some(&languages),
                &HSTRING::from(region),
                &calendar,
                &clock,
            )
            .unwrap()
        };
        let date = formatter(date_pattern);
        let time = formatter(time_pattern);
        instants.map(|seconds| {
            host_clock::windows::format_save_time(seconds, &date, &time, Some(&zone))
                .unwrap()
                .replace('\u{200e}', "")
        })
    };
    assert_eq!(
        format(
            "en-US",
            "US",
            ClockIdentifiers::TwelveHour().unwrap(),
            "{month.integer}/{day.integer}/{year.full}",
            "{hour.integer}:{minute.integer(2)}:{second.integer(2)} {period.abbreviated}",
        ),
        [
            "3/10/2024 1:59:00 AM",
            "3/10/2024 3:01:00 AM",
            "3/12/2006 3:00:00 PM"
        ]
    );
    assert_eq!(
        format(
            "de-DE",
            "DE",
            ClockIdentifiers::TwentyFourHour().unwrap(),
            "{day.integer(2)}.{month.integer(2)}.{year.full}",
            "{hour.integer(2)}:{minute.integer(2)}:{second.integer(2)}",
        ),
        [
            "10.03.2024 01:59:00",
            "10.03.2024 03:01:00",
            "12.03.2006 15:00:00"
        ]
    );
}
