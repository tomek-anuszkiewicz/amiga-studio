#![allow(unsafe_code, clippy::cast_ptr_alignment)]

//! Host topology detection for handwritten benchmark affinity selection.
//!
//! The measurement runner applies and verifies its own host affinity policy.

#[derive(Debug, Clone)]
pub struct HostEnvironment {
    pub p_core_id: Option<usize>,
}

impl Default for HostEnvironment {
    fn default() -> Self {
        Self::detect()
    }
}

impl HostEnvironment {
    /// Dynamically discovers host CPU topology and identifies primary P-Core
    pub fn detect() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Self { p_core_id: None }
        }

        #[cfg(all(windows, not(target_arch = "wasm32")))]
        {
            let (_, p_core_id) = detect_windows_p_core();
            Self { p_core_id }
        }

        #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
        {
            let (_, p_core_id) = detect_linux_p_core();
            Self { p_core_id }
        }

        #[cfg(all(target_os = "macos", not(target_arch = "wasm32")))]
        {
            let (_, p_core_id) = detect_macos_p_core();
            Self { p_core_id }
        }

        #[cfg(all(
            not(windows),
            not(target_os = "linux"),
            not(target_os = "macos"),
            not(target_arch = "wasm32")
        ))]
        {
            Self { p_core_id: Some(0) }
        }
    }
}

// ---------------------------------------------------------------------------
// Windows Native Hardware Topology & Affinity Pinning
// ---------------------------------------------------------------------------

#[cfg(all(windows, not(target_arch = "wasm32")))]
mod win_ffi {
    pub const RELATION_PROCESSOR_CORE: u32 = 0;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct PROCESSOR_CORE_INFO {
        pub flags: u8,
        pub efficiency_class: u8,
        pub reserved: [u8; 20],
        pub group_count: u16,
        pub group_mask: [GROUP_AFFINITY; 1],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct GROUP_AFFINITY {
        pub mask: usize,
        pub group: u16,
        pub reserved: [u16; 3],
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX {
        pub relationship: u32,
        pub size: u32,
        pub core: PROCESSOR_CORE_INFO,
    }

    extern "system" {
        pub fn GetLogicalProcessorInformationEx(
            relationship_type: u32,
            buffer: *mut u8,
            returned_length: *mut u32,
        ) -> i32;
    }
}

#[cfg(all(windows, not(target_arch = "wasm32")))]
fn detect_windows_p_core() -> (bool, Option<usize>) {
    use win_ffi::{
        GetLogicalProcessorInformationEx, RELATION_PROCESSOR_CORE,
        SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
    };

    let mut len: u32 = 0;
    unsafe {
        GetLogicalProcessorInformationEx(RELATION_PROCESSOR_CORE, std::ptr::null_mut(), &mut len);
    }

    if len == 0 {
        return (false, Some(0));
    }

    let mut buf = vec![0u8; len as usize];
    let ok = unsafe {
        GetLogicalProcessorInformationEx(RELATION_PROCESSOR_CORE, buf.as_mut_ptr(), &mut len)
    };

    if ok == 0 {
        return (false, Some(0));
    }

    let mut offset = 0;
    let mut best_core: Option<usize> = None;
    let mut max_efficiency: u8 = 0;
    let mut has_hybrid = false;

    while offset < len as usize {
        let ptr =
            unsafe { buf.as_ptr().add(offset) as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX };
        let info = unsafe { &*ptr };

        if info.relationship == RELATION_PROCESSOR_CORE {
            let eff = info.core.efficiency_class;
            let mask = info.core.group_mask[0].mask;
            let core_idx = mask.trailing_zeros() as usize;

            if eff > 0 {
                has_hybrid = true;
            }
            if eff >= max_efficiency {
                max_efficiency = eff;
                best_core = Some(core_idx);
            }
        }

        if info.size == 0 {
            break;
        }
        offset += info.size as usize;
    }

    let chosen_core = best_core.unwrap_or(0);
    (has_hybrid, Some(chosen_core))
}

// ---------------------------------------------------------------------------
// Linux Native Hardware Topology & Affinity Pinning
// ---------------------------------------------------------------------------

#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
fn detect_linux_p_core() -> (bool, Option<usize>) {
    let mut best_core = 0;
    let mut max_freq = 0u64;
    let mut freqs = Vec::new();

    if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("cpu") && name_str[3..].chars().all(|c| c.is_ascii_digit()) {
                if let Ok(core_id) = name_str[3..].parse::<usize>() {
                    let freq_path = entry.path().join("cpufreq/cpuinfo_max_freq");
                    if let Ok(content) = std::fs::read_to_string(freq_path) {
                        if let Ok(freq) = content.trim().parse::<u64>() {
                            freqs.push((core_id, freq));
                            if freq > max_freq {
                                max_freq = freq;
                                best_core = core_id;
                            }
                        }
                    }
                }
            }
        }
    }

    let has_hybrid = freqs.iter().any(|&(_, f)| f < max_freq);
    (has_hybrid, Some(best_core))
}

// ---------------------------------------------------------------------------
// macOS Native P-Core Detection
// ---------------------------------------------------------------------------

#[cfg(all(target_os = "macos", not(target_arch = "wasm32")))]
fn detect_macos_p_core() -> (bool, Option<usize>) {
    // macOS Apple Silicon exposes hw.perflevel0 (P-Cores) and hw.perflevel1 (E-Cores)
    (true, Some(0))
}

#[cfg(windows)]
pub mod windows {
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn GetPriorityClass(process: *mut c_void) -> u32;
        fn GetCurrentThread() -> *mut c_void;
        fn SetThreadAffinityMask(thread: *mut c_void, mask: usize) -> usize;
    }

    pub struct AffinityGuard {
        previous: usize,
        pub core: usize,
        pub priority_class: u32,
    }

    impl AffinityGuard {
        pub fn pin(core: usize) -> Result<Self, String> {
            let mask = 1usize
                .checked_shl(core as u32)
                .ok_or("core exceeds affinity mask")?;
            let priority_class = unsafe { GetPriorityClass(GetCurrentProcess()) };
            if priority_class != 0x20 {
                return Err(format!(
                    "expected normal process priority, found {priority_class:#X}"
                ));
            }
            let previous = unsafe { SetThreadAffinityMask(GetCurrentThread(), mask) };
            if previous == 0 {
                return Err(format!("failed to pin logical CPU {core}"));
            }
            let confirmed_previous = unsafe { SetThreadAffinityMask(GetCurrentThread(), mask) };
            if confirmed_previous != mask {
                unsafe {
                    SetThreadAffinityMask(GetCurrentThread(), previous);
                }
                return Err(format!("logical CPU {core} affinity was not confirmed"));
            }
            Ok(Self {
                previous,
                core,
                priority_class,
            })
        }
    }

    impl Drop for AffinityGuard {
        fn drop(&mut self) {
            unsafe {
                SetThreadAffinityMask(GetCurrentThread(), self.previous);
            }
        }
    }
}
