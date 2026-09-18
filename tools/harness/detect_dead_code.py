#!/usr/bin/env python3
"""
detect_dead_code.py - Amiga 500 Emulator Dead & Zombie Code Auditor

Identifies:
1. Completely Dead Code: Symbols with 0 callers anywhere in workspace.
2. Test-Only Zombie Code: Symbols with 0 callers in production (crates/*/src/),
   but referenced solely in unit/integration tests (crates/*/tests/).
"""

import argparse
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CRATES_DIR = REPO_ROOT / "crates"

# Crates or modules with special execution models (e.g. static dispatch tables)
EXEMPT_CRATES = {
    # m68000 instruction handlers (e.g. op_move_b) are called via 65,536-entry function pointer array
    "m68000",
}

# Standard trait/boilerplate methods to ignore
BOILERPLATE_NAMES = {
    "new", "default", "clone", "fmt", "eq", "ne", "drop",
    "serialize", "deserialize", "from", "into", "as_ref",
    "step", "step_cck", "reset", "reset_warm",
}

# Regexes for symbol declarations in Rust source
RE_FN = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s+(?:const\s+|unsafe\s+)?fn\s+([a-zA-Z0-9_]+)\b")
RE_CONST = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s+const\s+([A-Z0-9_]+)\b")
RE_TYPE = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s+(?:struct|enum|type)\s+([a-zA-Z0-9_]+)\b")


def collect_declared_symbols(crate_dir):
    """Collects all declared public functions, constants, and types in crates/<crate>/src/."""
    src_dir = crate_dir / "src"
    if not src_dir.exists():
        return []

    declared = []
    for rs_file in src_dir.rglob("*.rs"):
        try:
            content = rs_file.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue

        for line_num, line in enumerate(content.splitlines(), 1):
            # Match function
            m_fn = RE_FN.match(line)
            if m_fn:
                name = m_fn.group(1)
                if name not in BOILERPLATE_NAMES and not name.startswith("_"):
                    declared.append({
                        "kind": "fn",
                        "name": name,
                        "file": rs_file,
                        "line": line_num,
                        "crate": crate_dir.name,
                    })
                continue

            # Match const
            m_const = RE_CONST.match(line)
            if m_const:
                name = m_const.group(1)
                if not name.startswith("_"):
                    declared.append({
                        "kind": "const",
                        "name": name,
                        "file": rs_file,
                        "line": line_num,
                        "crate": crate_dir.name,
                    })
                continue

            # Match struct / enum / type
            m_type = RE_TYPE.match(line)
            if m_type:
                name = m_type.group(1)
                if not name.startswith("_"):
                    declared.append({
                        "kind": "type",
                        "name": name,
                        "file": rs_file,
                        "line": line_num,
                        "crate": crate_dir.name,
                    })
                continue

    return declared


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


def count_symbol_references(symbol_name, files, def_file, def_line):
    """Counts references to symbol_name across given files, excluding the definition point."""
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
            # Exclude pub use re-exports as independent caller
            if "pub use " in line and symbol_name in line:
                continue
            if pattern.search(line):
                callers.append((str(file_path.relative_to(REPO_ROOT)), line_num))

    return callers


def scan_dead_and_zombie_code(target_crate=None):
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
            "crate": sym["crate"],
            "file": str(def_file.relative_to(REPO_ROOT)),
            "line": def_line,
            "prod_callers_count": len(prod_callers),
            "test_callers_count": len(test_callers),
            "test_callers": test_callers[:5],
        }

        if len(prod_callers) == 0 and len(test_callers) == 0:
            completely_dead.append(sym_info)
        elif len(prod_callers) == 0 and len(test_callers) > 0:
            test_only_zombies.append(sym_info)

    return completely_dead, test_only_zombies


def main():
    parser = argparse.ArgumentParser(description="Audit dead and test-only zombie code across crates.")
    parser.add_argument("--crate", help="Filter scan to a specific crate")
    parser.add_argument("--dead-only", action="store_true", help="Only show completely dead code")
    parser.add_argument("--zombies-only", action="store_true", help="Only show test-only zombie code")
    parser.add_argument("--json", action="store_true", help="Output results as JSON")
    args = parser.parse_args()

    dead, zombies = scan_dead_and_zombie_code(args.crate)

    if args.json:
        out = {
            "completely_dead": dead if not args.zombies_only else [],
            "test_only_zombies": zombies if not args.dead_only else [],
        }
        print(json.dumps(out, indent=2))
        return

    if sys.stdout.encoding != "utf-8":
        try:
            sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass

    print("=" * 70)
    print(" AMIGA 500 EMULATOR: DEAD & ZOMBIE CODE AUDIT REPORT")
    print("=" * 70)

    if not args.zombies_only:
        print(f"\n[DEAD CODE] ({len(dead)} symbol(s) with 0 callers anywhere):")
        if not dead:
            print("   [Clean] No completely dead public symbols found.")
        else:
            for s in dead:
                print(f"   - [{s['kind']}] {s['crate']}::{s['name']} -> {s['file']}:{s['line']}")

    if not args.dead_only:
        print(f"\n[TEST-ONLY ZOMBIES] ({len(zombies)} symbol(s) called ONLY in unit tests):")
        print("   (Unused by production code; exists solely to satisfy its own test)")
        if not zombies:
            print("   [Clean] No test-only zombie symbols found.")
        else:
            for s in zombies:
                callers_str = ", ".join(f"{c[0]}:{c[1]}" for c in s["test_callers"][:2])
                print(f"   - [{s['kind']}] {s['crate']}::{s['name']} -> {s['file']}:{s['line']}")
                print(f"       Tested in: {callers_str}")

    print("\n" + "=" * 70)
    print(f"Summary: {len(dead)} dead, {len(zombies)} test-only zombie symbols.")
    print("=" * 70)


if __name__ == "__main__":
    main()
