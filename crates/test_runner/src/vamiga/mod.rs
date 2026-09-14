//! vAmigaTS Automated Test Suite Execution Harness & Silicon Verification Module
//!
//! Provides direct-injection ADF payload extraction, stub OS vector tables,
//! and golden 716 x 285 RGB24 reference frame comparisons against physical silicon test captures.

pub mod injector;
pub mod matcher;
pub mod runner;

pub use injector::{
    inject_vamiga_test, setup_stub_os_vectors, ADF_PAYLOAD_OFFSET, MAX_PAYLOAD_SIZE,
    STUB_EXEC_BASE, STUB_GFX_BASE, VAMIGA_ENTRY_POINT, VAMIGA_STACK_POINTER,
};
pub use matcher::{
    compare_raw_frames, VamigaDiff, VamigaTestResult, VAMIGA_RAW_BYTE_SIZE, VAMIGA_RAW_HEIGHT,
    VAMIGA_RAW_PIXELS, VAMIGA_RAW_WIDTH,
};
pub use runner::{run_vamiga_test_buffers, run_vamiga_test_from_dir, VamigaRunConfig};
