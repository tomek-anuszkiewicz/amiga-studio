use objc2::rc::autoreleasepool;
use objc2_foundation::{NSDate, NSDateFormatter, NSDateFormatterStyle, NSLocale, NSTimeZone};

pub(super) fn local_save_times(timestamps: &[u64]) -> Vec<Option<String>> {
    autoreleasepool(|_| {
        let formatter = NSDateFormatter::new();
        formatter.setLocale(Some(&NSLocale::currentLocale()));
        formatter.setTimeZone(Some(&NSTimeZone::localTimeZone()));
        formatter.setDateStyle(NSDateFormatterStyle::ShortStyle);
        formatter.setTimeStyle(NSDateFormatterStyle::MediumStyle);

        timestamps
            .iter()
            .map(|&seconds| {
                let date = NSDate::dateWithTimeIntervalSince1970(seconds as f64);
                let label = formatter.stringFromDate(&date).to_string();
                (!label.trim().is_empty()).then_some(label)
            })
            .collect()
    })
}
