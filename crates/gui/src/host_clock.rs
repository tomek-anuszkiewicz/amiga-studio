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

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Default)]
pub(crate) struct SaveTimes {
    labels: std::collections::BTreeMap<u64, Option<String>>,
    job: Option<async_channel::Receiver<Vec<(u64, Option<String>)>>>,
    dirty: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl SaveTimes {
    pub fn changed(&mut self) {
        self.dirty = true;
    }

    pub fn label(&self, time: u64) -> &str {
        match self.labels.get(&time) {
            Some(Some(label)) => label,
            Some(None) => "Local save time unavailable",
            None => "Reading local save time…",
        }
    }

    pub fn poll<I: IntoIterator<Item = u64>>(
        &mut self,
        ctx: &egui::Context,
        timestamps: impl FnOnce() -> I,
    ) {
        if let Some(job) = &self.job {
            match job.try_recv() {
                Ok(labels) => self.labels.extend(labels),
                Err(async_channel::TryRecvError::Closed) => self.dirty = true,
                Err(async_channel::TryRecvError::Empty) => return,
            }
            self.job = None;
        }
        if !std::mem::take(&mut self.dirty) {
            return;
        }
        let timestamps: std::collections::BTreeSet<_> = timestamps().into_iter().collect();
        self.labels.retain(|time, _| timestamps.contains(time));
        let missing: Vec<_> = timestamps
            .into_iter()
            .filter(|time| !self.labels.contains_key(time))
            .collect();
        if missing.is_empty() {
            return;
        }
        let (sender, receiver) = async_channel::bounded(1);
        self.job = Some(receiver);
        let ctx = ctx.clone();
        blocking::unblock(move || {
            let mut labels = Vec::with_capacity(missing.len());
            for batch in missing.chunks(64) {
                labels.extend(batch.iter().copied().zip(local_save_times(batch)));
            }
            let _ = sender.try_send(labels);
            ctx.request_repaint();
        })
        .detach();
    }
}

/// Use the desktop's regional formatter and time zone rules at each saved instant.
#[cfg(not(target_arch = "wasm32"))]
fn local_save_times(timestamps: &[u64]) -> Vec<Option<String>> {
    #[cfg(windows)]
    {
        windows_save_times(
            timestamps,
            "$culture = [Globalization.CultureInfo]::CurrentCulture\n$zone = [TimeZoneInfo]::Local",
        )
    }
    #[cfg(target_os = "macos")]
    {
        // Foundation uses Region preferences even when a Finder launch has no LC_TIME.
        let script = r#"
function run(timestamps) {
    ObjC.import('Foundation');
    const formatter = $.NSDateFormatter.alloc.init;
    formatter.locale = $.NSLocale.currentLocale;
    formatter.timeZone = $.NSTimeZone.localTimeZone;
    formatter.dateStyle = 1;
    formatter.timeStyle = 2;
    return timestamps.map(seconds => {
        const date = $.NSDate.dateWithTimeIntervalSince1970(Number(seconds));
        return ObjC.unwrap(formatter.stringFromDate(date));
    }).join('\n');
}
"#;
        let output = std::process::Command::new("osascript")
            .args(["-l", "JavaScript", "-e", script])
            .args(timestamps.iter().map(u64::to_string))
            .output();
        save_time_output(output, timestamps.len())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        timestamps
            .iter()
            .map(|time| {
                let output = std::process::Command::new("date")
                    .args(["-d", &format!("@{time}"), "+%x %X"])
                    .output();
                save_time_output(output, 1).pop().flatten()
            })
            .collect()
    }
    #[cfg(not(any(unix, windows)))]
    {
        vec![None; timestamps.len()]
    }
}

#[cfg(windows)]
pub(crate) fn windows_save_times(timestamps: &[u64], preferences: &str) -> Vec<Option<String>> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let list = timestamps
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let script = format!(
        r#"
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
{preferences}
foreach ($seconds in @({list})) {{
try {{
    [TimeZoneInfo]::ConvertTime([DateTimeOffset]::FromUnixTimeSeconds([long]$seconds), $zone).ToString('G', $culture)
}} catch {{
    ''
}}
}}
"#
    );
    let output = std::process::Command::new("powershell.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .output();
    save_time_output(output, timestamps.len())
}

#[cfg(all(not(target_arch = "wasm32"), any(unix, windows)))]
fn save_time_output(
    output: std::io::Result<std::process::Output>,
    expected: usize,
) -> Vec<Option<String>> {
    let lines = output
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|output| {
            output
                .lines()
                .map(|line| (!line.trim().is_empty()).then(|| line.trim().to_owned()))
                .collect::<Vec<_>>()
        });
    lines
        .filter(|lines| lines.len() == expected)
        .unwrap_or_else(|| vec![None; expected])
}
