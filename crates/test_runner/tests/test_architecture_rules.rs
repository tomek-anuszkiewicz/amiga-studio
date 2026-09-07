//! Automated Architecture & Engineering Rules Validation Tests
//!
//! Enforces guidelines from AGENTS.md:
//! 1. File size limit: <= 800 lines (with recognized exceptions).
//! 2. Zero runtime panics: no `.unwrap()` / `.expect()` in core emulation crates.
//! 3. Strict path privacy: zero hardcoded user/host paths.
//! 4. Strict macro prohibition: zero `macro_rules!` definitions in workspace crates.

use std::fs;
use std::path::{Path, PathBuf};

/// Recognized exceptions allowed to exceed the 800-line threshold
/// (e.g. compile-time static dispatch tables).
const LINE_COUNT_EXCEPTIONS: &[&str] = &[
    "dispatch_table.rs", // 65,536-entry static opcode jump table
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
        "Architecture Rule Violation: The following file(s) exceed the 800-line limit per AGENTS.md:\n{}",
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

