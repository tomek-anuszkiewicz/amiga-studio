use windows::core::HSTRING;
use windows::Foundation::DateTime;
use windows::Globalization::DateTimeFormatting::DateTimeFormatter;

pub(super) fn local_save_times(timestamps: &[u64]) -> Vec<Option<String>> {
    let formatters = DateTimeFormatter::ShortDate().and_then(|date| {
        let time = DateTimeFormatter::LongTime()?;
        Ok((date, time))
    });
    timestamps
        .iter()
        .map(|&seconds| {
            let (date, time) = formatters.as_ref().ok()?;
            format_save_time(seconds, date, time, None)
        })
        .collect()
}

pub(crate) fn format_save_time(
    seconds: u64,
    date: &DateTimeFormatter,
    time: &DateTimeFormatter,
    zone: Option<&HSTRING>,
) -> Option<String> {
    const SECONDS_FROM_1601_TO_1970: i64 = 11_644_473_600;
    const TICKS_PER_SECOND: i64 = 10_000_000;
    let instant = DateTime {
        UniversalTime: i64::try_from(seconds)
            .ok()?
            .checked_add(SECONDS_FROM_1601_TO_1970)?
            .checked_mul(TICKS_PER_SECOND)?,
    };
    let format = |formatter: &DateTimeFormatter| match zone {
        Some(zone) => formatter.FormatUsingTimeZone(instant, zone).ok(),
        None => formatter.Format(instant).ok(),
    };
    let date = format(date)?;
    let time = format(time)?;
    let label = format!("{date} {time}");
    let label = label.trim();
    (!label.is_empty()).then(|| label.to_owned())
}
