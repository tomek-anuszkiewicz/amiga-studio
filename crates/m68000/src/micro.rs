//! Motorola 68000 CPU Micro-Step State Machine Subsystem
//!
//! Models 2-phase Color Clock execution (CCK1 and CCK2) per 4-clock CPU bus cycle,
//! driving atomic MicroSteps directly from pre-compiled slices without dynamic branching.

pub mod common;
pub mod dispatch_table;
pub mod ea;
pub mod engine;
pub mod types;

pub use common::*;
pub use dispatch_table::*;
pub use ea::*;
pub use engine::*;
pub use types::*;

