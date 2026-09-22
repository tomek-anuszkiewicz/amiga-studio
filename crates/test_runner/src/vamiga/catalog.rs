//! vAmigaTS Test Catalog, Discovery & Categorization Engine
//!
//! Indexes all test directories in `ref_src/vAmigaTS`, classifies each test into
//! roadmap sub-suites, parses bootblock direct-injection compatibility, and tags
//! non-OCS suites with explicit deferral reasons.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Primary verification sub-suite categories matching ROADMAP.md Step 2
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VamigaCategory {
    Copper,
    Blitter,
    Agnus,
    Denise,
    Paula,
    Cpu,
    Cia,
    Mainboard,
    Memory,
    Misc,
    All,
}

impl VamigaCategory {
    /// Formats the category as a lowercase command-line / report string
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Copper => "copper",
            Self::Blitter => "blitter",
            Self::Agnus => "agnus",
            Self::Denise => "denise",
            Self::Paula => "paula",
            Self::Cpu => "cpu",
            Self::Cia => "cia",
            Self::Mainboard => "mainboard",
            Self::Memory => "memory",
            Self::Misc => "misc",
            Self::All => "all",
        }
    }

    /// Parses a category from a string identifier
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "copper" | "cop" => Some(Self::Copper),
            "blitter" | "blt" => Some(Self::Blitter),
            "agnus" => Some(Self::Agnus),
            "denise" | "video" => Some(Self::Denise),
            "paula" | "audio" => Some(Self::Paula),
            "cpu" | "m68k" | "68000" => Some(Self::Cpu),
            "cia" | "timers" => Some(Self::Cia),
            "mainboard" | "ports" => Some(Self::Mainboard),
            "memory" | "ram" => Some(Self::Memory),
            "misc" => Some(Self::Misc),
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

/// Explicit architectural rationale for deferring a test from Phase 1 OCS Baseline
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeferredReason {
    /// Test requires Motorola MC68881/68882 floating-point math coprocessor
    Fpu,
    /// Test only provides ECS Denise 8373 / AGA 24-bit palette reference captures
    EcsOrAgaOnly,
    /// Test targets Motorola 68010 CPU architecture (VBR, loop mode, BKPT)
    Cpu68010Only,
    /// Test relies on full AmigaOS floppy bootstrap or non-standard bootblock
    NonStandardBootblock,
    /// Test lacks 716 x 285 RGB24 reference capture (e.g. CRT monitor photograph)
    NoRawReference,
}

impl DeferredReason {
    /// Human-readable label for report tables
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Fpu => "FPU Coprocessor Required (MC68881/68882)",
            Self::EcsOrAgaOnly => "ECS/AGA Silicon Only (_ecs.raw / _plus.raw / _aga.raw)",
            Self::Cpu68010Only => "Motorola 68010 Required (_68010.raw)",
            Self::NonStandardBootblock => "AmigaOS Floppy MFM Bootblock Required (Non-Sector 2)",
            Self::NoRawReference => "Non-Visual / Photo Only (No 24-bit .raw Capture)",
        }
    }

    /// Roadmap milestone where this deferred suite will be verified
    pub const fn roadmap_target(&self) -> &'static str {
        match self {
            Self::Fpu => "Phase 3: Advanced Graphics Architecture & FPU",
            Self::EcsOrAgaOnly => "Phase 2 (ECS) & Phase 3 (AGA)",
            Self::Cpu68010Only => "68010 CPU Architecture Milestone",
            Self::NonStandardBootblock => "Step 6: Real-World Amiga Workloads (MFM Boot)",
            Self::NoRawReference => "Step 2.7: Non-Visual Register Assertions",
        }
    }
}

/// Execution status of an individual vAmigaTS test case
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VamigaTestStatus {
    /// Active test runnable under Phase 1 Baseline A500 OCS via Sector 2 direct injection
    Runnable,
    /// Deferred test with explicit architectural reason
    Deferred(DeferredReason),
}

/// Metadata descriptor for an individual vAmigaTS test case
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VamigaTestDescriptor {
    /// Test case name (e.g. "coptim1", "bbusy0")
    pub name: String,
    /// Primary verification category
    pub category: VamigaCategory,
    /// Relative directory path from `ref_src/vAmigaTS`
    pub rel_dir: PathBuf,
    /// Absolute path to the test ADF disk image
    pub adf_path: PathBuf,
    /// Absolute path to the reference `.raw` frame capture (if any)
    pub raw_path: Option<PathBuf>,
    /// Absolute path to the `.retrosh` execution script (if any)
    pub retrosh_path: Option<PathBuf>,
    /// Active Phase 1 status or deferral classification
    pub status: VamigaTestStatus,
}

/// Summary statistics of a discovered catalog
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CatalogStats {
    pub total_tests: usize,
    pub runnable_tests: usize,
    pub deferred_tests: usize,
    pub deferred_by_reason: BTreeMap<&'static str, usize>,
    pub tests_by_category: BTreeMap<VamigaCategory, usize>,
    pub runnable_by_category: BTreeMap<VamigaCategory, usize>,
}

/// Catalog holding all indexed vAmigaTS tests
#[derive(Debug, Clone, Default)]
pub struct VamigaCatalog {
    pub tests: Vec<VamigaTestDescriptor>,
}

impl VamigaCatalog {
    /// Discovers and classifies all tests in `ref_src/vAmigaTS`
    pub fn discover(vamiga_root: &Path) -> Self {
        let mut tests = Vec::with_capacity(2100);
        if !vamiga_root.is_dir() {
            return Self { tests };
        }

        Self::scan_dir_recursive(vamiga_root, vamiga_root, &mut tests);
        tests.sort_by(|a, b| a.rel_dir.cmp(&b.rel_dir));
        Self { tests }
    }

    /// Recursively scans for directories containing `.adf` test images
    fn scan_dir_recursive(root: &Path, current: &Path, out: &mut Vec<VamigaTestDescriptor>) {
        let entries = match fs::read_dir(current) {
            Ok(e) => e,
            Err(_) => return,
        };

        let mut subdirs = Vec::new();
        let mut adf_file = None;
        let mut raw_files = Vec::new();
        let mut retrosh_files = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                subdirs.push(path);
            } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                let ext_lower = ext.to_ascii_lowercase();
                if ext_lower == "adf" && adf_file.is_none() {
                    adf_file = Some(path);
                } else if ext_lower == "raw" {
                    raw_files.push(path);
                } else if ext_lower == "retrosh" {
                    retrosh_files.push(path);
                }
            }
        }

        if let Some(adf_path) = adf_file {
            let rel_dir = current.strip_prefix(root).unwrap_or(current).to_path_buf();
            let test_name = adf_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("test")
                .to_string();

            let category = Self::determine_category(&rel_dir);
            let (raw_path, retrosh_path, status) = Self::classify_test(
                &adf_path,
                &test_name,
                &rel_dir,
                category,
                &raw_files,
                &retrosh_files,
            );

            out.push(VamigaTestDescriptor {
                name: test_name,
                category,
                rel_dir,
                adf_path,
                raw_path,
                retrosh_path,
                status,
            });
        }

        for subdir in subdirs {
            Self::scan_dir_recursive(root, &subdir, out);
        }
    }

    /// Maps a relative directory path to its primary `VamigaCategory`
    fn determine_category(rel_dir: &Path) -> VamigaCategory {
        let comps: Vec<String> = rel_dir
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase())
            .collect();

        if comps.is_empty() {
            return VamigaCategory::Misc;
        }

        match comps[0].as_str() {
            "agnus" => {
                if comps.get(1).map(|s| s.as_str()) == Some("copper") {
                    VamigaCategory::Copper
                } else if comps.get(1).map(|s| s.as_str()) == Some("blitter") {
                    VamigaCategory::Blitter
                } else {
                    VamigaCategory::Agnus
                }
            }
            "denise" => VamigaCategory::Denise,
            "paula" => VamigaCategory::Paula,
            "cpu" => VamigaCategory::Cpu,
            "cia" => VamigaCategory::Cia,
            "mainboard" => VamigaCategory::Mainboard,
            "memory" => VamigaCategory::Memory,
            "fpu" => VamigaCategory::Cpu,
            _ => VamigaCategory::Misc,
        }
    }

    /// Checks whether an ADF has the standard Sector 2 direct-injection micro-bootblock
    fn is_standard_injection_bootblock(adf_path: &Path) -> bool {
        let mut file = match File::open(adf_path) {
            Ok(f) => f,
            Err(_) => return false,
        };

        let mut header = [0u8; 1024];
        if file.read_exact(&mut header).is_err() {
            return false;
        }

        // Must begin with Amiga DOS magic: "DOS\0" (0x44, 0x4F, 0x53, 0x00)
        if header[0..4] != [0x44, 0x4F, 0x53, 0x00] {
            return false;
        }

        // Standard vAmigaTS Sector 2 loader instructions at offset 12..40:
        // lea $70000, a5 (4b f9 00 07 00 00)
        // move.l #len, 36(a1) (23 7c [len] 00 24)
        // move.l a5, 40(a1) (23 4d 00 28)
        // move.l #$400, 44(a1) (23 7c 00 00 04 00 00 2c)
        // jsr -456(a6) (4e ae fe 38)
        if header[12..18] != [0x4B, 0xF9, 0x00, 0x07, 0x00, 0x00] {
            return false;
        }

        if header[18..20] != [0x23, 0x7C] || header[24..26] != [0x00, 0x24] {
            return false;
        }

        if header[26..30] != [0x23, 0x4D, 0x00, 0x28] {
            return false;
        }

        if header[30..38] != [0x23, 0x7C, 0x00, 0x00, 0x04, 0x00, 0x00, 0x2C] {
            return false;
        }

        true
    }

    /// Classifies an individual test into Runnable or Deferred
    fn classify_test(
        adf_path: &Path,
        _test_name: &str,
        rel_dir: &Path,
        _category: VamigaCategory,
        raw_files: &[PathBuf],
        retrosh_files: &[PathBuf],
    ) -> (Option<PathBuf>, Option<PathBuf>, VamigaTestStatus) {
        let comps: Vec<String> = rel_dir
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase())
            .collect();

        // 1. FPU tests require Motorola MC68881/68882 math coprocessor
        if !comps.is_empty() && comps[0] == "fpu" {
            let raw = raw_files.first().cloned();
            let retrosh = retrosh_files.first().cloned();
            return (
                raw,
                retrosh,
                VamigaTestStatus::Deferred(DeferredReason::Fpu),
            );
        }

        // 2. Tests without 24-bit RGB `.raw` frame captures
        if raw_files.is_empty() {
            let retrosh = retrosh_files.first().cloned();
            return (
                None,
                retrosh,
                VamigaTestStatus::Deferred(DeferredReason::NoRawReference),
            );
        }

        // 3. Tests requiring full AmigaOS DOS bootstrap (non-standard bootblock)
        if !Self::is_standard_injection_bootblock(adf_path) {
            let raw = raw_files.first().cloned();
            let retrosh = retrosh_files.first().cloned();
            return (
                raw,
                retrosh,
                VamigaTestStatus::Deferred(DeferredReason::NonStandardBootblock),
            );
        }

        // 4. Resolve preferred reference raw capture: prefer _ocs.raw, then _68000.raw, then plain .raw
        let mut ocs_raw = None;
        let mut plain_raw = None;
        let mut cpu68010_raw = None;
        let mut ecs_or_aga_raw = None;

        for r in raw_files {
            let stem = r.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let parts: Vec<&str> = stem.split('_').collect();
            let suffix = if parts.len() > 1 {
                parts.last().copied().unwrap_or("").to_ascii_lowercase()
            } else {
                String::new()
            };

            match suffix.as_str() {
                "ocs" => ocs_raw = Some(r.clone()),
                "68000" => {
                    if ocs_raw.is_none() {
                        ocs_raw = Some(r.clone());
                    }
                }
                "" => plain_raw = Some(r.clone()),
                "68010" => cpu68010_raw = Some(r.clone()),
                "ecs" | "plus" | "aga" => {
                    if ecs_or_aga_raw.is_none() {
                        ecs_or_aga_raw = Some(r.clone());
                    }
                }
                _ => {
                    if plain_raw.is_none() {
                        plain_raw = Some(r.clone());
                    }
                }
            }
        }

        // Resolve retrosh script matching selected raw capture
        let resolve_retrosh = |chosen_raw: &Path| -> Option<PathBuf> {
            let raw_stem = chosen_raw
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let candidate = chosen_raw.with_file_name(format!("{}.retrosh", raw_stem));
            if candidate.is_file() {
                Some(candidate)
            } else {
                retrosh_files.first().cloned()
            }
        };

        if let Some(raw) = ocs_raw {
            let retrosh = resolve_retrosh(&raw);
            (Some(raw), retrosh, VamigaTestStatus::Runnable)
        } else if let Some(raw) = plain_raw {
            let retrosh = resolve_retrosh(&raw);
            (Some(raw), retrosh, VamigaTestStatus::Runnable)
        } else if let Some(raw) = cpu68010_raw {
            let retrosh = resolve_retrosh(&raw);
            (
                Some(raw),
                retrosh,
                VamigaTestStatus::Deferred(DeferredReason::Cpu68010Only),
            )
        } else if let Some(raw) = ecs_or_aga_raw {
            let retrosh = resolve_retrosh(&raw);
            (
                Some(raw),
                retrosh,
                VamigaTestStatus::Deferred(DeferredReason::EcsOrAgaOnly),
            )
        } else {
            let chosen = raw_files.first().cloned();
            let retrosh = chosen.as_ref().and_then(|r| resolve_retrosh(r));
            (
                chosen,
                retrosh,
                VamigaTestStatus::Deferred(DeferredReason::EcsOrAgaOnly),
            )
        }
    }

    /// Filters tests by category (matching `VamigaCategory`)
    pub fn filter_category(&self, category: VamigaCategory) -> Vec<&VamigaTestDescriptor> {
        if category == VamigaCategory::All {
            self.tests.iter().collect()
        } else {
            self.tests
                .iter()
                .filter(|t| t.category == category)
                .collect()
        }
    }

    /// Returns only active Phase 1 Baseline runnable tests
    pub fn runnable_tests(&self) -> Vec<&VamigaTestDescriptor> {
        self.tests
            .iter()
            .filter(|t| t.status == VamigaTestStatus::Runnable)
            .collect()
    }

    /// Returns only deferred tests
    pub fn deferred_tests(&self) -> Vec<&VamigaTestDescriptor> {
        self.tests
            .iter()
            .filter(|t| matches!(t.status, VamigaTestStatus::Deferred(_)))
            .collect()
    }

    /// Finds a test by exact or loose name or relative path query
    pub fn find_test(&self, query: &str) -> Option<&VamigaTestDescriptor> {
        let q_clean = query.replace('\\', "/").to_ascii_lowercase();

        // 1. Exact relative directory match
        if let Some(t) = self.tests.iter().find(|t| {
            t.rel_dir
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase()
                == q_clean
        }) {
            return Some(t);
        }

        // 2. Exact test name match
        if let Some(t) = self
            .tests
            .iter()
            .find(|t| t.name.to_ascii_lowercase() == q_clean)
        {
            return Some(t);
        }

        // 3. Substring / suffix match
        self.tests.iter().find(|t| {
            let rel_str = t
                .rel_dir
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase();
            rel_str.ends_with(&q_clean) || t.name.to_ascii_lowercase().contains(&q_clean)
        })
    }

    /// Computes summary statistics across all discovered tests
    pub fn stats(&self) -> CatalogStats {
        let mut stats = CatalogStats {
            total_tests: self.tests.len(),
            ..Default::default()
        };

        for t in &self.tests {
            *stats.tests_by_category.entry(t.category).or_insert(0) += 1;
            match &t.status {
                VamigaTestStatus::Runnable => {
                    stats.runnable_tests += 1;
                    *stats.runnable_by_category.entry(t.category).or_insert(0) += 1;
                }
                VamigaTestStatus::Deferred(reason) => {
                    stats.deferred_tests += 1;
                    *stats.deferred_by_reason.entry(reason.as_str()).or_insert(0) += 1;
                }
            }
        }

        stats
    }
}
