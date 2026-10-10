#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Direct benchmark CSV reference comparisons and measurement schema checks.

use std::fs;
use std::path::{Path, PathBuf};
use test_runner::benchmark::{
    all_benchmark_specs, run_benchmark_suite, BenchmarkConfig, BenchmarkProfile,
};

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

/// Reads deterministic catalog fields from the tracked baseline without modifying it.
fn reference_catalog_lines() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let content = fs::read_to_string(root.join("tests/benchmarks/m68k_benchmark_baseline.csv"))
        .expect("Failed to read reference CSV");
    let rows: Vec<_> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(
        rows.len(),
        109,
        "Reference CSV must have a header and 108 rows"
    );
    rows[1..]
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let columns: Vec<_> = row.split(',').collect();
            assert_eq!(
                columns.len(),
                14,
                "Reference row {} has invalid columns",
                index + 1
            );
            columns[1..=6].join(",")
        })
        .collect()
}

fn compare_rows(actual: &[String], expected: &[String], label: &str) {
    assert_eq!(actual.len(), expected.len(), "{label}: row count differs");
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(actual, expected, "{label}: row {} differs", index + 1);
    }
}

/// Ensures that the quick profile benchmark CSV exists on disk with full 108 specifications.
/// If missing or incomplete (< 109 lines), runs the quick suite.
fn ensure_quick_benchmark_csv_exists(repo_root: &Path) {
    let quick_csv = repo_root.join(".agent/tmp/benchmark-csv/quick/m68k_benchmark_latest.csv");
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
            out_dir: repo_root.join(".agent/tmp/benchmark-csv"),
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
/// - Direct reference comparison for deterministic columns 1..=7
fn validate_benchmark_csv_file(
    path: &Path,
    expected_total_ops: u64,
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
    let reference = reference_catalog_lines();

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
            cols[1],
            spec.mnemonic,
            "\n🔴 BENCHMARK CATALOG MISMATCH in row {} of {}:\nExpected mnemonic: {}\nFound: {}",
            row_idx + 1,
            path.display(),
            spec.mnemonic,
            cols[1]
        );
        assert_eq!(
            cols[2],
            expected_variant,
            "\n🔴 BENCHMARK CATALOG MISMATCH in row {} of {}:\nExpected variant: {}\nFound: {}",
            row_idx + 1,
            path.display(),
            expected_variant,
            cols[2]
        );
        assert_eq!(
            cols[3],
            spec.addressing_mode.name(),
            "\n🔴 BENCHMARK CATALOG MISMATCH in row {} of {}:\nExpected mode: {}\nFound: {}",
            row_idx + 1,
            path.display(),
            spec.addressing_mode.name(),
            cols[3]
        );
        assert_eq!(
            cols[4],
            spec.category.name(),
            "\n🔴 BENCHMARK CATALOG MISMATCH in row {} of {}:\nExpected category: {}\nFound: {}",
            row_idx + 1,
            path.display(),
            spec.category.name(),
            cols[4]
        );
        assert_eq!(
            cols[5],
            expected_opcode_hex,
            "\n🔴 BENCHMARK CATALOG MISMATCH in row {} of {}:\nExpected opcode hex: {}\nFound: {}",
            row_idx + 1,
            path.display(),
            expected_opcode_hex,
            cols[5]
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
            amiga_cck,
            spec.amiga_cck,
            "\n🔴 BENCHMARK CCK MISMATCH in row {} of {}:\nExpected CCK: {}\nFound: {}",
            row_idx + 1,
            path.display(),
            spec.amiga_cck,
            amiga_cck
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

        assert_eq!(
            row_static_str,
            format!("{},{}", reference[row_idx], expected_total_ops),
            "{} profile: reference fields differ at row {}",
            profile_name,
            row_idx + 1
        );
    }

    Ok(108)
}

#[test]
fn test_benchmark_catalog_matches_reference_csv() {
    compare_rows(
        &synthesize_catalog_structure_lines(),
        &reference_catalog_lines(),
        "Catalog",
    );
}

#[test]
fn test_benchmark_profiles_match_reference_csv_fields() {
    let reference = reference_catalog_lines();
    for (profile, expected_ops) in [
        (BenchmarkProfile::Quick, 31_500u64),
        (BenchmarkProfile::Standard, 6_997_200),
        (BenchmarkProfile::Thorough, 149_992_500),
    ] {
        let total_ops = 700 * profile.default_iterations() as u64 * profile.default_passes() as u64;
        let expected: Vec<_> = reference
            .iter()
            .map(|line| format!("{line},{expected_ops}"))
            .collect();
        compare_rows(
            &synthesize_static_csv_lines(total_ops),
            &expected,
            profile.name(),
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
            repo_root.join(".agent/tmp/benchmark-csv/quick"),
        ),
        (
            "standard",
            6997200u64,
            repo_root.join(".agent/tmp/benchmark-csv/standard"),
        ),
        (
            "thorough",
            149992500u64,
            repo_root.join(".agent/tmp/benchmark-csv/thorough"),
        ),
    ];

    let mut total_opcodes_verified = 0;
    let mut profiles_verified = 0;

    for (profile_name, expected_total_ops, dir) in &profile_targets {
        let latest_csv = dir.join("m68k_benchmark_latest.csv");
        let canonical_csv = dir.join("m68k_benchmark.csv");

        // Validate m68k_benchmark_latest.csv if present
        if latest_csv.exists() {
            match validate_benchmark_csv_file(
                &latest_csv,
                *expected_total_ops,
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
            let _ = validate_benchmark_csv_file(&canonical_csv, *expected_total_ops, profile_name)
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

    compare_rows(
        &synthesize_catalog_structure_lines(),
        &catalog_lines,
        "Baseline catalog",
    );
}

#[test]
fn test_cross_profile_csv_invariance_direct_diff() {
    let repo_root = match find_repo_root() {
        Some(r) => r,
        None => return,
    };

    ensure_quick_benchmark_csv_exists(&repo_root);

    let quick_path = repo_root.join(".agent/tmp/benchmark-csv/quick/m68k_benchmark_latest.csv");
    let standard_path =
        repo_root.join(".agent/tmp/benchmark-csv/standard/m68k_benchmark_latest.csv");
    let thorough_path =
        repo_root.join(".agent/tmp/benchmark-csv/thorough/m68k_benchmark_latest.csv");

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
