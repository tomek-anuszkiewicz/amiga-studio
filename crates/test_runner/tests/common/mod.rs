//! Shared CPU verification helpers compiled into each integration test target.

// Each suite uses a different subset of the shared harness.
#![allow(dead_code)]

pub mod diagnostic;
pub mod reporter;
pub mod schema;
pub mod singlestep_runner;
pub mod test_memory_bus;
pub mod transactions;
