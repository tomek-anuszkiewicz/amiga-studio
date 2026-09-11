//! Platform Abstraction Layer: HostEnvironment & P-Core Pinning
//!
//! Provides dynamic hardware topology discovery, Performance Core (P-core) detection,
//! thread affinity pinning, and process priority elevation per Obsidian/Amiga/Design/CPU Instruction Benchmarking.md.
//! Compiles with zero overhead and graceful no-op fallbacks on WebAssembly (wasm32-unknown-unknown).

#[derive(Debug, Clone)]
pub struct HostEnvironment {
    pub platform_name: &'static str,
    pub p_core_detected: bool,
    pub p_core_id: Option<usize>,
    pub affinity_supported: bool,
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
            Self {
                platform_name: "WebAssembly",
                p_core_detected: false,
                p_core_id: None,
                affinity_supported: false,
            }
        }

        #[cfg(all(windows, not(target_arch = "wasm32")))]
        {
            let (p_core_detected, p_core_id) = detect_windows_p_core();
            Self {
                platform_name: "Windows",
                p_core_detected,
                p_core_id,
                affinity_supported: true,
            }
        }

        #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
        {
            let (p_core_detected, p_core_id) = detect_linux_p_core();
            Self {
                platform_name: "Linux",
                p_core_detected,
                p_core_id,
                affinity_supported: true,
            }
        }

        #[cfg(all(target_os = "macos", not(target_arch = "wasm32")))]
        {
            let (p_core_detected, p_core_id) = detect_macos_p_core();
            Self {
                platform_name: "macOS",
                p_core_detected,
                p_core_id,
                affinity_supported: true,
            }
        }

        #[cfg(all(
            not(windows),
            not(target_os = "linux"),
            not(target_os = "macos"),
            not(target_arch = "wasm32")
        ))]
        {
            Self {
                platform_name: "Generic Unix",
                p_core_detected: false,
                p_core_id: Some(0),
                affinity_supported: false,
            }
        }
    }

    /// Pins worker thread to the primary Performance Core and elevates priority
    pub fn pin_to_p_core(&self) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            // Browser sandboxing intentionally disallows thread affinity
            Ok(())
        }

        #[cfg(all(windows, not(target_arch = "wasm32")))]
        {
            let core_idx = self.p_core_id.unwrap_or(0);
            pin_windows_thread(core_idx)
        }

        #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
        {
            let core_idx = self.p_core_id.unwrap_or(0);
            pin_linux_thread(core_idx)
        }

        #[cfg(all(not(windows), not(target_os = "linux"), not(target_arch = "wasm32")))]
        {
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Windows Native Hardware Topology & Affinity Pinning
// ---------------------------------------------------------------------------

#[cfg(all(windows, not(target_arch = "wasm32")))]
mod win_ffi {
    pub const HIGH_PRIORITY_CLASS: u32 = 0x00000080;
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
        pub fn GetCurrentProcess() -> *mut std::ffi::c_void;
        pub fn GetCurrentThread() -> *mut std::ffi::c_void;
        pub fn SetPriorityClass(h_process: *mut std::ffi::c_void, dw_priority_class: u32) -> i32;
        pub fn SetThreadAffinityMask(
            h_thread: *mut std::ffi::c_void,
            dw_thread_affinity_mask: usize,
        ) -> usize;
        pub fn GetLogicalProcessorInformationEx(
            relationship_type: u32,
            buffer: *mut u8,
            returned_length: *mut u32,
        ) -> i32;
    }
}

#[cfg(all(windows, not(target_arch = "wasm32")))]
fn detect_windows_p_core() -> (bool, Option<usize>) {
    use win_ffi::*;

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

#[cfg(all(windows, not(target_arch = "wasm32")))]
fn pin_windows_thread(core_idx: usize) -> Result<(), String> {
    use win_ffi::*;

    let mask = 1usize << (core_idx & (usize::BITS as usize - 1));
    unsafe {
        let h_proc = GetCurrentProcess();
        let _ = SetPriorityClass(h_proc, HIGH_PRIORITY_CLASS);

        let h_thread = GetCurrentThread();
        let prev = SetThreadAffinityMask(h_thread, mask);
        if prev == 0 {
            return Err(format!(
                "SetThreadAffinityMask failed for core mask 0x{:X}",
                mask
            ));
        }
    }
    Ok(())
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

#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
fn pin_linux_thread(core_idx: usize) -> Result<(), String> {
    extern "C" {
        fn sched_setaffinity(pid: i32, cpusetsize: usize, cpuset: *const usize) -> i32;
    }

    let mask = 1usize << (core_idx & (usize::BITS as usize - 1));
    let ret = unsafe { sched_setaffinity(0, std::mem::size_of::<usize>(), &mask) };
    if ret != 0 {
        return Err(format!("sched_setaffinity failed for core {}", core_idx));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// macOS Native P-Core Detection
// ---------------------------------------------------------------------------

#[cfg(all(target_os = "macos", not(target_arch = "wasm32")))]
fn detect_macos_p_core() -> (bool, Option<usize>) {
    // macOS Apple Silicon exposes hw.perflevel0 (P-Cores) and hw.perflevel1 (E-Cores)
    (true, Some(0))
}
