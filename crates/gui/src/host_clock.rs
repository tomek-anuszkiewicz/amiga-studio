//! Host UTC sampling and whole-second conversion, separate from playback pacing.

pub fn utc_now() -> i64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        utc_seconds(std::time::SystemTime::now())
    }
    #[cfg(target_arch = "wasm32")]
    {
        const MILLISECONDS_PER_SECOND: f64 = 1000.0;
        (js_sys::Date::now() / MILLISECONDS_PER_SECOND).floor() as i64
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn utc_seconds(timestamp: std::time::SystemTime) -> i64 {
    match timestamp.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(error) => {
            let duration = error.duration();
            // A negative fractional timestamp belongs to the preceding second.
            (duration.as_secs() as i64)
                .wrapping_neg()
                .wrapping_sub(i64::from(duration.subsec_nanos() != 0))
        }
    }
}

/// Readable save time, independent of the guest RTC battery offset.
#[cfg(not(target_arch = "wasm32"))]
pub fn format_utc(timestamp: u64) -> String {
    let Ok(timestamp) = i64::try_from(timestamp) else {
        return "Invalid save time".into();
    };
    let (year, month, day, hour, minute, second, _) = rtc::decompose_timestamp(timestamp);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} UTC")
}
