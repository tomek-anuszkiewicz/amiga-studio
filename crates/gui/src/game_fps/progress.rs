//! Conditions for recognizing Mixed Scene startup and rejecting interrupted sessions.

use serde::Serialize;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Default)]
pub(super) struct StartupFocus {
    focused_since: Option<Instant>,
}

#[cfg(not(target_arch = "wasm32"))]
impl StartupFocus {
    pub(super) fn observe(&mut self, focused: bool, now: Instant) -> bool {
        if !focused {
            self.focused_since = None;
            return false;
        }
        let since = *self.focused_since.get_or_insert(now);
        now.duration_since(since) >= Duration::from_millis(300)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(super) struct Progress {
    pub(super) fields: u32,
    pub(super) work_loops: u32,
    pub(super) fault: u32,
    pub(super) cck: u64,
    pub(super) field_number: u64,
}

impl Progress {
    pub(super) fn advances(self, earlier: Self) -> bool {
        self.fields > earlier.fields
            && self.work_loops > earlier.work_loops
            && self.cck > earlier.cck
            && self.field_number > earlier.field_number
            && self.fault == 0
            && earlier.fault == 0
    }
}

#[derive(Debug, Default)]
pub(super) struct Boot {
    first: Option<Progress>,
}

impl Boot {
    pub(super) fn observe(
        &mut self,
        signature_matches: bool,
        progress: Progress,
    ) -> Result<Option<(Progress, Progress)>, String> {
        if !signature_matches {
            self.first = None;
            return Ok(None);
        }
        if progress.fault != 0 {
            return Err("Mixed Scene reported a fault during boot".into());
        }
        let first = *self.first.get_or_insert(progress);
        Ok(progress.advances(first).then_some((first, progress)))
    }
}

pub(super) fn validate_session(
    generation: Option<u64>,
    current_generation: u64,
    focused: bool,
) -> Result<(), String> {
    if !focused {
        return Err("The Game window lost focus".into());
    }
    if generation.is_some_and(|generation| generation != current_generation) {
        return Err("Machine generation changed during the benchmark".into());
    }
    Ok(())
}
