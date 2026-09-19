//! Automated Architecture & Engineering Rules Validation Tests
//!
//! Enforces guidelines from AGENTS.md and .agents/rules/:
//! 1. Rust source file size limit: <= 800 lines in crates/*/src/ (with recognized exceptions).
//!    Documentation (.md) has no line count limits.
//! 2. Zero runtime panics: no `.unwrap()` / `.expect()` in core emulation crates.
//! 3. Strict path privacy: zero hardcoded user/host paths.
//! 4. Strict macro prohibition: zero `macro_rules!` definitions in workspace crates.
//! 5. Rule files size safety: AGENTS.md <= 14,000 bytes (constitutional non-redundancy) and .agents/rules/*.md <= 23,000 bytes (prompt truncation safety).

use std::fs;
use std::path::{Path, PathBuf};

/// Recognized exceptions allowed to exceed the 800-line threshold
/// (e.g. compile-time static dispatch tables, exhaustive linear instruction decoders/slices).
const LINE_COUNT_EXCEPTIONS: &[&str] = &[
    "dispatch_table.rs",
    "move_w.rs",
    "move_b.rs",
    "move_l.rs",
    "add.rs",
    "sub.rs",
    "and.rs",
    "or.rs",
    "cmpi.rs",
];

/// Core emulation crates where `.unwrap()` and `.expect()` are strictly forbidden in runtime code.
const CORE_EMULATION_CRATES: &[&str] = &[
    "m68000",
    "physical_memory",
    "memory_bus",
    "config",
    "rtc",
    "disassembler",
    "debugger",
    "copper",
    "blitter",
    "dma",
    "agnus",
    "sprites",
    "frame_builder",
    "mouse",
    "joystick",
    "denise",
    "audio",
    "floppy",
    "interrupts",
    "paula",
    "keyboard",
    "game_ports",
    "cia",
    "machine_loop",
];

fn find_repo_root() -> PathBuf {
    // Current test binary runs in target/debug/deps, CWD is repo root
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    if cwd.join("ROADMAP.md").exists() {
        return cwd;
    }
    // Fallback: search parent directories
    let mut dir = cwd;
    while let Some(parent) = dir.parent() {
        if parent.join("ROADMAP.md").exists() {
            return parent.to_path_buf();
        }
        dir = parent.to_path_buf();
    }
    panic!("Could not locate repository root containing ROADMAP.md");
}

fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, files);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
}

#[test]
fn test_file_size_limits() {
    let repo_root = find_repo_root();
    let crates_dir = repo_root.join("crates");
    let mut rs_files = Vec::new();
    collect_rs_files(&crates_dir, &mut rs_files);

    let mut violations = Vec::new();

    for file in rs_files {
        // Only inspect production files under src/
        if !file.components().any(|c| c.as_os_str() == "src") {
            continue;
        }

        let file_name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if LINE_COUNT_EXCEPTIONS.contains(&file_name) {
            continue;
        }

        let content = fs::read_to_string(&file).expect("Failed to read file");
        let line_count = content.lines().count();

        if line_count > 800 {
            let rel_path = file.strip_prefix(&repo_root).unwrap_or(&file);
            violations.push(format!(
                "{} ({} lines > 800 line limit)",
                rel_path.display(),
                line_count
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: The following Rust source file(s) exceed the 800-line limit per AGENTS.md:\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_no_stale_line_count_exceptions() {
    let repo_root = find_repo_root();
    let crates_dir = repo_root.join("crates");
    let mut rs_files = Vec::new();
    collect_rs_files(&crates_dir, &mut rs_files);

    let mut stale_exceptions = Vec::new();

    for &exception_name in LINE_COUNT_EXCEPTIONS {
        let matching_files: Vec<&PathBuf> = rs_files
            .iter()
            .filter(|f| {
                f.components().any(|c| c.as_os_str() == "src")
                    && f.file_name().and_then(|n| n.to_str()) == Some(exception_name)
            })
            .collect();

        if matching_files.is_empty() {
            stale_exceptions.push(format!(
                "Exception '{}' does not match any production file under crates/*/src/",
                exception_name
            ));
            continue;
        }

        for file in matching_files {
            let content = fs::read_to_string(file).expect("Failed to read exception file");
            let line_count = content.lines().count();
            if line_count <= 800 {
                let rel_path = file.strip_prefix(&repo_root).unwrap_or(file);
                stale_exceptions.push(format!(
                    "{} ({} lines <= 800 limit; exception is stale and requires manual user approval to prune)",
                    rel_path.display(),
                    line_count
                ));
            }
        }
    }

    assert!(
        stale_exceptions.is_empty(),
        "Architecture Rule Violation: Found stale or missing LINE_COUNT_EXCEPTIONS.\n\
         Automated or silent exception list modifications are strictly forbidden.\n\
         Notify the user for explicit confirmation before removing any entry:\n{}",
        stale_exceptions.join("\n")
    );
}

#[test]
fn test_zero_runtime_panics_or_unwraps() {
    let repo_root = find_repo_root();
    let mut violations = Vec::new();

    for &crate_name in CORE_EMULATION_CRATES {
        let src_dir = repo_root.join("crates").join(crate_name).join("src");
        if !src_dir.exists() {
            continue;
        }

        let mut rs_files = Vec::new();
        collect_rs_files(&src_dir, &mut rs_files);

        for file in rs_files {
            let content = fs::read_to_string(&file).expect("Failed to read file");
            let mut in_block_comment = false;

            for (line_idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();

                // Handle multi-line comment state
                if in_block_comment {
                    if let Some(end_idx) = trimmed.find("*/") {
                        in_block_comment = false;
                        // Continue checking line after comment end
                        let rest = &trimmed[end_idx + 2..];
                        if rest.contains(".unwrap()") || rest.contains(".expect(") {
                            let rel_path = file.strip_prefix(&repo_root).unwrap_or(&file);
                            violations.push(format!(
                                "{}:{} -> {}",
                                rel_path.display(),
                                line_idx + 1,
                                trimmed
                            ));
                        }
                    }
                    continue;
                }

                if trimmed.starts_with("/*") {
                    if !trimmed.contains("*/") {
                        in_block_comment = true;
                    }
                    continue;
                }

                // Ignore single-line comments
                if trimmed.starts_with("//") {
                    continue;
                }

                // Strip inline comments
                let code_part = if let Some(idx) = trimmed.find("//") {
                    &trimmed[..idx]
                } else {
                    trimmed
                };

                if code_part.contains(".unwrap()") || code_part.contains(".expect(") {
                    let rel_path = file.strip_prefix(&repo_root).unwrap_or(&file);
                    violations.push(format!(
                        "{}:{} -> {}",
                        rel_path.display(),
                        line_idx + 1,
                        trimmed
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Found .unwrap() or .expect() in core emulation crates:\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_no_external_hardcoded_paths() {
    let repo_root = find_repo_root();
    let mut files_to_check = Vec::new();

    // Check crates source files
    let crates_dir = repo_root.join("crates");
    collect_rs_files(&crates_dir, &mut files_to_check);

    let mut violations = Vec::new();
    let forbidden_patterns = ["C:\\Users\\", "C:/Users/", "/home/", "Google Drive"];

    for file in files_to_check {
        // Skip this test file itself from the literal pattern search
        if file.file_name().and_then(|n| n.to_str()) == Some("test_architecture_rules.rs") {
            continue;
        }

        let content = fs::read_to_string(&file).expect("Failed to read file");
        for (line_idx, line) in content.lines().enumerate() {
            for pattern in &forbidden_patterns {
                if line.contains(pattern) {
                    let rel_path = file.strip_prefix(&repo_root).unwrap_or(&file);
                    violations.push(format!(
                        "{}:{} contains forbidden external path pattern '{}'",
                        rel_path.display(),
                        line_idx + 1,
                        pattern
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Path Privacy Violation: Found hardcoded external paths per AGENTS.md:\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_zero_user_defined_macros() {
    let repo_root = find_repo_root();
    let mut files_to_check = Vec::new();

    let crates_dir = repo_root.join("crates");
    collect_rs_files(&crates_dir, &mut files_to_check);

    let mut violations = Vec::new();

    for file in files_to_check {
        // Skip this test file itself from the literal pattern search
        if file.file_name().and_then(|n| n.to_str()) == Some("test_architecture_rules.rs") {
            continue;
        }

        let content = fs::read_to_string(&file).expect("Failed to read file");
        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            if trimmed.contains("macro_rules!") {
                let rel_path = file.strip_prefix(&repo_root).unwrap_or(&file);
                violations.push(format!(
                    "{}:{} -> {}",
                    rel_path.display(),
                    line_idx + 1,
                    trimmed
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Found custom macros (`macro_rules!`) which are strictly forbidden per AGENTS.md:\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_zero_const_generic_handlers() {
    let repo_root = find_repo_root();
    let m68k_src = repo_root.join("crates").join("m68000").join("src");
    let mut files_to_check = Vec::new();
    collect_rs_files(&m68k_src, &mut files_to_check);

    let mut violations = Vec::new();

    for file in files_to_check {
        let content = fs::read_to_string(&file).expect("Failed to read file");
        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            if trimmed.contains("<const ") {
                let rel_path = file.strip_prefix(&repo_root).unwrap_or(&file);
                violations.push(format!(
                    "{}:{} -> {}",
                    rel_path.display(),
                    line_idx + 1,
                    trimmed
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Found const-generic functions (`<const N: ...>`) in M68000 core, strictly forbidden per AGENTS.md:\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_audit_micro_step_coverage() {
    let mut micro_covered = 0;

    for op in 0..=65535usize {
        let desc = &m68000::micro::OPCODE_DESCRIPTOR_TABLE[op];
        if !desc.steps.is_empty() {
            micro_covered += 1;
        }
    }

    println!("AUDIT RESULTS:");
    println!(
        "  Micro-step covered: {} / 65536 ({:.2}%)",
        micro_covered,
        (micro_covered as f64 / 65536.0) * 100.0
    );
    assert!(
        micro_covered >= 15000,
        "Expected >=15000 opcodes covered, got {}",
        micro_covered
    );
}

#[test]
fn test_code_formatting_compliance() {
    let repo_root = find_repo_root();
    let status = std::process::Command::new("cargo")
        .args(["fmt", "--all", "--", "--check"])
        .current_dir(&repo_root)
        .status()
        .expect("Failed to execute `cargo fmt` check");

    assert!(
        status.success(),
        "Architecture Rule Violation: Code is not formatted according to `cargo fmt`. Run `cargo fmt --all` to resolve formatting issues."
    );
}

#[test]
fn test_inlining_guidelines_compliance() {
    let repo_root = find_repo_root();

    // 1. Cold exception/trap trigger paths must have #[inline(never)]
    let mut m68k_files = Vec::new();
    collect_rs_files(&repo_root.join("crates/m68000/src"), &mut m68k_files);

    for file in &m68k_files {
        let content = fs::read_to_string(file).expect("Failed to read file");
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("pub fn trigger_") || trimmed.starts_with("fn trigger_") {
                let prev_lines = if i >= 2 {
                    &lines[i - 2..i]
                } else {
                    &lines[..i]
                };
                let has_inline_never = prev_lines.iter().any(|l| l.contains("#[inline(never)]"));
                assert!(
                    has_inline_never,
                    "Inlining Guideline Violation: Cold exception trigger `{}` in {} must be annotated with #[inline(never)]",
                    trimmed,
                    file.strip_prefix(&repo_root).unwrap_or(file).display()
                );
            }
        }
    }

    // 2. CCR bitwise setter methods in state.rs must have #[inline(always)]
    let ccr_setters = [
        "pub fn set_ccr_xnzvc",
        "pub fn set_ccr_nzvc",
        "pub fn set_ccr_nz_clear_vc",
        "pub fn set_ccr_z_only",
        "pub fn set_ccr_v_clear_c",
    ];
    let state_rs = repo_root.join("crates/m68000/src/state.rs");
    let content = fs::read_to_string(&state_rs).expect("Failed to read state.rs");
    let lines: Vec<&str> = content.lines().collect();
    for setter in &ccr_setters {
        let mut found = false;
        for (i, line) in lines.iter().enumerate() {
            if line.contains(setter) {
                found = true;
                let prev_lines = if i >= 2 {
                    &lines[i - 2..i]
                } else {
                    &lines[..i]
                };
                let has_inline_always = prev_lines.iter().any(|l| l.contains("#[inline(always)]"));
                assert!(
                    has_inline_always,
                    "Inlining Guideline Violation: CCR setter `{}` in state.rs must be annotated with #[inline(always)]",
                    setter
                );
            }
        }
        assert!(found, "Could not find CCR setter `{}` in state.rs", setter);
    }

    // 3. Leaf ALU arithmetic and shift functions in instructions/ must have #[inline(always)]
    let leaf_prefixes = [
        "pub fn add_",
        "pub fn sub_",
        "pub fn and_",
        "pub fn or_",
        "pub fn eor_",
        "pub fn cmp_",
        "pub fn asr_",
        "pub fn asl_",
        "pub fn lsr_",
        "pub fn lsl_",
        "pub fn ror_",
        "pub fn rol_",
        "pub fn roxr_",
        "pub fn roxl_",
        "pub fn neg_",
        "pub fn negx_",
        "pub fn not_",
        "pub fn tst_",
        "pub fn abcd_",
        "pub fn sbcd_",
        "pub fn nbcd_",
        "pub fn bchg_",
        "pub fn bclr_",
        "pub fn bset_",
        "pub fn btst_",
    ];
    let inst_dir = repo_root.join("crates/m68000/src/instructions");
    let mut inst_files = Vec::new();
    collect_rs_files(&inst_dir, &mut inst_files);

    for file in &inst_files {
        let content = fs::read_to_string(file).expect("Failed to read file");
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            for prefix in &leaf_prefixes {
                if trimmed.starts_with(prefix) {
                    let prev_lines = if i >= 2 {
                        &lines[i - 2..i]
                    } else {
                        &lines[..i]
                    };
                    let has_inline_always =
                        prev_lines.iter().any(|l| l.contains("#[inline(always)]"));
                    assert!(
                        has_inline_always,
                        "Inlining Guideline Violation: Leaf ALU function `{}` in {} must be annotated with #[inline(always)]",
                        trimmed,
                        file.strip_prefix(&repo_root).unwrap_or(file).display()
                    );
                }
            }
        }
    }
}

fn collect_subdirectories_recursive(dir: &Path, repo_root: &Path, subdirs: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let rel_path = path.strip_prefix(repo_root).unwrap_or(&path);
                subdirs.push(rel_path.display().to_string());
                collect_subdirectories_recursive(&path, repo_root, subdirs);
            }
        }
    }
}

#[test]
fn test_flat_instruction_hierarchy_and_zero_subdirectories() {
    let repo_root = find_repo_root();
    let inst_dir = repo_root
        .join("crates")
        .join("m68000")
        .join("src")
        .join("instructions");
    assert!(
        inst_dir.exists(),
        "M68000 instructions directory does not exist: {}",
        inst_dir.display()
    );

    // 1. Assert zero subdirectories exist at any recursion depth
    let mut subdirectories = Vec::new();
    collect_subdirectories_recursive(&inst_dir, &repo_root, &mut subdirectories);
    assert!(
        subdirectories.is_empty(),
        "Architecture Rule Violation: Subdirectories in `crates/m68000/src/instructions/` are strictly forbidden per AGENTS.md.\n\
        All instructions must be flat `<mnemonic>.rs` files directly under `instructions/`.\n\
        Found subdirectories:\n{}",
        subdirectories.join("\n")
    );

    // 2. Assert that legacy bundled multi-instruction files are not present
    let forbidden_legacy_files = [
        "mul.rs",
        "div.rs",
        "link_unlk.rs",
        "bcd.rs",
        "privileged.rs",
        "bits.rs",
        "shifts.rs",
        "system.rs",
    ];
    let mut forbidden_found = Vec::new();
    for file_name in &forbidden_legacy_files {
        if inst_dir.join(file_name).exists() {
            forbidden_found.push(*file_name);
        }
    }
    assert!(
        forbidden_found.is_empty(),
        "Architecture Rule Violation: Legacy bundled instruction file(s) found in `crates/m68000/src/instructions/`:\n\
        {:?}\n\
        Instructions must adhere to 1:1 mnemonic-to-file mapping (e.g. mulu.rs/muls.rs, divu.rs/divs.rs, link.rs/unlk.rs, abcd.rs/sbcd.rs/nbcd.rs, trapv.rs/rtr.rs/rte.rs/stop.rs/reset.rs/move_usp.rs).",
        forbidden_found
    );

    // 3. Assert that every entry in instructions/ is a valid .rs file
    let entries = fs::read_dir(&inst_dir).expect("Failed to read instructions directory");
    let mut non_rs_files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            let rel_path = path.strip_prefix(&repo_root).unwrap_or(&path);
            non_rs_files.push(rel_path.display().to_string());
        }
    }
    assert!(
        non_rs_files.is_empty(),
        "Architecture Rule Violation: Found non-Rust source file(s) in `crates/m68000/src/instructions/`:\n{}",
        non_rs_files.join("\n")
    );
}

#[test]
fn test_idle_microstep_naming_and_prohibition_of_anonymous_idle_structs() {
    let repo_root = find_repo_root();
    let inst_dir = repo_root
        .join("crates")
        .join("m68000")
        .join("src")
        .join("instructions");
    let mut inst_files = Vec::new();
    collect_rs_files(&inst_dir, &mut inst_files);

    let forbidden_legacy_aliases = [
        "READ_WORD_FINISH",
        "READ_BYTE_FINISH",
        "PREFETCH_NEXT_RETIRE",
        "REFILL_FIRST_FINISH",
        "REFILL_SECOND_FINISH",
        "READ_TARGET_OPCODE_FINISH",
        "READ_VECTOR_HIGH_FINISH",
        "ALU_INTERNAL_2CLK",
    ];

    let mut violations = Vec::new();

    for file in &inst_files {
        let content = fs::read_to_string(file).expect("Failed to read instruction file");
        let rel_path = file.strip_prefix(&repo_root).unwrap_or(file);
        let lines: Vec<&str> = content.lines().collect();

        // 1. Check for legacy non-idle finish/retire aliases
        for (line_idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            for alias in &forbidden_legacy_aliases {
                if trimmed.contains(alias) {
                    violations.push(format!(
                        "{}:{} -> Uses legacy alias `{}`. Use `common::BUS_READ_IDLE` or `common::ALU_IDLE` instead.",
                        rel_path.display(),
                        line_idx + 1,
                        alias
                    ));
                }
            }
        }

        // 2. Check for anonymous idle MicroStep structs (bus_fn: None, alu_fn: None)
        for (i, line) in lines.iter().enumerate() {
            if line.contains("MicroStep {") {
                let end_idx = std::cmp::min(i + 6, lines.len());
                let block = lines[i..end_idx].join(" ");
                if block.contains("bus_fn: None") && block.contains("alu_fn: None") {
                    let canonical_hint = if block.contains("base_clocks: 2") {
                        "common::ALU_IDLE"
                    } else if block.contains("base_clocks: 4") {
                        "common::ALU_IDLE_4CLK"
                    } else if block.contains("base_clocks: 8") {
                        "common::ALU_IDLE_8CLK"
                    } else if block.contains("base_clocks: 128") {
                        "common::ALU_IDLE_128CLK"
                    } else {
                        "standardized idle constant from `common`"
                    };

                    violations.push(format!(
                        "{}:{} -> Anonymous idle MicroStep struct found. Replace with `{}`.",
                        rel_path.display(),
                        i + 1,
                        canonical_hint
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Non-standard or anonymous idle micro-step usage found in `crates/m68000/src/instructions/`:\n\
        All micro-steps performing no active memory bus transfer must use standardized constants with `IDLE` in their name.\n\
        Violations:\n{}",
        violations.join("\n")
    );
}

/// Maximum allowable byte size for any rule file (`.agents/rules/*.md` or `GEMINI.md`).
/// Antigravity prompt injection silently truncates individual rule files exceeding ~24,000 bytes.
/// A strict safety ceiling of 23,000 bytes guarantees zero risk of truncation across all agent sessions.
const MAX_RULE_FILE_BYTES: u64 = 23_000;

/// Maximum allowable byte size for root constitutional file (`AGENTS.md`).
/// AGENTS.md must strictly serve as an architectural constitution and index, never duplicating
/// modularized rules from `.agents/rules/*.md`. A tight ceiling of 14,000 bytes permanently
/// prevents prompt bloat and redundant rule replication.
const MAX_AGENTS_MD_BYTES: u64 = 14_000;

#[test]
fn test_rule_files_size_limit_and_truncation_safety() {
    let repo_root = find_repo_root();
    let mut violations = Vec::new();

    // 1. Check root AGENTS.md against tightened constitutional limit
    let agents_path = repo_root.join("AGENTS.md");
    if agents_path.exists() {
        if let Ok(meta) = fs::metadata(&agents_path) {
            if meta.len() > MAX_AGENTS_MD_BYTES {
                violations.push(format!(
                    "AGENTS.md is {} bytes (exceeds constitutional non-redundancy ceiling of {} bytes; modularize rules into .agents/rules/)",
                    meta.len(),
                    MAX_AGENTS_MD_BYTES
                ));
            }
        }
    }

    // Check GEMINI.md against prompt injection safety ceiling
    let gemini_path = repo_root.join("GEMINI.md");
    if gemini_path.exists() {
        if let Ok(meta) = fs::metadata(&gemini_path) {
            if meta.len() > MAX_RULE_FILE_BYTES {
                violations.push(format!(
                    "GEMINI.md is {} bytes (exceeds safety ceiling of {} bytes; risk of silent prompt truncation at ~24 KB)",
                    meta.len(),
                    MAX_RULE_FILE_BYTES
                ));
            }
        }
    }

    // 2. Check all markdown rule files in .agents/rules/
    let rules_dir = repo_root.join(".agents").join("rules");
    if rules_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&rules_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                    if let Ok(meta) = fs::metadata(&path) {
                        if meta.len() > MAX_RULE_FILE_BYTES {
                            violations.push(format!(
                                ".agents/rules/{} is {} bytes (exceeds safety ceiling of {} bytes; risk of silent prompt truncation at ~24 KB)",
                                path.file_name().unwrap_or_default().to_string_lossy(),
                                meta.len(),
                                MAX_RULE_FILE_BYTES
                            ));
                        }
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Rule file(s) exceed size safety thresholds:\n{}\n\
        Antigravity silently truncates rule files exceeding ~24,000 bytes with `<truncated N bytes>`. \
        Keep AGENTS.md (<= {} bytes) and individual .agents/rules/*.md files (<= {} bytes) concise and modularized.",
        violations.join("\n"),
        MAX_AGENTS_MD_BYTES,
        MAX_RULE_FILE_BYTES
    );
}

#[test]
fn test_golden_hash_anti_tamper_policy_compliance() {
    let repo_root = find_repo_root();
    let tests_dir = repo_root.join("crates").join("test_runner").join("tests");

    let target_files = [
        ("test_benchmark_trace.rs", true),
        ("test_benchmark_csv.rs", true),
        ("test_golden_row_hashes.rs", false),
    ];

    let mut violations = Vec::new();

    for (file_name, check_prompts) in target_files {
        let path = tests_dir.join(file_name);
        if !path.exists() {
            violations.push(format!(
                "Missing expected golden benchmark test file: {}",
                file_name
            ));
            continue;
        }

        let content = fs::read_to_string(&path).expect("Failed to read benchmark test file");

        // 1. Must contain explicit ANTI-TAMPER policy contract header
        if !content.contains("ANTI-TAMPER POLICY & INVARIANCE CONTRACT") {
            violations.push(format!(
                "{} is missing the mandatory `ANTI-TAMPER POLICY & INVARIANCE CONTRACT` header block",
                file_name
            ));
        }

        // 2. Must NEVER contain permissive instructions to update golden hashes
        if check_prompts {
            let lower = content.to_lowercase();
            if lower.contains("update the golden hash") || lower.contains("update golden hash") {
                violations.push(format!(
                    "{} contains permissive instruction to update golden hashes (strictly forbidden by AGENTS.md anti-tamper rule)",
                    file_name
                ));
            }
            if !content.contains("ANTI-TAMPER RULE") {
                violations.push(format!(
                    "{} does not contain `ANTI-TAMPER RULE` warning in test failure messages",
                    file_name
                ));
            }
        }
    }

    // 3. Verify AGENTS.md and spec-compliance.md codify the anti-tamper rule
    let agents_md = fs::read_to_string(repo_root.join("AGENTS.md")).unwrap_or_default();
    if !agents_md.contains("Prohibition of Blind Golden Hash Modifications") {
        violations.push(
            "AGENTS.md is missing `Prohibition of Blind Golden Hash Modifications`".to_string(),
        );
    }

    let spec_md = fs::read_to_string(
        repo_root
            .join(".agents")
            .join("rules")
            .join("spec-compliance.md"),
    )
    .unwrap_or_default();
    if !spec_md.contains("Golden Test Vector & Hash Invariance (Anti-Tamper Rule)") {
        violations.push(
            "spec-compliance.md is missing `Golden Test Vector & Hash Invariance (Anti-Tamper Rule)`"
                .to_string(),
        );
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Golden benchmark anti-tamper policy failed:\n{}",
        violations.join("\n")
    );
}

fn url_decode_path(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..=i + 2]) {
                if let Ok(b) = u8::from_str_radix(hex_str, 16) {
                    result.push(b);
                    i += 3;
                    continue;
                }
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

/// Enforces vault linking integrity across all design documents in Obsidian/Amiga/Design/.
/// Verifies zero broken links and active dual-layer linking standard.
#[test]
fn test_obsidian_design_docs_links_integrity() {
    let repo_root = find_repo_root();
    let design_dir = repo_root.join("Obsidian").join("Amiga").join("Design");
    assert!(
        design_dir.is_dir(),
        "Obsidian design directory does not exist: {}",
        design_dir.display()
    );

    let entries =
        fs::read_dir(&design_dir).expect("Failed to read Obsidian/Amiga/Design directory");
    let mut md_files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            md_files.push(path);
        }
    }
    md_files.sort();

    assert!(
        md_files.len() >= 25,
        "Expected at least 25 design documents, found {}",
        md_files.len()
    );

    let mut violations = Vec::new();
    let mut total_links = 0;

    for file in &md_files {
        let file_name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let content = fs::read_to_string(file).expect("Failed to read markdown file");

        let mut rem = content.as_str();
        while let Some(start_idx) = rem.find("](") {
            let after = &rem[start_idx + 2..];
            if let Some(end_idx) = after.find(')') {
                let link = after[..end_idx].trim();
                rem = &after[end_idx + 1..];

                if link.starts_with("http://")
                    || link.starts_with("https://")
                    || link.starts_with("mailto:")
                {
                    continue;
                }

                let path_part = link.split('#').next().unwrap_or("").trim();
                if path_part.is_empty() {
                    continue;
                }

                total_links += 1;
                let decoded = url_decode_path(path_part);
                let target = design_dir.join(&decoded);

                if !target.exists() {
                    violations.push(format!(
                        "[{}] Broken link `{}` -> `{}`",
                        file_name,
                        link,
                        target.display()
                    ));
                }
            } else {
                break;
            }
        }
    }

    assert!(
        total_links >= 300,
        "Expected at least 300 links across design docs, found {}",
        total_links
    );

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Broken markdown links detected in Obsidian/Amiga/Design/ ({} broken links):\n{}",
        violations.len(),
        violations.join("\n")
    );
}

#[test]
fn test_zero_inline_tests_in_crates_src() {
    let repo_root = find_repo_root();
    let crates_dir = repo_root.join("crates");
    let mut rs_files = Vec::new();
    collect_rs_files(&crates_dir, &mut rs_files);

    let mut violations = Vec::new();

    for file in rs_files {
        // Only inspect production files under src/
        if !file.components().any(|c| c.as_os_str() == "src") {
            continue;
        }

        let content = fs::read_to_string(&file).expect("Failed to read source file");
        let rel_path = file.strip_prefix(&repo_root).unwrap_or(&file);

        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed == "#[cfg(test)]"
                || trimmed == "mod tests {"
                || trimmed == "mod test {"
                || trimmed.starts_with("#[test]")
            {
                violations.push(format!(
                    "{}:{} -> Found inline test attribute or module: `{}`",
                    rel_path.display(),
                    line_idx + 1,
                    trimmed
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Inline tests found in production src/ files:\n{}\n\
        All tests must be placed in dedicated test files under crates/<crate>/tests/ per .agents/rules/unit-testing-policy.md.",
        violations.join("\n")
    );
}

#[test]
fn test_zero_backward_compatibility_shims_and_stale_aliases() {
    let repo_root = find_repo_root();
    let crates_dir = repo_root.join("crates");
    let mut rs_files = Vec::new();
    collect_rs_files(&crates_dir, &mut rs_files);

    let forbidden_phrases = [
        "backward compatibility alias",
        "legacy compatibility",
        "compatibility alias",
        "run_dual_test",
    ];

    let forbidden_identifiers = [
        "reset_cold",
        "dispatch_custom_write",
        "dispatch_agnus_action",
        "dispatch_paula_action",
        "dispatch_denise_action",
        "dispatch_cia_action",
        "propagate_agnus_write",
        "propagate_paula_write",
        "propagate_denise_write",
        "propagate_cia_write",
        "inject_kickstart_rom",
        "save_state_self_contained",
    ];

    let mut violations = Vec::new();

    for file in rs_files {
        let file_name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name == "test_architecture_rules.rs" {
            continue;
        }

        let content = fs::read_to_string(&file).expect("Failed to read source file");
        let rel_path = file.strip_prefix(&repo_root).unwrap_or(&file);

        // 1. Check for forbidden phrases/aliases in comments and code
        for (line_idx, line) in content.lines().enumerate() {
            let lower = line.to_lowercase();
            for phrase in &forbidden_phrases {
                if lower.contains(phrase) {
                    violations.push(format!(
                        "{}:{} -> Contains forbidden compatibility phrase/alias `{}`",
                        rel_path.display(),
                        line_idx + 1,
                        phrase
                    ));
                }
            }
            for ident in &forbidden_identifiers {
                if line.contains(ident) {
                    violations.push(format!(
                        "{}:{} -> Contains forbidden stale/legacy identifier `{}`",
                        rel_path.display(),
                        line_idx + 1,
                        ident
                    ));
                }
            }
        }

        // 2. Check for dummy wrapper modules like `pub mod <name> { pub use ...; }` in crate root
        let file_stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let parent_dir_name = file
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let is_crate_root = file_stem == parent_dir_name;
        if is_crate_root {
            let lines: Vec<&str> = content.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("pub mod ") && trimmed.ends_with('{') {
                    let mod_name = trimmed
                        .strip_prefix("pub mod ")
                        .unwrap_or("")
                        .trim_end_matches('{')
                        .trim();
                    // Legitimate namespace modules (function_code in arbitration, micro in m68000)
                    if mod_name == "micro" || mod_name == "function_code" {
                        continue;
                    }
                    let end_idx = std::cmp::min(i + 5, lines.len());
                    let block = lines[i..end_idx].join(" ");
                    if block.contains("pub use ") && block.contains('*') {
                        violations.push(format!(
                            "{}:{} -> Dummy backward-compatibility wrapper module `{}` detected. Re-export directly or update callers per .agents/rules/workspace-structure-and-reexports.md.",
                            rel_path.display(),
                            i + 1,
                            trimmed
                        ));
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Backward-compatibility shims or stale aliases detected:\n{}\n\
        All refactorings must be atomic with zero transitional aliases per .agents/rules/workspace-structure-and-reexports.md.",
        violations.join("\n")
    );
}

#[test]
fn test_every_crate_has_dedicated_external_tests_suite() {
    let repo_root = find_repo_root();
    let crates_dir = repo_root.join("crates");

    let mut missing_tests_crates = Vec::new();

    let entries = fs::read_dir(&crates_dir).expect("Failed to read crates directory");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("Cargo.toml").exists() {
            let crate_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let tests_dir = path.join("tests");
            if !tests_dir.is_dir() {
                missing_tests_crates.push(format!(
                    "crates/{} -> Missing dedicated tests/ directory",
                    crate_name
                ));
                continue;
            }

            let mut rs_test_count = 0;
            let mut total_test_functions = 0;
            let mut total_assertions = 0;

            if let Ok(dir_entries) = fs::read_dir(&tests_dir) {
                for file_entry in dir_entries.flatten() {
                    let file_path = file_entry.path();
                    if file_path.extension().map_or(false, |ext| ext == "rs") {
                        rs_test_count += 1;
                        if let Ok(content) = fs::read_to_string(&file_path) {
                            for line in content.lines() {
                                let trimmed = line.trim();
                                if trimmed.starts_with("#[test]") {
                                    total_test_functions += 1;
                                }
                                if trimmed.contains("assert!")
                                    || trimmed.contains("assert_eq!")
                                    || trimmed.contains("assert_ne!")
                                {
                                    total_assertions += 1;
                                }
                            }
                        }
                    }
                }
            }

            if rs_test_count == 0 {
                missing_tests_crates.push(format!(
                    "crates/{} -> tests/ directory contains zero .rs test files",
                    crate_name
                ));
            } else if total_test_functions < 2 {
                missing_tests_crates.push(format!(
                    "crates/{} -> Shallow test suite: found only {} active #[test] function(s) (minimum 2 required to prevent placeholder scaffolding)",
                    crate_name, total_test_functions
                ));
            } else if total_assertions < 10 {
                missing_tests_crates.push(format!(
                    "crates/{} -> Insufficient assertion density: found only {} assertion(s) across test suite (minimum 10 required)",
                    crate_name, total_assertions
                ));
            }
        }
    }

    assert!(
        missing_tests_crates.is_empty(),
        "Architecture Rule Violation: Crates missing dedicated external test suites:\n{}\n\
        Every crate must contain an active tests/ directory with dedicated .rs unit test files per .agents/rules/unit-testing-policy.md.",
        missing_tests_crates.join("\n")
    );
}

#[test]
fn test_named_crate_roots_and_zero_generic_lib_rs() {
    let repo_root = find_repo_root();
    let crates_dir = repo_root.join("crates");

    let mut violations = Vec::new();

    // 1. Assert zero files named lib.rs exist in the repository
    let mut all_rs_files = Vec::new();
    collect_rs_files(&repo_root, &mut all_rs_files);
    for file in all_rs_files {
        if file
            .components()
            .any(|c| c.as_os_str() == "target" || c.as_os_str() == ".git")
        {
            continue;
        }
        if file.file_name().map_or(false, |n| n == "lib.rs") {
            let rel = file.strip_prefix(&repo_root).unwrap_or(&file);
            violations.push(format!(
                "{}: Generic `lib.rs` file name is strictly forbidden. Use `<crate_name>.rs` matching the crate.",
                rel.display()
            ));
        }
    }

    // 2. For each crate under crates/*, verify that src/<crate_name>.rs exists and Cargo.toml configures [lib] path
    let entries = fs::read_dir(&crates_dir).expect("Failed to read crates directory");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("Cargo.toml").exists() {
            let crate_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let expected_root = path.join("src").join(format!("{}.rs", crate_name));
            if !expected_root.is_file() {
                violations.push(format!(
                    "crates/{}: Missing expected crate root file `src/{}.rs`",
                    crate_name, crate_name
                ));
            }

            let cargo_toml = path.join("Cargo.toml");
            let toml_content = fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml");
            let expected_lib_entry = format!("path = \"src/{}.rs\"", crate_name);
            if !toml_content.contains(&expected_lib_entry) {
                violations.push(format!(
                    "crates/{}/Cargo.toml: Missing `[lib]` configuration with `{}`",
                    crate_name, expected_lib_entry
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Crate root naming violations detected:\n{}\n\
        All workspace library crates must name their entry point `src/<crate_name>.rs` and configure `[lib] path` per .agents/rules/workspace-structure-and-reexports.md.",
        violations.join("\n")
    );
}

#[test]
fn test_canonical_test_file_naming_convention() {
    let repo_root = find_repo_root();
    let crates_dir = repo_root.join("crates");

    let mut violations = Vec::new();

    let entries = fs::read_dir(&crates_dir).expect("Failed to read crates directory");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("Cargo.toml").exists() {
            let crate_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let tests_dir = path.join("tests");
            if tests_dir.is_dir() {
                if let Ok(dir_entries) = fs::read_dir(&tests_dir) {
                    for file_entry in dir_entries.flatten() {
                        let file_path = file_entry.path();
                        if file_path.is_file()
                            && file_path.extension().map_or(false, |ext| ext == "rs")
                        {
                            let file_name =
                                file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                            if !file_name.starts_with("test_") {
                                violations.push(format!(
                                    "crates/{}/tests/{}: Test file does not start with `test_` prefix",
                                    crate_name, file_name
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Non-canonical test filenames detected:\n{}\n\
        In Cargo workspaces, all integration test files directly inside `crates/<crate>/tests/` must strictly start with `test_` (e.g. `test_<name>.rs`) per .agents/rules/unit-testing-policy.md.",
        violations.join("\n")
    );
}

#[test]
fn test_multi_module_crate_test_parity() {
    let repo_root = find_repo_root();
    let crates_dir = repo_root.join("crates");

    let required_multi_module_tests: &[(&str, &[&str])] = &[
        (
            "blitter",
            &[
                "test_blitter.rs",
                "test_line.rs",
                "test_minterm.rs",
                "test_phase.rs",
            ],
        ),
        (
            "disassembler",
            &[
                "test_align.rs",
                "test_alu.rs",
                "test_branch.rs",
                "test_data.rs",
                "test_ea.rs",
            ],
        ),
        ("floppy", &["test_floppy.rs", "test_mfm.rs"]),
        ("config", &["test_config.rs", "test_mutation.rs"]),
        (
            "physical_memory",
            &[
                "test_address_bus.rs",
                "test_map.rs",
                "test_physical_memory.rs",
                "test_presets.rs",
            ],
        ),
        (
            "paula",
            &["test_paula.rs", "test_paula_registers.rs", "test_serial.rs"],
        ),
    ];

    let mut violations = Vec::new();

    for &(crate_name, expected_files) in required_multi_module_tests {
        let tests_dir = crates_dir.join(crate_name).join("tests");
        for &expected_file in expected_files {
            let file_path = tests_dir.join(expected_file);
            if !file_path.is_file() {
                violations.push(format!(
                    "crates/{}/tests/{}: Missing 1:1 modular unit test file for submodule",
                    crate_name, expected_file
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Missing modular unit test files in multi-module crates:\n{}\n\
        Multi-module crates must maintain 1:1 test parity with dedicated unit test suites for each major algorithmic submodule.",
        violations.join("\n")
    );
}

#[test]
fn test_all_rules_audited_in_quality_harness() {
    let repo_root = find_repo_root();
    let rules_dir = repo_root.join(".agents").join("rules");

    // Authoritative registry of all 29 active rules in .agents/rules/
    let registered_rules: &[&str] = &[
        "amiga-rag.md",
        "asset-descriptions.md",
        "audio-transcription.md",
        "clean-break-refactoring.md",
        "diary-maintenance.md",
        "docs-maintenance.md",
        "egui-best-practices.md",
        "file-size-and-cohesion.md",
        "git-commits.md",
        "git-merge-commits.md",
        "graphify.md",
        "hardware-bus-topology.md",
        "information-hierarchy.md",
        "language-policy.md",
        "method-inlining.md",
        "model-reasoning-advisory.md",
        "no-external-paths.md",
        "opcode-naming.md",
        "parallel-execution.md",
        "performance-and-readability.md",
        "practitioner-voice-and-tone.md",
        "repro-first.md",
        "roadmap-maintenance.md",
        "rust-best-practices.md",
        "spec-compliance.md",
        "structural-root-cause.md",
        "unit-testing-policy.md",
        "vault-linking-and-graph-integrity.md",
        "workspace-structure-and-reexports.md",
    ];

    let mut on_disk_rules = Vec::new();
    if let Ok(entries) = fs::read_dir(&rules_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    on_disk_rules.push(name.to_string());
                }
            }
        }
    }
    on_disk_rules.sort();

    let mut violations = Vec::new();

    // Check for unregistered on-disk rules
    for rule in &on_disk_rules {
        if !registered_rules.contains(&rule.as_str()) {
            violations.push(format!(
                ".agents/rules/{}: Rule file is not registered in the Architecture & Quality Audit Registry.",
                rule
            ));
        }
    }

    // Check for phantom registered rules
    for &reg in registered_rules {
        if !on_disk_rules.iter().any(|r| r == reg) {
            violations.push(format!(
                ".agents/rules/{}: Registered rule is missing from on-disk .agents/rules/ directory.",
                reg
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture Rule Violation: Unaudited or phantom rules detected:\n{}\n\
        Every rule in .agents/rules/*.md must have verified audit coverage per AGENTS.md.",
        violations.join("\n")
    );
}
