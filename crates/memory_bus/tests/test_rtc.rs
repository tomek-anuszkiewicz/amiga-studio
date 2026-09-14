//! Unit tests for Real-Time Clock space in PhysicalMemory
//!
//! Verifies that PhysicalMemory treats Bank 0xDC as open bus space (returning $FF)
//! with silent no-op writes, and verifies crate re-exports.

use memory_bus::{A500Config, MemoryBus, RtcModel, VideoStandard};

#[test]
fn test_rtc_open_bus_in_physical_memory() {
    let config = A500Config::standard_1mb(VideoStandard::Pal);
    let mut bus = MemoryBus::from_config(config);

    assert_eq!(bus.config.rtc(), RtcModel::Msm6242b);

    // In PhysicalMemory, Bank 0xDC space returns open bus $FF on all addresses
    assert_eq!(bus.read_byte_debug(0xDC0000), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0001), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0004), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0005), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC003C), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC003D), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC0040), 0xFF);
    assert_eq!(bus.read_byte_debug(0xDC1000), 0xFF);

    // Writes are silent no-ops
    bus.write_byte_debug(0xDC0001, 0x05);
    assert_eq!(bus.read_byte_debug(0xDC0001), 0xFF);
}

#[test]
fn test_rtc_reexported_module_namespace() {
    // Verify that downstream callers can construct or type-check RtcMsm6242b
    // directly via memory_bus::rtc::RtcMsm6242b without depending directly on crate rtc
    let rtc_instance = memory_bus::rtc::RtcMsm6242b::new(memory_bus::RtcModel::Msm6242b);
    assert_eq!(rtc_instance.model, memory_bus::RtcModel::Msm6242b);
}
