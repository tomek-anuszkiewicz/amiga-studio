#!/usr/bin/env python3
"""
Amiga 500 Emulator: Public API Test Coverage Auditor
Statically inspects all workspace crates and verifies that public functions
(pub fn) declared in crates/<crate>/src/ are referenced in unit and integration tests.
"""

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CRATES_DIR = REPO_ROOT / "crates"

# Known dispatch tables or auto-generated opcode handlers that are executed via tables
EXEMPT_CRATES = {
    # m68000 uses a 65,536-entry static fn pointer table (dispatch_table.rs)
    # where handlers like op_move_b are called via opcode index, not by direct name.
    "m68000",
}

# Standard boilerplate methods that don't need explicit name-matching in tests
BOILERPLATE_NAMES = {
    "new", "default", "clone", "fmt", "eq", "ne", "drop",
    "serialize", "deserialize",
}

def audit_crate_api(crate_path):
    src_dir = crate_path / "src"
    tests_dir = crate_path / "tests"

    if not src_dir.exists() or not tests_dir.exists():
        return None

    # Collect all pub fn names from src/
    pub_fns = set()
    for rs_file in src_dir.rglob("*.rs"):
        try:
            content = rs_file.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue

        # Match pub fn, pub const fn, pub unsafe fn
        for match in re.finditer(r"pub\s+(?:const\s+|unsafe\s+)?fn\s+([a-zA-Z0-9_]+)", content):
            fn_name = match.group(1)
            if fn_name not in BOILERPLATE_NAMES and not fn_name.startswith("_"):
                pub_fns.add(fn_name)

    if not pub_fns:
        return None

    # Collect all test content
    test_content = ""
    for rs_file in tests_dir.rglob("*.rs"):
        try:
            test_content += rs_file.read_text(encoding="utf-8", errors="ignore") + "\n"
        except Exception:
            continue

    # Also collect workspace tests that might integrate this crate
    all_tests_dir = REPO_ROOT / "crates" / "test_runner" / "tests"
    if all_tests_dir.exists() and crate_path.name != "test_runner":
        for rs_file in all_tests_dir.rglob("*.rs"):
            try:
                test_content += rs_file.read_text(encoding="utf-8", errors="ignore") + "\n"
            except Exception:
                continue

    untested = sorted([fn for fn in pub_fns if fn not in test_content])
    return {
        "crate": crate_path.name,
        "total": len(pub_fns),
        "tested": len(pub_fns) - len(untested),
        "untested": untested
    }

def main():
    print("=== Scanning Workspace Crates for Public API Test References ===")
    results = []

    for item in sorted(CRATES_DIR.iterdir()):
        if not item.is_dir() or not (item / "Cargo.toml").exists():
            continue
        if item.name in EXEMPT_CRATES:
            continue

        res = audit_crate_api(item)
        if res:
            results.append(res)

    failing_crates = []
    for r in results:
        pct = (r["tested"] / r["total"] * 100) if r["total"] > 0 else 100
        status = "PASS" if not r["untested"] else "WARN"
        print(f"  [{status}] crates/{r['crate']:15} -> {r['tested']}/{r['total']} public fns tested ({pct:.1f}%)")
        if r["untested"]:
            for fn in r["untested"][:5]:
                print(f"         - missing test for: {fn}")
            if len(r["untested"]) > 5:
                print(f"         - ... and {len(r['untested']) - 5} more")

    # In strict mode, fail if any peripheral or utility crate has untested public methods
    strict_crates = {
        "joystick", "mouse", "parallel_port", "serial_port", "frame_builder",
        "keyboard", "game_ports", "rtc"
    }

    strict_failures = [r for r in results if r["crate"] in strict_crates and r["untested"]]
    if strict_failures:
        print(f"\n[FAIL] Found {len(strict_failures)} peripheral/utility crate(s) with untested public APIs:")
        for r in strict_failures:
            print(f"  • crates/{r['crate']}: {len(r['untested'])} untested: {', '.join(r['untested'][:5])}")
        if "--strict" in sys.argv:
            sys.exit(1)

    print(f"\n[OK] Scan completed across {len(results)} crates.")
    sys.exit(0)

if __name__ == "__main__":
    main()
