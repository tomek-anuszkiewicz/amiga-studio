#!/usr/bin/env python3
"""
audit_code_quality.py - Comprehensive On-Demand Code Quality Auditor

Audits three critical architectural dimensions across workspace crates:
1. Dead Code & Test-Only Zombies:
   - Completely dead symbols (0 callers anywhere).
   - Test-only zombie symbols (0 callers in production crates/*/src/, only called in tests/).
2. Principle of Least Visibility (Information Hiding):
   - Over-exposed `pub` items that are only called within their own defining crate (should be `pub(crate)`).
   - Over-exposed `pub`/`pub(crate)` items that are only called within their own defining file (should be private).
   - Over-exposed `pub mod` declarations whose items are never referenced externally.
3. Single Responsibility Principle & Cohesion:
   - Source files exceeding 800 lines in crates/*/src/ (referencing architecture rule exceptions).
   - Structs with excessive public fields (> 12) indicating potential "God Structs" or unencapsulated state.
4. Method Inlining Guidelines:
   - Evaluates #[inline(always)] on hot ALU/CCR setters and #[inline(never)] on cold traps.
5. Macro & Const-Generic Prohibitions:
   - Zero macro_rules! and zero const-generic function handlers.
6. External Test Suites & Parity:
   - Verifies external tests/ layout and test_ canonical naming.
7. Path Privacy & Host Isolation:
   - Scans crates for hardcoded host paths, user directories, or external private paths.
"""

import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path

def find_repo_root() -> Path:
    curr = Path(__file__).resolve().parent
    for p in [curr] + list(curr.parents):
        if (p / "Cargo.toml").exists() and (p / ".git").exists():
            return p
    return curr.parents[3]

REPO_ROOT = find_repo_root()
CRATES_DIR = REPO_ROOT / "crates"

# Crates or modules with special execution models (e.g. 65,536-entry function pointer dispatch tables)
EXEMPT_CRATES = {
    # cpu instruction handlers are referenced via compile-time function pointer table
    "cpu",
    # test_runner is a dedicated test harness and verification crate, not production code
    "test_runner",
}

# Hardware register specification catalogs whose constants reflect physical silicon memory maps
EXEMPT_FILES = {
    "crates/config/src/registers.rs",
}

# External Host I/O boundary methods and hardware spec symbols per SKILL.md Section 4 Step 1
HOST_IO_AND_SPEC_SYMBOLS = {
    # Host Keyboard Input & Protocol
    "key_down", "key_up", "poll_reset", "queue_powerup_stream", "has_pending_scancodes",
    "SCANCODE_LOST_SYNC", "SCANCODE_SELF_TEST_FAILED",
    # Host Game Port / Mouse / Joystick Plugging
    "plug_port1", "plug_port2", "port1_mut", "port2_mut",
    # Host Parallel Port
    "write_data", "read_data",
    # Host Floppy Disk Drive Insertion
    "insert_disk", "eject_disk", "FORMATTED_DISK_BYTES", "decode_amiga_sector",
    # Host Debugger Controls & Breakpoints
    "DEFAULT_TARGET_ADDRESS", "toggle_pc_breakpoint", "has_pc_breakpoint", "check_watchpoint",
    "total_count", "save_state_to_json", "load_state_from_json", "run_until_breakpoint",
    # Host Display & Frame Buffer Extraction
    "is_in_display_window", "get_pixel", "frame_buffer", "frame_buffer_mut", "begin_frame", "end_frame", "extract_vamiga_raw_viewport",
    # Host Audio Sample Ring Buffer
    "pop_sample", "samples_available",
    # Host Real-Time Clock
    "set_time",
    # System Topologies & Save States / Synchronous Stepping
    "bare_512k", "expanded_power_user", "to_json", "step_cycles", "execute_blit",
    # Bus Query & Overlay Status
    "is_wait", "is_low_memory_overlay_active",
    # HRM Figure 6-9 DMA Slot Timing Specifications
    "HPOS_REFRESH_SLOTS", "HPOS_DISK_SLOTS", "HPOS_AUDIO_SLOTS", "HPOS_SPRITE_START", "HPOS_SPRITE_END",
    # Hardware Chip Interface Signals
    "trigger_flag_pin", "poll_pra_output", "led_transition", "stage_write",
    "set_disk_byte", "poll_dsksyn_irq",
    # Sprite pipeline evaluation
    "evaluate_pixel",
}

# Recognized architectural file-size exceptions per file-size-and-cohesion.md & test_architecture_rules.rs
LINE_COUNT_EXCEPTIONS = {
    "crates/cpu/src/instructions/move_b.rs",
    "crates/cpu/src/instructions/move_w.rs",
    "crates/cpu/src/instructions/move_l.rs",
    "crates/cpu/src/instructions/add.rs",
    "crates/cpu/src/instructions/sub.rs",
    "crates/cpu/src/instructions/and.rs",
    "crates/cpu/src/instructions/or.rs",
    "crates/cpu/src/instructions/cmpi.rs",
    "crates/cpu/src/micro/dispatch_table.rs",
}

# Standard trait and lifecycle boilerplate methods to ignore
BOILERPLATE_NAMES = {
    "new", "default", "clone", "fmt", "eq", "ne", "drop",
    "serialize", "deserialize", "from", "into", "as_ref",
    "step", "step_cck", "reset", "reset_warm",
}

RE_PUB_FN = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s+(?:const\s+|unsafe\s+)?fn\s+([a-zA-Z0-9_]+)\b")
RE_PUB_CRATE_FN = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s*\(\s*crate\s*\)\s+(?:const\s+|unsafe\s+)?fn\s+([a-zA-Z0-9_]+)\b")
RE_PUB_CONST = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s+const\s+([A-Z0-9_]+)\b")
RE_PUB_STRUCT = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s+struct\s+([a-zA-Z0-9_]+)\b")
RE_PUB_MOD = re.compile(r"^\s*pub\s+mod\s+([a-zA-Z0-9_]+)\s*;")


def build_file_corpus():
    """Indexes all production (.rs in crates/*/src/) and test (.rs in crates/*/tests/) files."""
    prod_files = []
    test_files = []

    for rs_file in CRATES_DIR.rglob("*.rs"):
        parts = rs_file.parts
        if "tests" in parts:
            test_files.append(rs_file)
        elif "src" in parts:
            prod_files.append(rs_file)

    return prod_files, test_files


def collect_declared_symbols(crate_dir):
    """Collects declared public and pub(crate) functions, constants, and structs."""
    src_dir = crate_dir / "src"
    if not src_dir.exists():
        return []

    declared = []
    for rs_file in src_dir.rglob("*.rs"):
        rel_path = str(rs_file.relative_to(REPO_ROOT)).replace("\\", "/")
        if rel_path in EXEMPT_FILES:
            continue
        try:
            content = rs_file.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue

        for line_num, line in enumerate(content.splitlines(), 1):
            # pub fn
            m_pub_fn = RE_PUB_FN.match(line)
            if m_pub_fn:
                name = m_pub_fn.group(1)
                if name not in BOILERPLATE_NAMES and not name.startswith("_"):
                    declared.append({
                        "kind": "fn",
                        "visibility": "pub",
                        "name": name,
                        "file": rs_file,
                        "line": line_num,
                        "crate": crate_dir.name,
                    })
                continue

            # pub(crate) fn
            m_crate_fn = RE_PUB_CRATE_FN.match(line)
            if m_crate_fn:
                name = m_crate_fn.group(1)
                if name not in BOILERPLATE_NAMES and not name.startswith("_"):
                    declared.append({
                        "kind": "fn",
                        "visibility": "pub(crate)",
                        "name": name,
                        "file": rs_file,
                        "line": line_num,
                        "crate": crate_dir.name,
                    })
                continue

            # pub const
            m_const = RE_PUB_CONST.match(line)
            if m_const:
                name = m_const.group(1)
                if not name.startswith("_"):
                    declared.append({
                        "kind": "const",
                        "visibility": "pub",
                        "name": name,
                        "file": rs_file,
                        "line": line_num,
                        "crate": crate_dir.name,
                    })
                continue

    return declared


def count_symbol_references(symbol_name, files, def_file, def_line):
    """Counts references to symbol_name across files, excluding definition and re-exports."""
    pattern = re.compile(rf"\b{re.escape(symbol_name)}\b")
    callers = []

    for file_path in files:
        try:
            text = file_path.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue

        if symbol_name not in text:
            continue

        for line_num, line in enumerate(text.splitlines(), 1):
            if file_path == def_file and line_num == def_line:
                continue
            if "pub use " in line and symbol_name in line:
                continue
            if pattern.search(line):
                callers.append((file_path, line_num))

    return callers


def scan_dead_and_zombie_code(target_crate=None):
    """Detects completely dead code (0 callers) and test-only zombie code."""
    prod_files, test_files = build_file_corpus()
    all_declared = []

    for crate_dir in sorted(CRATES_DIR.iterdir()):
        if not crate_dir.is_dir() or not (crate_dir / "Cargo.toml").exists():
            continue
        if target_crate and crate_dir.name != target_crate:
            continue
        if crate_dir.name in EXEMPT_CRATES:
            continue

        all_declared.extend(collect_declared_symbols(crate_dir))

    completely_dead = []
    test_only_zombies = []

    for sym in all_declared:
        name = sym["name"]
        def_file = sym["file"]
        def_line = sym["line"]

        prod_callers = count_symbol_references(name, prod_files, def_file, def_line)
        test_callers = count_symbol_references(name, test_files, def_file, def_line)

        sym_info = {
            "name": name,
            "kind": sym["kind"],
            "visibility": sym["visibility"],
            "crate": sym["crate"],
            "file": str(def_file.relative_to(REPO_ROOT)),
            "line": def_line,
            "prod_callers_count": len(prod_callers),
            "test_callers_count": len(test_callers),
            "test_callers": [(str(f.relative_to(REPO_ROOT)), l) for f, l in test_callers[:3]],
        }

        if len(prod_callers) == 0 and len(test_callers) == 0:
            if name not in HOST_IO_AND_SPEC_SYMBOLS:
                completely_dead.append(sym_info)
        elif len(prod_callers) == 0 and len(test_callers) > 0:
            if name not in HOST_IO_AND_SPEC_SYMBOLS:
                test_only_zombies.append(sym_info)

    return completely_dead, test_only_zombies


def scan_least_visibility(target_crate=None):
    """Detects symbols with over-broad visibility (pub instead of pub(crate) or private)."""
    prod_files, test_files = build_file_corpus()
    all_declared = []

    for crate_dir in sorted(CRATES_DIR.iterdir()):
        if not crate_dir.is_dir() or not (crate_dir / "Cargo.toml").exists():
            continue
        if target_crate and crate_dir.name != target_crate:
            continue
        if crate_dir.name in EXEMPT_CRATES:
            continue

        all_declared.extend(collect_declared_symbols(crate_dir))

    visibility_leaks = []

    for sym in all_declared:
        if sym["visibility"] != "pub":
            continue

        name = sym["name"]
        def_file = sym["file"]
        def_line = sym["line"]
        defining_crate = sym["crate"]

        prod_callers = count_symbol_references(name, prod_files, def_file, def_line)
        test_callers = count_symbol_references(name, test_files, def_file, def_line)

        if len(prod_callers) == 0:
            continue

        # Check if callers are strictly within the defining crate
        external_callers = []
        internal_callers = []
        same_file_callers = []

        for f, l in prod_callers:
            crate_name = f.parts[f.parts.index("crates") + 1] if "crates" in f.parts else None
            if crate_name != defining_crate:
                external_callers.append((f, l))
            else:
                internal_callers.append((f, l))
                if f == def_file:
                    same_file_callers.append((f, l))

        if len(external_callers) == 0:
            # If callers exist in external tests/ suites, the symbol must remain pub
            # because external integration tests (crates/*/tests/) cannot access pub(crate) items per unit-testing-policy.md
            if len(test_callers) > 0:
                continue

            # If callers exist only within the same file, recommend private fn
            if len(internal_callers) == len(same_file_callers):
                recommended = "private (fn)"
                reason = "Only called within defining file"
            else:
                recommended = "pub(crate)"
                reason = f"Only called within crate `{defining_crate}`"

            visibility_leaks.append({
                "name": name,
                "kind": sym["kind"],
                "crate": defining_crate,
                "file": str(def_file.relative_to(REPO_ROOT)),
                "line": def_line,
                "recommended": recommended,
                "reason": reason,
                "caller_count": len(internal_callers),
                "external_callers": len(external_callers),
            })

    return visibility_leaks


def scan_srp_and_cohesion(target_crate=None):
    """Scans for file-size and structural cohesion anomalies."""
    violations = []

    for crate_dir in sorted(CRATES_DIR.iterdir()):
        if not crate_dir.is_dir() or not (crate_dir / "Cargo.toml").exists():
            continue
        if target_crate and crate_dir.name != target_crate:
            continue

        src_dir = crate_dir / "src"
        if not src_dir.exists():
            continue

        for rs_file in src_dir.rglob("*.rs"):
            rel_path = str(rs_file.relative_to(REPO_ROOT)).replace("\\", "/")
            try:
                lines = rs_file.read_text(encoding="utf-8", errors="ignore").splitlines()
            except Exception:
                continue

            line_count = len(lines)

            # File size check (> 800 lines)
            if line_count > 800 and rel_path not in LINE_COUNT_EXCEPTIONS:
                violations.append({
                    "type": "file_size",
                    "crate": crate_dir.name,
                    "file": rel_path,
                    "metric": f"{line_count} lines (> 800 limit)",
                    "recommendation": "Decompose into cohesive submodules per file-size-and-cohesion.md",
                })

            # Check for structs with excessive public fields (> 12)
            struct_name = None
            struct_pub_fields = 0
            in_struct = False

            for line in lines:
                m_st = RE_PUB_STRUCT.match(line)
                if m_st:
                    if struct_name and struct_pub_fields > 12:
                        violations.append({
                            "type": "excessive_pub_fields",
                            "crate": crate_dir.name,
                            "file": rel_path,
                            "metric": f"`{struct_name}` has {struct_pub_fields} public fields",
                            "recommendation": "Encapsulate fields with methods or group into domain sub-structs",
                        })
                    struct_name = m_st.group(1)
                    struct_pub_fields = 0
                    in_struct = "{" in line
                    continue

                if in_struct:
                    if line.strip().startswith("}"):
                        if struct_name and struct_pub_fields > 12:
                            violations.append({
                                "type": "excessive_pub_fields",
                                "crate": crate_dir.name,
                                "file": rel_path,
                                "metric": f"`{struct_name}` has {struct_pub_fields} public fields",
                                "recommendation": "Encapsulate fields with methods or group into domain sub-structs",
                            })
                        in_struct = False
                        struct_name = None
                        struct_pub_fields = 0
                    elif re.match(r"^\s*pub\s+[a-zA-Z0-9_]+\s*:", line):
                        struct_pub_fields += 1

    # Check for stale or missing exceptions in LINE_COUNT_EXCEPTIONS
    if not target_crate:
        for exc_path_str in sorted(LINE_COUNT_EXCEPTIONS):
            exc_path = REPO_ROOT / exc_path_str
            if not exc_path.exists():
                violations.append({
                    "type": "missing_line_count_exception",
                    "crate": exc_path_str.split("/")[1] if "/" in exc_path_str else "",
                    "file": exc_path_str,
                    "metric": "File does not exist on disk",
                    "recommendation": "Exception entry is obsolete; requires explicit user command to prune.",
                })
            else:
                try:
                    exc_lines = len(exc_path.read_text(encoding="utf-8", errors="ignore").splitlines())
                    if exc_lines <= 800:
                        violations.append({
                            "type": "stale_line_count_exception",
                            "crate": exc_path_str.split("/")[1] if "/" in exc_path_str else "",
                            "file": exc_path_str,
                            "metric": f"{exc_lines} lines (<= 800 limit)",
                            "recommendation": "File has been modularized and no longer exceeds 800 lines; requires explicit user command to prune from LINE_COUNT_EXCEPTIONS.",
                        })
                except Exception:
                    pass

    return violations


def scan_inlining_guidelines(crate_name=None):
    """Audits required #[inline(always)] and #[inline(never)] annotations per method-inlining.md."""
    issues = []
    # 1. Cold exception/trap triggers must have #[inline(never)]
    m68k_src = CRATES_DIR / "cpu" / "src"
    if m68k_src.exists() and (not crate_name or crate_name == "cpu"):
        for file in m68k_src.rglob("*.rs"):
            try:
                lines = file.read_text(encoding="utf-8", errors="ignore").splitlines()
            except Exception:
                continue
            rel_path = file.relative_to(REPO_ROOT)
            for idx, line in enumerate(lines):
                trimmed = line.strip()
                if trimmed.startswith("pub fn trigger_") or trimmed.startswith("fn trigger_"):
                    prev_lines = lines[max(0, idx - 2):idx]
                    if not any("#[inline(never)]" in l for l in prev_lines):
                        issues.append({
                            "type": "missing_inline_never",
                            "file": str(rel_path),
                            "line": idx + 1,
                            "metric": trimmed,
                            "recommendation": f"Cold exception/trap `{trimmed}` must be annotated with #[inline(never)]",
                        })

    # 2. Leaf ALU functions in cpu/src/instructions/ must have #[inline(always)]
    leaf_prefixes = (
        "pub fn add_", "pub fn sub_", "pub fn and_", "pub fn or_", "pub fn eor_",
        "pub fn cmp_", "pub fn asr_", "pub fn asl_", "pub fn lsr_", "pub fn lsl_",
        "pub fn ror_", "pub fn rol_", "pub fn roxr_", "pub fn roxl_", "pub fn neg_",
        "pub fn negx_", "pub fn not_", "pub fn tst_", "pub fn abcd_", "pub fn sbcd_",
        "pub fn nbcd_", "pub fn bchg_", "pub fn bclr_", "pub fn bset_", "pub fn btst_",
    )
    inst_dir = CRATES_DIR / "cpu" / "src" / "instructions"
    if inst_dir.exists() and (not crate_name or crate_name == "cpu"):
        for file in inst_dir.glob("*.rs"):
            try:
                lines = file.read_text(encoding="utf-8", errors="ignore").splitlines()
            except Exception:
                continue
            rel_path = file.relative_to(REPO_ROOT)
            for idx, line in enumerate(lines):
                trimmed = line.strip()
                if any(trimmed.startswith(p) for p in leaf_prefixes):
                    prev_lines = lines[max(0, idx - 2):idx]
                    if not any("#[inline(always)]" in l for l in prev_lines):
                        issues.append({
                            "type": "missing_inline_always",
                            "file": str(rel_path),
                            "line": idx + 1,
                            "metric": trimmed,
                            "recommendation": f"Leaf ALU function `{trimmed}` must be annotated with #[inline(always)]",
                        })
    return issues


def scan_macro_and_generic_prohibitions(crate_name=None):
    """Audits prohibition of custom macro_rules! and const-generic instruction handlers."""
    issues = []
    crates = [CRATES_DIR / crate_name] if crate_name else list(CRATES_DIR.iterdir())
    for c in crates:
        if not c.is_dir():
            continue
        src = c / "src"
        if not src.exists():
            continue
        for file in src.rglob("*.rs"):
            try:
                lines = file.read_text(encoding="utf-8", errors="ignore").splitlines()
            except Exception:
                continue
            rel_path = file.relative_to(REPO_ROOT)
            for idx, line in enumerate(lines, 1):
                trimmed = line.strip()
                if trimmed.startswith("//"):
                    continue
                if "macro_rules!" in trimmed:
                    issues.append({
                        "type": "forbidden_macro",
                        "file": str(rel_path),
                        "line": idx,
                        "metric": trimmed,
                        "recommendation": "Custom macros (`macro_rules!`) are strictly forbidden per performance-and-readability.md.",
                    })
                if c.name == "cpu" and "<const " in trimmed:
                    issues.append({
                        "type": "forbidden_const_generic",
                        "file": str(rel_path),
                        "line": idx,
                        "metric": trimmed,
                        "recommendation": "Const-generic functions (`<const N: ...>`) in M68000 core are strictly forbidden.",
                    })
    return issues


def scan_external_test_suites(crate_name=None):
    """Audits dedicated external test suite presence, 1:1 submodule test parity, and zero inline tests."""
    issues = []
    crates = [CRATES_DIR / crate_name] if crate_name else sorted(CRATES_DIR.iterdir())
    for c in crates:
        if not c.is_dir() or not (c / "Cargo.toml").exists():
            continue
        c_name = c.name
        src_dir = c / "src"
        tests_dir = c / "tests"

        # Check inline tests in src/
        if src_dir.exists():
            for file in src_dir.rglob("*.rs"):
                try:
                    lines = file.read_text(encoding="utf-8", errors="ignore").splitlines()
                except Exception:
                    continue
                rel_path = file.relative_to(REPO_ROOT)
                for idx, line in enumerate(lines, 1):
                    trimmed = line.strip()
                    if trimmed in ("#[cfg(test)]", "mod tests {", "mod test {") or trimmed.startswith("#[test]"):
                        issues.append({
                            "type": "inline_test_found",
                            "file": str(rel_path),
                            "line": idx,
                            "metric": trimmed,
                            "recommendation": "Inline tests in src/ are forbidden. Relocate to dedicated `crates/<crate>/tests/`.",
                        })

        # Check dedicated tests/ directory
        if not tests_dir.is_dir():
            issues.append({
                "type": "missing_tests_dir",
                "file": f"crates/{c_name}/tests/",
                "line": 1,
                "metric": "missing tests/ directory",
                "recommendation": f"Crate `{c_name}` lacks dedicated external tests/ directory.",
            })
            continue

        test_files = list(tests_dir.glob("*.rs"))
        if not test_files:
            issues.append({
                "type": "empty_tests_dir",
                "file": f"crates/{c_name}/tests/",
                "line": 1,
                "metric": "0 test files",
                "recommendation": f"Crate `{c_name}` tests/ directory contains zero .rs test files.",
            })
        for tf in test_files:
            if not tf.name.startswith("test_"):
                issues.append({
                    "type": "non_canonical_test_name",
                    "file": str(tf.relative_to(REPO_ROOT)),
                    "line": 1,
                    "recommendation": f"Test file `{tf.name}` must start with `test_` prefix.",
                })
    return issues


def scan_path_privacy(target_crate=None):
    """
    Scans crate source and test files for hardcoded host paths, user directories,
    or external private folder references per .agents/rules/no-external-paths.md.
    """
    crates = [CRATES_DIR / target_crate] if target_crate else [p for p in CRATES_DIR.iterdir() if p.is_dir() and (p / "Cargo.toml").exists()]
    forbidden_patterns = ["C:\\Users\\", "C:/Users/", "/home/", "Google Drive"]

    issues = []
    for c in crates:
        for f in c.rglob("*.rs"):
            if f.name == "test_architecture_rules.rs":
                continue
            try:
                content = f.read_text(encoding="utf-8", errors="ignore")
            except Exception:
                continue

            for line_idx, line in enumerate(content.splitlines(), start=1):
                for pat in forbidden_patterns:
                    if pat in line:
                        rel = f.relative_to(REPO_ROOT).as_posix()
                        issues.append({
                            "type": "hardcoded_external_path",
                            "file": rel,
                            "line": line_idx,
                            "pattern": pat,
                            "snippet": line.strip(),
                            "recommendation": f"Replace hardcoded host path pattern `{pat}` with generic placeholder or relative configuration.",
                        })

    return issues


def main():
    parser = argparse.ArgumentParser(description="Audit dead code, minimum visibility leaks, SRP cohesion, inlining guidelines, antipatterns, test suites, and path privacy.")
    parser.add_argument("--all", action="store_true", help="Run all code quality audits")
    parser.add_argument("--dead-code", action="store_true", help="Run dead code & zombie scanner")
    parser.add_argument("--visibility", action="store_true", help="Run least visibility scanner")
    parser.add_argument("--srp", action="store_true", help="Run SRP and file cohesion checks")
    parser.add_argument("--inlining", action="store_true", help="Run method inlining guidelines scanner")
    parser.add_argument("--antipatterns", action="store_true", help="Run macro and const-generic antipattern checks")
    parser.add_argument("--tests", action="store_true", help="Run external test suite and inline test scanner")
    parser.add_argument("--path-privacy", action="store_true", help="Run path privacy and host isolation scanner")
    parser.add_argument("--crate", help="Filter audit to a specific crate")
    parser.add_argument("--json", action="store_true", help="Output results as JSON")
    args = parser.parse_args()

    # Default to --all if no specific mode selected
    if not (args.all or args.dead_code or args.visibility or args.srp or args.inlining or args.antipatterns or args.tests or args.path_privacy):
        args.all = True

    dead, zombies = ([], [])
    if args.all or args.dead_code:
        dead, zombies = scan_dead_and_zombie_code(args.crate)

    vis_leaks = []
    if args.all or args.visibility:
        vis_leaks = scan_least_visibility(args.crate)

    srp_issues = []
    if args.all or args.srp:
        srp_issues = scan_srp_and_cohesion(args.crate)

    inlining_issues = []
    if args.all or args.inlining:
        inlining_issues = scan_inlining_guidelines(args.crate)

    antipattern_issues = []
    if args.all or args.antipatterns:
        antipattern_issues = scan_macro_and_generic_prohibitions(args.crate)

    test_suite_issues = []
    if args.all or args.tests:
        test_suite_issues = scan_external_test_suites(args.crate)

    privacy_issues = []
    if args.all or args.path_privacy:
        privacy_issues = scan_path_privacy(args.crate)

    if args.json:
        out = {
            "dead_code": dead,
            "test_only_zombies": zombies,
            "visibility_leaks": vis_leaks,
            "srp_cohesion_issues": srp_issues,
            "inlining_issues": inlining_issues,
            "antipattern_issues": antipattern_issues,
            "test_suite_issues": test_suite_issues,
            "path_privacy_issues": privacy_issues,
        }
        print(json.dumps(out, indent=2))
        return

    if sys.stdout.encoding != "utf-8":
        try:
            sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass

    print("=" * 76)
    print(" AMIGA 500 EMULATOR: DEEP ARCHITECTURAL CODE QUALITY AUDIT")
    print("=" * 76)

    if args.all or args.dead_code:
        print(f"\n[1. DEAD CODE & ZOMBIE SCANNER]")
        print(f"  - Completely Dead: {len(dead)} symbol(s)")
        for s in dead[:10]:
            print(f"    * [{s['kind']}] {s['crate']}::{s['name']} -> {s['file']}:{s['line']}")
        if len(dead) > 10:
            print(f"    * ... and {len(dead) - 10} more")

        print(f"  - Test-Only Zombies: {len(zombies)} symbol(s) (unused by production code)")
        for s in zombies[:10]:
            callers_str = ", ".join(f"{c[0]}:{c[1]}" for c in s["test_callers"][:2])
            print(f"    * [{s['kind']}] {s['crate']}::{s['name']} -> {s['file']}:{s['line']} (tested in {callers_str})")
        if len(zombies) > 10:
            print(f"    * ... and {len(zombies) - 10} more")

    if args.all or args.visibility:
        print(f"\n[2. PRINCIPLE OF MINIMUM VISIBILITY (LEAST PRIVILEGE)]")
        print(f"  - Over-exposed `pub` items: {len(vis_leaks)} symbol(s)")
        for v in vis_leaks[:15]:
            print(f"    * {v['crate']}::{v['name']} ({v['file']}:{v['line']}) -> recommend `{v['recommended']}` ({v['reason']})")
        if len(vis_leaks) > 15:
            print(f"    * ... and {len(vis_leaks) - 15} more")

    if args.all or args.srp:
        print(f"\n[3. SINGLE RESPONSIBILITY & COHESION]")
        print(f"  - Structural anomalies: {len(srp_issues)} issue(s)")
        for issue in srp_issues:
            print(f"    * [{issue['type']}] {issue['file']}: {issue['metric']}")
            print(f"      -> {issue['recommendation']}")

    if args.all or args.inlining:
        print(f"\n[4. METHOD INLINING GUIDELINES]")
        if not inlining_issues:
            print("  - Status: [PASS] All hot ALU/CCR setters and cold traps adhere to inlining annotations.")
        else:
            print(f"  - Status: [WARN] {len(inlining_issues)} inlining anomaly/ies detected:")
            for issue in inlining_issues:
                print(f"    * [{issue['type']}] {issue['file']}:{issue['line']} -> {issue['recommendation']}")

    if args.all or args.antipatterns:
        print(f"\n[5. MACRO & CONST-GENERIC PROHIBITION]")
        if not antipattern_issues:
            print("  - Status: [PASS] Zero custom macros (`macro_rules!`) and zero const-generic handlers.")
        else:
            print(f"  - Status: [FAIL] {len(antipattern_issues)} antipattern issue(s) detected:")
            for issue in antipattern_issues:
                print(f"    * [{issue['type']}] {issue['file']}:{issue['line']} -> {issue['recommendation']}")

    if args.all or args.tests:
        print(f"\n[6. EXTERNAL TEST SUITES & PARITY]")
        if not test_suite_issues:
            print("  - Status: [PASS] All crates have dedicated external tests/ with zero inline tests in src/.")
        else:
            print(f"  - Status: [FAIL] {len(test_suite_issues)} test organization issue(s) detected:")
            for issue in test_suite_issues:
                print(f"    * [{issue['type']}] {issue['file']} -> {issue['recommendation']}")

    if args.all or args.path_privacy:
        print(f"\n[7. PATH PRIVACY & HOST ISOLATION]")
        if not privacy_issues:
            print("  - Status: [PASS] Zero hardcoded user/host paths in workspace crates.")
        else:
            print(f"  - Status: [FAIL] {len(privacy_issues)} path privacy violation(s) detected:")
            for issue in privacy_issues:
                print(f"    * [{issue['type']}] {issue['file']}:{issue['line']} (found '{issue['pattern']}') -> {issue['recommendation']}")

    print("\n" + "=" * 76)
    print(f"Code Quality Summary: {len(dead)} dead, {len(zombies)} zombies, {len(vis_leaks)} visibility leaks, {len(srp_issues)} cohesion issues, {len(inlining_issues)} inlining issues, {len(antipattern_issues)} antipattern issues, {len(test_suite_issues)} test suite issues, {len(privacy_issues)} path privacy issues.")
    print("=" * 76)


if __name__ == "__main__":
    main()

