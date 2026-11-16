//! First-failure publication and terminal polling, independent of owner locks.
use crate::host_runtime::HostRuntimeError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

#[derive(Debug, Default)]
pub struct HostFailure {
    failed: AtomicBool,
    message: OnceLock<String>,
}

impl HostFailure {
    pub(super) fn is_failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
    pub(super) fn record(&self, message: String) {
        let _ = self.message.set(message);
        self.mark_failed();
    }
    pub(super) fn mark_failed(&self) {
        self.failed.store(true, Ordering::Release);
    }
    pub(super) fn error(&self) -> HostRuntimeError {
        match self.message.get() {
            Some(message) => HostRuntimeError::Fatal(message.clone()),
            None => HostRuntimeError::WorkerStopped,
        }
    }
    pub(super) fn poll(&self, finished: impl FnOnce() -> bool) -> Option<HostRuntimeError> {
        if self.is_failed() {
            return Some(self.error());
        }
        // Completion may race the first flag read; its diagnostic is now published.
        if finished() {
            return Some(self.error());
        }
        None
    }
}
