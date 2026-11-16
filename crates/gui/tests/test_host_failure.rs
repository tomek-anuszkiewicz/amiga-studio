#![allow(clippy::unwrap_used)]
mod host_runtime {
    pub use gui::host_runtime::HostRuntimeError;
}
#[path = "../src/host_failure.rs"]
mod host_failure;
use host_failure::HostFailure;
use host_runtime::HostRuntimeError;

#[test]
fn completion_after_the_initial_flag_read_preserves_the_first_diagnostic() {
    let failure = HostFailure::default();
    assert!(!failure.is_failed());
    let error = failure
        .poll(|| {
            // This callback runs after poll's first flag read and before its terminal
            // classification, forcing the interleaving without a timing lottery.
            failure.record("original invariant diagnostic".into());
            failure.record("later failure must not replace the original".into());
            true
        })
        .unwrap();
    assert_eq!(
        error,
        HostRuntimeError::Fatal("original invariant diagnostic".into())
    );
    assert_eq!(failure.error(), error);
    assert_eq!(failure.poll(|| true), Some(error));
}

#[test]
fn generic_failure_and_healthy_running_are_distinct() {
    let failure = HostFailure::default();
    assert_eq!(failure.poll(|| false), None);
    assert_eq!(failure.poll(|| true), Some(HostRuntimeError::WorkerStopped));
    failure.mark_failed();
    assert_eq!(
        failure.poll(|| false),
        Some(HostRuntimeError::WorkerStopped)
    );
}
