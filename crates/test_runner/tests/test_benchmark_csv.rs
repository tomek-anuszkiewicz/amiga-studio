//! Golden Master Benchmark CSV Structure & Cycle Invariance Regression Test Suite
//!
//! Validates that the deterministic columns of benchmark CSV tables:
//!   `mnemonic,variant,mode,category,opcode,amiga_cck,total_ops`
//! match verified Golden Master 64-bit FNV-1a hashes across all 108 benchmark specifications
//! in the 3 profile directories: `quick`, `standard`, and `thorough` (3 x 108 = 324 opcode row entries).
//!
//! Measurements (host_ms_median, host_ns_op, host_ns_cck, host_mips, jitter_pct) vary by host CPU,
//! but the instruction catalog, addressing modes, opcode encodings, and Amiga CCK timings
//! must remain strictly invariant and regression-free.
//!
//! =========================================================================================
//! ⚠️ ANTI-TAMPER POLICY & INVARIANCE CONTRACT:
//! DO NOT MODIFY THE GOLDEN MASTER CONSTANTS TO "FIX" A FAILING TEST!
//! The golden constants below represent verified hardware truth:
//!   - GOLDEN_CATALOG_STRUCTURE_HASH: 0x3972F381CE69D98F
//!   - GOLDEN_CSV_HASH_QUICK:         0xB699CBA7E6ADD635
//!   - GOLDEN_CSV_HASH_STANDARD:      0x9FE3956BC57151D1
//!   - GOLDEN_CSV_HASH_THOROUGH:      0xDB747B4AC9321D39
//! If this test fails, it indicates an unintentional corruption of the instruction catalog,
//! addressing modes, opcode encodings, or Amiga CCK timings. Blindly updating these constants
//! to silence a test failure is strictly prohibited by AGENTS.md and spec-compliance.md.
//! =========================================================================================

mod golden_row_hashes;
pub use golden_row_hashes::GOLDEN_ROW_HASHES_324;

use std::fs;
use std::path::{Path, PathBuf};
use test_runner::benchmark::{
    all_benchmark_specs, run_benchmark_suite, BenchmarkConfig, BenchmarkProfile,
};

/// Fast, deterministic, platform-independent 64-bit FNV-1a hash.
/// Normalizes CRLF (\r\n) to LF (\n) to ensure cross-platform reproducibility.
fn fnv1a_64_normalized(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &b in text.as_bytes() {
        if b != b'\r' {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

/// Locates repository root for accessing `tests/benchmarks/` on disk
fn find_repo_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    while !dir.join("ROADMAP.md").exists() {
        if !dir.pop() {
            return None;
        }
    }
    Some(dir)
}

/// Verified golden hash for the pure catalog structure (columns 1..=6: mnemonic,variant,mode,category,opcode,amiga_cck).
/// Invariant across all execution profiles (Quick, Standard, Thorough).
pub const GOLDEN_CATALOG_STRUCTURE_HASH: u64 = 0x3972F381CE69D98F;

/// Verified golden hashes including nominal `total_ops` (columns 1..=7).
pub const GOLDEN_CSV_HASH_QUICK: u64 = 0xB699CBA7E6ADD635;
pub const GOLDEN_CSV_HASH_STANDARD: u64 = 0x9FE3956BC57151D1;
pub const GOLDEN_CSV_HASH_THOROUGH: u64 = 0xDB747B4AC9321D39;

/// Generates deterministic static CSV rows (columns 1..=7) in memory directly from `all_benchmark_specs()`.
fn synthesize_static_csv_lines(total_ops: u64) -> Vec<String> {
    let mut lines = Vec::with_capacity(108);
    for spec in all_benchmark_specs() {
        let opcode_hex = spec
            .opcode_words
            .iter()
            .map(|w| format!("{:04X}", w))
            .collect::<Vec<_>>()
            .join(" ");

        let line = format!(
            "{},{},{},{},{},{},{}",
            spec.mnemonic,
            spec.representative_syntax.replace(',', " "),
            spec.addressing_mode.name(),
            spec.category.name(),
            opcode_hex,
            spec.amiga_cck,
            total_ops
        );
        lines.push(line);
    }
    lines
}

/// Generates catalog structure lines (columns 1..=6, without total_ops) in memory.
fn synthesize_catalog_structure_lines() -> Vec<String> {
    let mut lines = Vec::with_capacity(108);
    for spec in all_benchmark_specs() {
        let opcode_hex = spec
            .opcode_words
            .iter()
            .map(|w| format!("{:04X}", w))
            .collect::<Vec<_>>()
            .join(" ");

        let line = format!(
            "{},{},{},{},{},{}",
            spec.mnemonic,
            spec.representative_syntax.replace(',', " "),
            spec.addressing_mode.name(),
            spec.category.name(),
            opcode_hex,
            spec.amiga_cck
        );
        lines.push(line);
    }
    lines
}

/// Ensures that the quick profile benchmark CSV exists on disk with full 108 specifications.
/// If missing or incomplete (< 109 lines), runs the quick suite in ~0.05s.
fn ensure_quick_benchmark_csv_exists(repo_root: &Path) {
    let quick_csv = repo_root.join("tests/benchmarks/quick/m68k_benchmark_latest.csv");
    let needs_gen = if !quick_csv.exists() {
        true
    } else {
        match fs::read_to_string(&quick_csv) {
            Ok(content) => content.lines().filter(|l| !l.trim().is_empty()).count() < 109,
            Err(_) => true,
        }
    };

    if needs_gen {
        println!("[*] Quick benchmark CSV missing or incomplete; auto-generating on the fly...");
        let config = BenchmarkConfig {
            profile: BenchmarkProfile::Quick,
            filter: None,
            unroll_k: 700,
            passes: BenchmarkProfile::Quick.default_passes(),
            iterations: BenchmarkProfile::Quick.default_iterations(),
            out_dir: repo_root.join("tests/benchmarks"),
            pin_core: false,
            verbose: false,
        };
        let _ =
            run_benchmark_suite(&config).expect("Failed to auto-generate quick benchmark suite");
    }
}

/// Validates a single benchmark CSV file on disk:
/// - Header schema (14 columns)
/// - Col 0 (`timestamp`): starts with `epoch_`
/// - Col 1..=5 (`mnemonic,variant,mode,category,opcode`): catalog correctness
/// - Col 6 (`amiga_cck`): hardware cycle exactness
/// - Col 7 (`total_ops`): nominal operation scaling for profile
/// - Col 8..=13 (`host_ms_median`, `host_ns_op`, etc.): numeric validity
/// - Catalog sequence hash (columns 1..=6)
/// - Profile CSV sequence hash (columns 1..=7)
fn validate_benchmark_csv_file(
    path: &Path,
    expected_total_ops: u64,
    expected_csv_hash: u64,
    profile_name: &str,
) -> Result<usize, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read CSV: {}", e))?;
    let raw_lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

    if raw_lines.len() != 109 {
        return Err(format!(
            "Expected exactly 109 lines (1 header + 108 opcodes), got {} in {}",
            raw_lines.len(),
            path.display()
        ));
    }

    // 1. Verify Header
    let expected_header = "timestamp,mnemonic,variant,addressing_mode,category,opcode_hex,amiga_cck_cycles,total_guest_instructions,host_duration_median_ms,host_ns_per_instruction,host_ns_per_guest_cck,host_mips,host_jitter_pct,anomaly_flag";
    assert_eq!(
        raw_lines[0].trim(),
        expected_header,
        "Header mismatch in {}",
        path.display()
    );

    let specs = all_benchmark_specs();
    let mut catalog_lines = Vec::with_capacity(108);
    let mut static_lines = Vec::with_capacity(108);

    for (row_idx, (line, spec)) in raw_lines[1..].iter().zip(specs).enumerate() {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() != 14 {
            return Err(format!(
                "Row {} in {}: expected 14 columns, got {}",
                row_idx + 1,
                path.display(),
                cols.len()
            ));
        }

        // Col 0: timestamp - variable, verify starts with "epoch_"
        if !cols[0].starts_with("epoch_") {
            return Err(format!(
                "Row {} in {}: timestamp '{}' does not start with 'epoch_'",
                row_idx + 1,
                path.display(),
                cols[0]
            ));
        }

        // Col 1..=5: mnemonic, variant, mode, category, opcode
        let expected_opcode_hex = spec
            .opcode_words
            .iter()
            .map(|w| format!("{:04X}", w))
            .collect::<Vec<_>>()
            .join(" ");
        let expected_variant = spec.representative_syntax.replace(',', " ");

        assert_eq!(
            cols[1], spec.mnemonic,
            "\n🔴 BENCHMARK CATALOG MISMATCH in row {} of {}:\nExpected mnemonic: {}\nFound: {}\n⚠️ ANTI-TAMPER RULE: Do NOT modify golden constants or assertions!",
            row_idx + 1, path.display(), spec.mnemonic, cols[1]
        );
        assert_eq!(
            cols[2], expected_variant,
            "\n🔴 BENCHMARK CATALOG MISMATCH in row {} of {}:\nExpected variant: {}\nFound: {}\n⚠️ ANTI-TAMPER RULE: Do NOT modify golden constants or assertions!",
            row_idx + 1, path.display(), expected_variant, cols[2]
        );
        assert_eq!(
            cols[3], spec.addressing_mode.name(),
            "\n🔴 BENCHMARK CATALOG MISMATCH in row {} of {}:\nExpected mode: {}\nFound: {}\n⚠️ ANTI-TAMPER RULE: Do NOT modify golden constants or assertions!",
            row_idx + 1, path.display(), spec.addressing_mode.name(), cols[3]
        );
        assert_eq!(
            cols[4], spec.category.name(),
            "\n🔴 BENCHMARK CATALOG MISMATCH in row {} of {}:\nExpected category: {}\nFound: {}\n⚠️ ANTI-TAMPER RULE: Do NOT modify golden constants or assertions!",
            row_idx + 1, path.display(), spec.category.name(), cols[4]
        );
        assert_eq!(
            cols[5], expected_opcode_hex,
            "\n🔴 BENCHMARK CATALOG MISMATCH in row {} of {}:\nExpected opcode hex: {}\nFound: {}\n⚠️ ANTI-TAMPER RULE: Do NOT modify golden constants or assertions!",
            row_idx + 1, path.display(), expected_opcode_hex, cols[5]
        );

        // Col 6: amiga_cck
        let amiga_cck: u32 = cols[6].parse().map_err(|e| {
            format!(
                "Row {} amiga_cck '{}' is not a valid integer: {}",
                row_idx + 1,
                cols[6],
                e
            )
        })?;
        assert_eq!(
            amiga_cck, spec.amiga_cck,
            "\n🔴 BENCHMARK CCK MISMATCH in row {} of {}:\nExpected CCK: {}\nFound: {}\n⚠️ ANTI-TAMPER RULE: Do NOT modify golden constants or assertions!",
            row_idx + 1, path.display(), spec.amiga_cck, amiga_cck
        );

        // Col 7: total_ops
        let total_ops: u64 = cols[7].parse().map_err(|e| {
            format!(
                "Row {} total_ops '{}' is not a valid integer: {}",
                row_idx + 1,
                cols[7],
                e
            )
        })?;
        assert_eq!(
            total_ops, expected_total_ops,
            "\n🔴 BENCHMARK TOTAL_OPS MISMATCH in row {} of {}:\nExpected total_ops: {}\nFound: {}\n",
            row_idx + 1, path.display(), expected_total_ops, total_ops
        );

        // Col 8: host_ms_median (float > 0)
        let ms_median: f64 = cols[8].parse().map_err(|e| {
            format!(
                "Row {} host_ms_median '{}' is not a valid float: {}",
                row_idx + 1,
                cols[8],
                e
            )
        })?;
        assert!(
            ms_median > 0.0,
            "Row {} host_ms_median must be positive",
            row_idx + 1
        );

        // Col 9: host_ns_op (float > 0)
        let ns_op: f64 = cols[9].parse().map_err(|e| {
            format!(
                "Row {} host_ns_op '{}' is not a valid float: {}",
                row_idx + 1,
                cols[9],
                e
            )
        })?;
        assert!(
            ns_op > 0.0,
            "Row {} host_ns_op must be positive",
            row_idx + 1
        );

        // Col 10: host_ns_cck (float > 0)
        let ns_cck: f64 = cols[10].parse().map_err(|e| {
            format!(
                "Row {} host_ns_cck '{}' is not a valid float: {}",
                row_idx + 1,
                cols[10],
                e
            )
        })?;
        assert!(
            ns_cck > 0.0,
            "Row {} host_ns_cck must be positive",
            row_idx + 1
        );

        // Col 11: host_mips (float > 0)
        let mips: f64 = cols[11].parse().map_err(|e| {
            format!(
                "Row {} host_mips '{}' is not a valid float: {}",
                row_idx + 1,
                cols[11],
                e
            )
        })?;
        assert!(mips > 0.0, "Row {} host_mips must be positive", row_idx + 1);

        // Col 12: jitter_pct (float >= 0)
        let jitter: f64 = cols[12].parse().map_err(|e| {
            format!(
                "Row {} jitter_pct '{}' is not a valid float: {}",
                row_idx + 1,
                cols[12],
                e
            )
        })?;
        assert!(
            jitter >= 0.0,
            "Row {} jitter_pct must be non-negative",
            row_idx + 1
        );

        // Col 13: anomaly (bool)
        assert!(
            cols[13] == "true" || cols[13] == "false",
            "Row {} anomaly '{}' must be 'true' or 'false'",
            row_idx + 1,
            cols[13]
        );

        let row_static_str = format!(
            "{},{},{},{},{},{},{}",
            cols[1], cols[2], cols[3], cols[4], cols[5], cols[6], cols[7]
        );

        let profile_offset = match profile_name {
            "quick" => 0,
            "standard" => 108,
            "thorough" => 216,
            _ => usize::MAX,
        };
        if profile_offset != usize::MAX {
            let row_hash = fnv1a_64_normalized(&row_static_str);
            let expected_row_hash = GOLDEN_ROW_HASHES_324[profile_offset + row_idx];
            assert_eq!(
                row_hash, expected_row_hash,
                "\n🔴 ROW HASH MISMATCH at row {} of {} [Profile: {}]:\nRow content: {}\nExpected Hash: 0x{:016X}\nActual Hash:   0x{:016X}\n⚠️ ANTI-TAMPER RULE: Do NOT modify golden constants or assertions!",
                row_idx + 1, path.display(), profile_name, row_static_str, expected_row_hash, row_hash
            );
        }

        catalog_lines.push(format!(
            "{},{},{},{},{},{}",
            cols[1], cols[2], cols[3], cols[4], cols[5], cols[6]
        ));
        static_lines.push(row_static_str);
    }

    // Sequence hash validation
    let cat_text = catalog_lines.join("\n") + "\n";
    let cat_hash = fnv1a_64_normalized(&cat_text);
    assert_eq!(
        cat_hash, GOLDEN_CATALOG_STRUCTURE_HASH,
        "\n🔴 CATALOG STRUCTURE HASH MISMATCH in {}\nExpected: 0x{:016X}\nActual:   0x{:016X}\n⚠️ ANTI-TAMPER RULE: Do NOT modify golden constants or assertions!",
        path.display(), GOLDEN_CATALOG_STRUCTURE_HASH, cat_hash
    );

    let static_text = static_lines.join("\n") + "\n";
    let static_hash = fnv1a_64_normalized(&static_text);
    assert_eq!(
        static_hash, expected_csv_hash,
        "\n🔴 PROFILE CSV HASH MISMATCH in {} [Profile: {}]\nExpected: 0x{:016X}\nActual:   0x{:016X}\n⚠️ ANTI-TAMPER RULE: Do NOT modify golden constants or assertions!",
        path.display(), profile_name, expected_csv_hash, static_hash
    );

    Ok(108)
}

#[test]
fn test_benchmark_catalog_structure_hash_in_memory() {
    let lines = synthesize_catalog_structure_lines();
    assert_eq!(
        lines.len(),
        108,
        "Benchmark catalog must contain exactly 108 specifications"
    );

    let text = lines.join("\n") + "\n";
    let actual_hash = fnv1a_64_normalized(&text);

    if actual_hash != GOLDEN_CATALOG_STRUCTURE_HASH {
        panic!(
            "\n🔴 BENCHMARK CATALOG STRUCTURE HASH MISMATCH!\n\
               Expected Golden Hash: 0x{:016X}\n\
               Actual Computed Hash: 0x{:016X}\n\
               An instruction mnemonic, variant syntax, addressing mode, opcode encoding, or Amiga CCK cycle count has changed!\n\
               Inspect the benchmark catalog definitions in crates/test_runner/src/benchmark/catalog_data_*.rs.\n\
               ⚠️ ANTI-TAMPER RULE: Do NOT modify GOLDEN_CATALOG_STRUCTURE_HASH to silence this error!\n",
            GOLDEN_CATALOG_STRUCTURE_HASH, actual_hash
        );
    }
}

#[test]
fn test_benchmark_csv_profiles_against_golden_hashes() {
    let unroll_k = 700u64;

    let profiles = [
        (
            BenchmarkProfile::Quick,
            unroll_k
                * (BenchmarkProfile::Quick.default_iterations() as u64)
                * (BenchmarkProfile::Quick.default_passes() as u64),
            GOLDEN_CSV_HASH_QUICK,
        ),
        (
            BenchmarkProfile::Standard,
            unroll_k
                * (BenchmarkProfile::Standard.default_iterations() as u64)
                * (BenchmarkProfile::Standard.default_passes() as u64),
            GOLDEN_CSV_HASH_STANDARD,
        ),
        (
            BenchmarkProfile::Thorough,
            unroll_k
                * (BenchmarkProfile::Thorough.default_iterations() as u64)
                * (BenchmarkProfile::Thorough.default_passes() as u64),
            GOLDEN_CSV_HASH_THOROUGH,
        ),
    ];

    for (profile, total_ops, expected_hash) in profiles {
        let lines = synthesize_static_csv_lines(total_ops);
        let text = lines.join("\n") + "\n";
        let actual_hash = fnv1a_64_normalized(&text);

        assert_eq!(
            actual_hash, expected_hash,
            "\n🔴 BENCHMARK CSV HASH MISMATCH for profile [{:?}] (total_ops = {})\n\
               Expected Golden Hash: 0x{:016X}\n\
               Actual Computed Hash: 0x{:016X}\n\
               The deterministic CSV columns for this profile have changed!\n\
               ⚠️ ANTI-TAMPER RULE: Do NOT modify golden profile constants to silence this error!\n",
            profile, total_ops, expected_hash, actual_hash
        );
    }
}

#[test]
fn test_all_three_profiles_disk_csv_row_by_row_validation() {
    let repo_root = match find_repo_root() {
        Some(r) => r,
        None => return,
    };

    // Auto-generate quick profile if missing or incomplete (< 109 lines)
    ensure_quick_benchmark_csv_exists(&repo_root);

    let profile_targets = [
        (
            "quick",
            31500u64,
            GOLDEN_CSV_HASH_QUICK,
            repo_root.join("tests/benchmarks/quick"),
        ),
        (
            "standard",
            6997200u64,
            GOLDEN_CSV_HASH_STANDARD,
            repo_root.join("tests/benchmarks/standard"),
        ),
        (
            "thorough",
            149992500u64,
            GOLDEN_CSV_HASH_THOROUGH,
            repo_root.join("tests/benchmarks/thorough"),
        ),
    ];

    let mut total_opcodes_verified = 0;
    let mut profiles_verified = 0;

    for (profile_name, expected_total_ops, expected_hash, dir) in &profile_targets {
        let latest_csv = dir.join("m68k_benchmark_latest.csv");
        let canonical_csv = dir.join("m68k_benchmark.csv");

        // Validate m68k_benchmark_latest.csv if present
        if latest_csv.exists() {
            match validate_benchmark_csv_file(
                &latest_csv,
                *expected_total_ops,
                *expected_hash,
                profile_name,
            ) {
                Ok(count) => {
                    total_opcodes_verified += count;
                    profiles_verified += 1;
                }
                Err(e) => panic!(
                    "Validation failed for {}: {}\nACTION: Regenerate with 'cargo run -p test_runner --release -- bench --{}'",
                    latest_csv.display(),
                    e,
                    profile_name
                ),
            }
        }

        // Validate canonical m68k_benchmark.csv if present
        if canonical_csv.exists() {
            let _ = validate_benchmark_csv_file(
                &canonical_csv,
                *expected_total_ops,
                *expected_hash,
                profile_name,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "Validation failed for canonical {}: {}",
                    canonical_csv.display(),
                    e
                )
            });
        }
    }

    println!(
        "[*] Verified {} profiles and {} opcode row entries (target: 3 profiles x 108 = 324 entries)",
        profiles_verified, total_opcodes_verified
    );
    assert!(
        total_opcodes_verified >= 108,
        "At least quick profile (108 opcodes) must be verified"
    );
}

#[test]
fn test_disk_benchmark_baseline_csv() {
    let repo_root = match find_repo_root() {
        Some(r) => r,
        None => return,
    };

    let baseline_path = repo_root.join("tests/benchmarks/m68k_benchmark_baseline.csv");
    if !baseline_path.exists() {
        return;
    }

    let content = fs::read_to_string(&baseline_path).expect("Failed to read baseline CSV");
    let raw_lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

    assert_eq!(
        raw_lines.len(),
        109,
        "Baseline CSV must contain header + 108 opcodes"
    );

    let mut catalog_lines = Vec::with_capacity(108);
    for (idx, line) in raw_lines[1..].iter().enumerate() {
        let cols: Vec<&str> = line.split(',').collect();
        assert!(
            cols.len() >= 7,
            "Baseline row {} has fewer than 7 columns",
            idx + 1
        );
        catalog_lines.push(format!(
            "{},{},{},{},{},{}",
            cols[1], cols[2], cols[3], cols[4], cols[5], cols[6]
        ));
    }

    let cat_text = catalog_lines.join("\n") + "\n";
    let cat_hash = fnv1a_64_normalized(&cat_text);
    assert_eq!(
        cat_hash, GOLDEN_CATALOG_STRUCTURE_HASH,
        "\n🔴 MASTER BASELINE CATALOG HASH MISMATCH!\nExpected: 0x{:016X}\nActual:   0x{:016X}\n⚠️ ANTI-TAMPER RULE: Do NOT modify golden constants or assertions!",
        GOLDEN_CATALOG_STRUCTURE_HASH, cat_hash
    );
}

#[test]
fn test_cross_profile_csv_invariance_direct_diff() {
    let repo_root = match find_repo_root() {
        Some(r) => r,
        None => return,
    };

    ensure_quick_benchmark_csv_exists(&repo_root);

    let quick_path = repo_root.join("tests/benchmarks/quick/m68k_benchmark_latest.csv");
    let standard_path = repo_root.join("tests/benchmarks/standard/m68k_benchmark_latest.csv");
    let thorough_path = repo_root.join("tests/benchmarks/thorough/m68k_benchmark_latest.csv");

    if !quick_path.exists() || !standard_path.exists() || !thorough_path.exists() {
        return;
    }

    let q_content = fs::read_to_string(&quick_path).expect("Failed to read quick CSV");
    let s_content = fs::read_to_string(&standard_path).expect("Failed to read standard CSV");
    let t_content = fs::read_to_string(&thorough_path).expect("Failed to read thorough CSV");

    let q_lines: Vec<&str> = q_content.lines().filter(|l| !l.trim().is_empty()).collect();
    let s_lines: Vec<&str> = s_content.lines().filter(|l| !l.trim().is_empty()).collect();
    let t_lines: Vec<&str> = t_content.lines().filter(|l| !l.trim().is_empty()).collect();

    assert_eq!(q_lines.len(), 109, "Quick CSV must have 109 lines");
    assert_eq!(s_lines.len(), 109, "Standard CSV must have 109 lines");
    assert_eq!(t_lines.len(), 109, "Thorough CSV must have 109 lines");

    // Header match
    assert_eq!(
        q_lines[0], s_lines[0],
        "Quick and Standard headers must match"
    );
    assert_eq!(
        q_lines[0], t_lines[0],
        "Quick and Thorough headers must match"
    );

    // Row-by-row comparison of columns 1..=6 (mnemonic, variant, mode, category, opcode, amiga_cck)
    for row in 1..109 {
        let q_cols: Vec<&str> = q_lines[row].split(',').collect();
        let s_cols: Vec<&str> = s_lines[row].split(',').collect();
        let t_cols: Vec<&str> = t_lines[row].split(',').collect();

        assert_eq!(
            q_cols[1..=6],
            s_cols[1..=6],
            "\n🔴 DIVERGENCE BETWEEN QUICK AND STANDARD at row {}:\nQuick:    {:?}\nStandard: {:?}",
            row,
            &q_cols[1..=6],
            &s_cols[1..=6]
        );
        assert_eq!(
            q_cols[1..=6],
            t_cols[1..=6],
            "\n🔴 DIVERGENCE BETWEEN QUICK AND THOROUGH at row {}:\nQuick:    {:?}\nThorough: {:?}",
            row,
            &q_cols[1..=6],
            &t_cols[1..=6]
        );
    }
}

#[test]
fn test_all_324_individual_golden_row_hashes_in_memory() {
    let profiles = [
        (0usize, 31500u64, "Quick"),
        (108usize, 6997200u64, "Standard"),
        (216usize, 149992500u64, "Thorough"),
    ];

    for (offset, total_ops, prof_name) in profiles {
        let lines = synthesize_static_csv_lines(total_ops);
        assert_eq!(lines.len(), 108);
        for (idx, line) in lines.iter().enumerate() {
            let actual_hash = fnv1a_64_normalized(line);
            let expected_hash = GOLDEN_ROW_HASHES_324[offset + idx];
            assert_eq!(
                actual_hash, expected_hash,
                "\n🔴 IN-MEMORY ROW HASH MISMATCH for {} row {}: {}\nExpected: 0x{:016X}\nActual:   0x{:016X}",
                prof_name, idx, line, expected_hash, actual_hash
            );
        }
    }
}
