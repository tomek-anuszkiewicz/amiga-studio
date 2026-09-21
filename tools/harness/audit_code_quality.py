#!/usr/bin/env python3
"""
audit_code_quality.py - Comprehensive On-Demand Code Quality Auditor

Audits four architectural dimensions across workspace crates:
1. Dead Code & Test-Only Zombies:
   - Completely dead symbols (0 callers anywhere).
   - Test-only zombie symbols (0 callers in production crates/*/src/, only called in tests/).
2. Principle of Least Visibility (Information Hiding):
   - Over-exposed `pub` items that are only called within their own defining crate (should be `pub(crate)`).
   - Over-exposed `pub`/`pub(crate)` items that are only called within their own defining file (should be private).
   - Over-exposed `pub mod` declarations whose items are never referenced externally.
3. Condition Soup & Self-Documenting Boolean Logic:
   - Complex compound boolean conditions needing named explaining variables or domain predicate methods.
4. Method Naming & Accessor Conventions:
   - Evaluates standard getters matching field name without `get_` prefix.
   - Evaluates boolean getters starting with `is_` (or `has_`/`can_`) and zero duplicate prefixes.
   - Evaluates setters starting with `set_<field>`.
   - Evaluates collection getters returning borrowed slices rather than concrete container references (&Vec<T>).

Note: Inlining rules, macro/const-generic prohibitions, external test directory layouts, path privacy,
and 800-line file size limits are verified directly via `cargo test -p test_runner --test test_architecture_rules`
and `cargo clippy --workspace --all-targets`.
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

# Module-level corpus cache: avoids re-scanning the repository on every pillar call.
# Populated once on first call to get_file_corpus_cached().
_CORPUS_CACHE: dict = {}

# Module-level inverted index cache: built once across all files in a single pass.
# Maps identifier -> [(file_path, line_num, is_reexport)] for prod and test corpora.
# Turns symbol reference counting from O(symbols x files) -> O(files + symbols).
_INVERTED_INDEX_CACHE: dict = {}

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

# Standard trait and lifecycle boilerplate methods to ignore
BOILERPLATE_NAMES = {
    "new", "default", "clone", "fmt", "eq", "ne", "drop",
    "serialize", "deserialize", "from", "into", "as_ref",
    "step", "step_cck", "reset", "reset_warm",
}

RE_PUB_FN = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s+(?:const\s+|unsafe\s+)?fn\s+([a-zA-Z0-9_]+)\b")
RE_PUB_CRATE_FN = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s*\(\s*crate\s*\)\s+(?:const\s+|unsafe\s+)?fn\s+([a-zA-Z0-9_]+)\b")
RE_PUB_CONST = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s+const\s+([A-Z0-9_]+)\b")
RE_PUB_MOD = re.compile(r"^\s*pub\s+mod\s+([a-zA-Z0-9_]+)\s*;")
# Matches all Rust identifiers (min 2 chars to skip noise like single-letter loop vars).
RE_IDENTIFIER = re.compile(r"\b([a-zA-Z_][a-zA-Z0-9_]+)\b")


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


def get_file_corpus_cached():
    """Returns (prod_files, test_files) from cache; builds once on first call."""
    if not _CORPUS_CACHE:
        prod_files, test_files = build_file_corpus()
        _CORPUS_CACHE["prod"] = prod_files
        _CORPUS_CACHE["test"] = test_files
    return _CORPUS_CACHE["prod"], _CORPUS_CACHE["test"]


def _build_index_for_corpus(file_list: list) -> dict:
    """Scans files once, returning identifier -> [(file_path, line_num, is_reexport)]."""
    index: dict = {}
    for file_path in file_list:
        try:
            text = file_path.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue
        for line_num, line in enumerate(text.splitlines(), 1):
            is_reexport = "pub use " in line
            for m in RE_IDENTIFIER.finditer(line):
                token = m.group(1)
                if token not in index:
                    index[token] = []
                index[token].append((file_path, line_num, is_reexport))
    return index


def build_inverted_index_cached(prod_files: list, test_files: list) -> tuple:
    """Builds prod/test inverted indexes in a single file-system pass; cached for the process lifetime."""
    if not _INVERTED_INDEX_CACHE:
        _INVERTED_INDEX_CACHE["prod"] = _build_index_for_corpus(prod_files)
        _INVERTED_INDEX_CACHE["test"] = _build_index_for_corpus(test_files)
    return _INVERTED_INDEX_CACHE["prod"], _INVERTED_INDEX_CACHE["test"]


def count_symbol_references_cached(symbol_name, prod_files, test_files, def_file, def_line):
    """O(1) symbol lookup via inverted index; filters definition line and re-export lines."""
    prod_idx, test_idx = build_inverted_index_cached(prod_files, test_files)

    def filter_refs(raw: list) -> list:
        result = []
        for file_path, line_num, is_reexport in raw:
            if file_path == def_file and line_num == def_line:
                continue
            if is_reexport:
                continue
            result.append((file_path, line_num))
        return result

    return (
        filter_refs(prod_idx.get(symbol_name, [])),
        filter_refs(test_idx.get(symbol_name, [])),
    )


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



def scan_dead_and_zombie_code(target_crate=None):
    """Detects completely dead code (0 callers) and test-only zombie code."""
    prod_files, test_files = get_file_corpus_cached()
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

        prod_callers, test_callers = count_symbol_references_cached(
            name, prod_files, test_files, def_file, def_line
        )

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
    prod_files, test_files = get_file_corpus_cached()
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

        prod_callers, test_callers = count_symbol_references_cached(
            name, prod_files, test_files, def_file, def_line
        )

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


def scan_condition_soup(target_crate=None):
    """
    Scans crate source files for complex, multi-clause compound boolean conditionals
    (e.g. mixed nested &&/|| with parentheses or >= 3 boolean operators) that lack
    explaining variables per .agents/rules/performance-and-readability.md.
    """
    crates = (
        [CRATES_DIR / target_crate]
        if target_crate
        else [p for p in CRATES_DIR.iterdir() if p.is_dir() and (p / "Cargo.toml").exists()]
    )
    issues = []

    exempt_files = {
        "crates/cpu/src/micro/dispatch_table.rs",
    }

    for c in crates:
        src_dir = c / "src"
        if not src_dir.exists():
            continue

        for f in src_dir.rglob("*.rs"):
            rel = f.relative_to(REPO_ROOT).as_posix()
            if rel in exempt_files:
                continue

            try:
                content = f.read_text(encoding="utf-8", errors="ignore")
            except Exception:
                continue

            # Strip comments to avoid false positives on commented logic
            clean_lines = []
            in_block_comment = False
            for line in content.splitlines():
                if in_block_comment:
                    if "*/" in line:
                        in_block_comment = False
                        line = line.split("*/", 1)[1]
                    else:
                        clean_lines.append("")
                        continue
                if "/*" in line and "*/" not in line:
                    in_block_comment = True
                    line = line.split("/*", 1)[0]
                elif "//" in line:
                    line = line.split("//", 1)[0]
                clean_lines.append(line)

            clean_text = "\n".join(clean_lines)

            for m in re.finditer(r'\b(if|while)\s+([^\{]+)\{', clean_text):
                cond = m.group(2).strip()
                if cond.startswith("let ") or "let " in cond:
                    continue

                and_count = cond.count("&&")
                or_count = cond.count("||")
                has_mixed_nesting = "(" in cond and and_count >= 1 and or_count >= 1

                if and_count + or_count >= 3 or has_mixed_nesting:
                    line_idx = content[: m.start()].count("\n") + 1
                    first_line = cond.split("\n")[0].strip()
                    issues.append({
                        "type": "compound_condition_soup",
                        "file": rel,
                        "line": line_idx,
                        "keyword": m.group(1),
                        "snippet": first_line[:80],
                        "recommendation": "Decompose compound boolean condition into named explaining variables or domain predicate methods, preserving short-circuit evaluation without eager speculative computation.",
                    })

    return issues


def scan_accessor_conventions(target_crate=None):
    """
    Scans crate source files for violations of method naming and accessor conventions
    per .agents/rules/rust-best-practices.md:
    1. Standard getters must match field name without `get_` prefix.
    2. Boolean getters must start with `is_` (or retain `has_`/`can_`) with zero duplicate prefixes.
    3. Setters must start with `set_<field>`.
    """
    crates = (
        [CRATES_DIR / target_crate]
        if target_crate
        else [p for p in CRATES_DIR.iterdir() if p.is_dir() and (p / "Cargo.toml").exists()]
    )
    issues = []

    re_struct_start = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?struct\s+([a-zA-Z0-9_]+)\b[^{;]*\{")
    re_bool_field = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?([a-zA-Z0-9_]+)\s*:\s*bool\b")
    re_impl_start = re.compile(r"^\s*impl(?:\s*<[^>]+>)?\s+([a-zA-Z0-9_]+)\b")
    re_fn_start = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?(?:const\s+|unsafe\s+)?fn\s+([a-zA-Z0-9_]+)\s*\(")
    re_fn_sig = re.compile(
        r"^\s*(?:pub(?:\([^)]+\))?\s+)?(?:const\s+|unsafe\s+)?fn\s+([a-zA-Z0-9_]+)\s*\((.*?)\)(?:\s*->\s*([^{;]+))?"
    )

    for c in crates:
        src_dir = c / "src"
        if not src_dir.exists():
            continue

        for f in src_dir.rglob("*.rs"):
            rel = f.relative_to(REPO_ROOT).as_posix()
            try:
                content = f.read_text(encoding="utf-8", errors="ignore")
            except Exception:
                continue

            lines = content.splitlines()

            # Pass 1: collect structs and their boolean fields
            structs = {}
            curr_struct = None
            in_struct = False
            for line in lines:
                if not in_struct:
                    m_st = re_struct_start.match(line)
                    if m_st:
                        curr_struct = m_st.group(1)
                        structs[curr_struct] = []
                        in_struct = True
                else:
                    if line.strip().startswith("}"):
                        in_struct = False
                        curr_struct = None
                    else:
                        m_bf = re_bool_field.match(line)
                        if m_bf and curr_struct:
                            structs[curr_struct].append(m_bf.group(1))

            # Pass 2: parse methods inside impl blocks (with multi-line support)
            curr_impl = None
            in_sig = False
            sig_buffer = []
            sig_start_line = 0

            for line_idx, line in enumerate(lines, start=1):
                trimmed = line.strip()
                if trimmed.startswith("//"):
                    continue

                m_impl = re_impl_start.match(line)
                if m_impl:
                    curr_impl = m_impl.group(1)
                elif trimmed.startswith("}") and curr_impl and not in_sig:
                    curr_impl = None

                if not in_sig:
                    if re_fn_start.match(line):
                        if ")" in line:
                            # Single-line signature
                            full_sig = line
                            start_line = line_idx
                            m_fn = re_fn_sig.match(full_sig)
                        else:
                            in_sig = True
                            sig_buffer = [line]
                            sig_start_line = line_idx
                            continue
                    else:
                        continue
                else:
                    sig_buffer.append(line)
                    if ")" in line:
                        in_sig = False
                        full_sig = " ".join(s.strip() for s in sig_buffer)
                        start_line = sig_start_line
                        sig_buffer = []
                        m_fn = re_fn_sig.match(full_sig)
                    else:
                        continue

                if not m_fn:
                    continue

                fn_name = m_fn.group(1)
                params = m_fn.group(2).strip()
                ret_type = (m_fn.group(3) or "").strip()

                is_method = params.startswith("&self") or params.startswith("&mut self")
                if not is_method:
                    continue

                params_without_self = re.sub(r"^&(?:mut\s+)?self\s*,?\s*", "", params).strip()
                has_extra_params = len(params_without_self) > 0

                # 1. Standard getter with forbidden get_ prefix (0 additional arguments)
                if fn_name.startswith("get_") and not has_extra_params:
                    issues.append({
                        "type": "forbidden_get_prefix",
                        "crate": c.name,
                        "file": rel,
                        "line": start_line,
                        "metric": fn_name,
                        "recommendation": f"Standard getter `{fn_name}` must not use `get_` prefix per rust-best-practices.md. Use `<field>(&self)` (or `is_<field>(&self)` for booleans).",
                    })

                # 2. Duplicate prefixes
                for dup_prefix in ("is_is_", "has_has_", "can_can_", "set_set_"):
                    if fn_name.startswith(dup_prefix):
                        issues.append({
                            "type": "duplicate_accessor_prefix",
                            "crate": c.name,
                            "file": rel,
                            "line": start_line,
                            "metric": fn_name,
                            "recommendation": f"Method `{fn_name}` has duplicate prefix `{dup_prefix}` per rust-best-practices.md.",
                        })

                # 3. Boolean getter missing is_/has_/can_ prefix
                if ret_type == "bool" and not has_extra_params and curr_impl and curr_impl in structs:
                    known_bool_fields = structs[curr_impl]
                    if fn_name in known_bool_fields and not fn_name.startswith(("is_", "has_", "can_")):
                        issues.append({
                            "type": "missing_boolean_prefix",
                            "crate": c.name,
                            "file": rel,
                            "line": start_line,
                            "metric": fn_name,
                            "recommendation": f"Boolean getter `{fn_name}` for field in `{curr_impl}` must start with `is_` (e.g. `is_{fn_name}`) or retain `has_`/`can_` per rust-best-practices.md.",
                        })

                # 4. Collection getter returning concrete container (&Vec or &mut Vec) instead of borrowed slice
                if (ret_type.startswith("&Vec<") or ret_type.startswith("&mut Vec<")) and not has_extra_params:
                    inner_m = re.match(r"^&(?:mut\s+)?Vec<\s*(.+)\s*>$", ret_type)
                    slice_type = f"&[{inner_m.group(1)}]" if inner_m else "&[T]"
                    if ret_type.startswith("&mut"):
                        slice_type = f"&mut [{inner_m.group(1)}]" if inner_m else "&mut [T]"
                    issues.append({
                        "type": "concrete_container_getter",
                        "crate": c.name,
                        "file": rel,
                        "line": start_line,
                        "metric": f"{fn_name}() -> {ret_type}",
                        "recommendation": f"Collection getter `{fn_name}` exposes internal `{ret_type}`. Return borrowed slice `{slice_type}` per rust-best-practices.md.",
                    })

    return issues


def main():
    parser = argparse.ArgumentParser(
        description="Audit dead code, minimum visibility leaks, boolean conditions, and method naming & accessor conventions."
    )
    parser.add_argument("--all", action="store_true", help="Run all code quality audits")
    parser.add_argument("--per-commit", action="store_true", help="Run per-commit audits (Pillars 1 & 2: dead code & least visibility)")
    parser.add_argument("--milestone", action="store_true", help="Run milestone audits (Pillars 3 & 4: condition soup & accessors)")
    parser.add_argument("--dead-code", action="store_true", help="Run dead code & zombie scanner")
    parser.add_argument("--visibility", action="store_true", help="Run least visibility scanner")
    parser.add_argument("--conditions", action="store_true", help="Run condition soup & boolean clarity scanner")
    parser.add_argument("--accessors", action="store_true", help="Run method naming and accessor convention checks")
    parser.add_argument("--strict", action="store_true", help="Exit with non-zero exit code if issues are found")
    parser.add_argument("--crate", help="Filter audit to a specific crate")
    parser.add_argument("--json", action="store_true", help="Output results as JSON")
    args = parser.parse_args()

    if args.per_commit:
        args.dead_code = True
        args.visibility = True
    if args.milestone:
        args.conditions = True
        args.accessors = True

    # Default to --all if no specific mode selected
    if not (
        args.all
        or args.per_commit
        or args.milestone
        or args.dead_code
        or args.visibility
        or args.conditions
        or args.accessors
    ):
        args.all = True

    dead, zombies = ([], [])
    if args.all or args.dead_code:
        dead, zombies = scan_dead_and_zombie_code(args.crate)

    vis_leaks = []
    if args.all or args.visibility:
        vis_leaks = scan_least_visibility(args.crate)

    condition_issues = []
    if args.all or args.conditions:
        condition_issues = scan_condition_soup(args.crate)

    accessor_issues = []
    if args.all or args.accessors:
        accessor_issues = scan_accessor_conventions(args.crate)

    if args.json:
        out = {
            "dead_code": dead,
            "test_only_zombies": zombies,
            "visibility_leaks": vis_leaks,
            "condition_issues": condition_issues,
            "accessor_issues": accessor_issues,
        }
        print(json.dumps(out, indent=2))
        return

    if sys.stdout.encoding != "utf-8":
        try:
            sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass

    print("=" * 76)
    print(" AMIGA 500 EMULATOR: ARCHITECTURAL CODE QUALITY AUDIT")
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

    if args.all or args.conditions:
        print(f"\n[3. CONDITION SOUP & SELF-DOCUMENTING BOOLEAN LOGIC]")
        if not condition_issues:
            print("  - Status: [PASS] All conditionals use clean explaining variables and domain predicates.")
        else:
            print(f"  - Status: [WARN] {len(condition_issues)} compound condition(s) detected without explaining variables:")
            for issue in condition_issues[:15]:
                print(f"    * [{issue['type']}] {issue['file']}:{issue['line']} (`{issue['keyword']}`) -> {issue['snippet']}")
                print(f"      -> {issue['recommendation']}")
            if len(condition_issues) > 15:
                print(f"    * ... and {len(condition_issues) - 15} more")

    if args.all or args.accessors:
        print(f"\n[4. METHOD NAMING & ACCESSOR CONVENTIONS]")
        if not accessor_issues:
            print("  - Status: [PASS] All getters and setters adhere to method naming conventions (no get_ prefix, is_/has_/can_ booleans, set_ setters, slice view collection getters).")
        else:
            print(f"  - Status: [WARN] {len(accessor_issues)} accessor convention violation(s) detected:")
            for issue in accessor_issues[:15]:
                print(f"    * [{issue['type']}] {issue['file']}:{issue['line']} -> `{issue['metric']}`")
                print(f"      -> {issue['recommendation']}")
            if len(accessor_issues) > 15:
                print(f"    * ... and {len(accessor_issues) - 15} more")

    print("\n" + "=" * 76)
    print(
        f"Code Quality Summary: {len(dead)} dead, {len(zombies)} zombies, {len(vis_leaks)} visibility leaks, "
        f"{len(condition_issues)} condition soup issues, "
        f"{len(accessor_issues)} method naming & accessor issues."
    )
    print("=" * 76)

    if args.strict:
        failures = len(dead) + len(zombies) + len(vis_leaks)
        if failures > 0:
            sys.exit(1)


if __name__ == "__main__":
    main()

