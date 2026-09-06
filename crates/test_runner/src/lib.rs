//! SingleStepTest Runner for M68000 validation against MAME and Tom Harte suites

pub mod runner;
pub mod schema;

pub use runner::{run_single_test, run_test_file};
pub use schema::{CpuTestState, SingleStepTest};
