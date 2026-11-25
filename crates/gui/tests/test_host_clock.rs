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
fn regional_save_time_uses_the_clock_style_and_offset_at_each_saved_instant() {
    // New York changes from 01:59 to 03:01 at the 2024 spring transition.
    let instants = [1_710_053_940, 1_710_054_060];
    let zone = "$zone = [TimeZoneInfo]::FindSystemTimeZoneById('Eastern Standard Time')";
    let format = |culture| {
        host_clock::windows_save_times(
            &instants,
            &format!(
                "$culture = New-Object Globalization.CultureInfo('{culture}', $false)\n{zone}"
            ),
        )
    };
    assert_eq!(
        format("en-US"),
        [
            Some("3/10/2024 1:59:00 AM".to_owned()),
            Some("3/10/2024 3:01:00 AM".to_owned())
        ]
    );
    assert_eq!(
        format("de-DE"),
        [
            Some("10.03.2024 01:59:00".to_owned()),
            Some("10.03.2024 03:01:00".to_owned())
        ]
    );
}
