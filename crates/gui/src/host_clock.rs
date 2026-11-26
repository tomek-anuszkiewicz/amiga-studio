//! Host UTC sampling and local save time labels.

#[cfg(target_os = "linux")]
#[path = "host_clock/linux.rs"]
mod linux;
#[cfg(target_os = "macos")]
#[path = "host_clock/macos.rs"]
mod macos;
#[cfg(windows)]
#[path = "host_clock/windows.rs"]
pub mod windows;

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
pub struct SaveTimes {
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
            let local_times = local_save_times(&missing);
            let labels = missing.into_iter().zip(local_times).collect();
            let _ = sender.try_send(labels);
            ctx.request_repaint();
        })
        .detach();
    }
}

/// Use the desktop's regional formatter and time zone rules at each saved instant.
#[cfg(not(target_arch = "wasm32"))]
pub fn local_save_times(timestamps: &[u64]) -> Vec<Option<String>> {
    #[cfg(windows)]
    {
        windows::local_save_times(timestamps)
    }
    #[cfg(target_os = "macos")]
    {
        macos::local_save_times(timestamps)
    }
    #[cfg(target_os = "linux")]
    {
        linux::local_save_times(timestamps)
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        vec![None; timestamps.len()]
    }
}
