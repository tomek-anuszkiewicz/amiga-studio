#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Unit tests for M68000 Benchmark Platform Abstraction & Affinity Pinning
//!
//! Validates HostEnvironment detection, platform name identification,
//! and Performance Core (P-core) affinity pinning without panicking.

use test_runner::benchmark::platform::HostEnvironment;

#[test]
fn test_platform_detection() {
    let env = HostEnvironment::detect();

    assert!(
        !env.platform_name.is_empty(),
        "Platform name must not be empty"
    );

    #[cfg(target_arch = "wasm32")]
    {
        assert_eq!(env.platform_name, "WebAssembly");
        assert!(!env.affinity_supported);
    }

    #[cfg(all(windows, not(target_arch = "wasm32")))]
    {
        assert_eq!(env.platform_name, "Windows");
        assert!(env.affinity_supported);
        assert!(env.p_core_id.is_some());
    }

    #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
    {
        assert_eq!(env.platform_name, "Linux");
        assert!(env.affinity_supported);
    }

    #[cfg(all(target_os = "macos", not(target_arch = "wasm32")))]
    {
        assert_eq!(env.platform_name, "macOS");
        assert!(env.affinity_supported);
    }
}

#[test]
fn test_platform_default_matches_detect() {
    let default_env = HostEnvironment::default();
    let detected_env = HostEnvironment::detect();

    assert_eq!(default_env.platform_name, detected_env.platform_name);
    assert_eq!(
        default_env.affinity_supported,
        detected_env.affinity_supported
    );
    assert_eq!(default_env.p_core_detected, detected_env.p_core_detected);
    assert_eq!(default_env.p_core_id, detected_env.p_core_id);
}

#[test]
fn test_platform_pin_to_p_core_execution() {
    let env = HostEnvironment::detect();
    let result = env.pin_to_p_core();

    // Pinning should succeed on host OS
    assert!(result.is_ok(), "pin_to_p_core failed: {:?}", result.err());
}
