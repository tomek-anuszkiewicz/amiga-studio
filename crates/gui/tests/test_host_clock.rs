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
