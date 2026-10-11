//! Shared CPU verification helpers compiled into each integration test target.

// Each suite uses a different subset of the shared harness.
#![allow(dead_code)]

pub mod diagnostic;
pub mod reporting;
pub mod schema;
pub mod singlestep_runner;
pub mod singlestep_suite;
pub mod singlestep_vectors;
pub mod test_memory_bus;
pub mod transactions;
