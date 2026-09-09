//! Automated Architecture & Engineering Rules Validation Tests
//!
//! Enforces guidelines from AGENTS.md:
//! 1. Rust source file size limit: <= 800 lines in crates/*/src/ (with recognized exceptions).
//!    Documentation (.md) has no line count limits.
//! 2. Zero runtime panics: no `.unwrap()` / `.expect()` in core emulation crates.
//! 3. Strict path privacy: zero hardcoded user/host paths.
//! 4. Strict macro prohibition: zero `macro_rules!` definitions in workspace crates.

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
    "blep_tables.rs",
];

/// Core emulation crates where `.unwrap()` and `.expect()` are strictly forbidden in runtime code.
const CORE_EMULATION_CRATES: &[&str] = &["m68000", "memory_bus", "config", "rtc", "debugger"];

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
