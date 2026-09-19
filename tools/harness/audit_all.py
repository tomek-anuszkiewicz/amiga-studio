#!/usr/bin/env python3
"""
Amiga 500 Emulator: Unified Quality Audit Master Orchestrator (audit-all)
Executes the comprehensive triad of quality auditing suites:
  1. audit-code-quality: Rust code health, dead code, zombies, visibility, SRP, inlining, tests.
  2. audit-docs-quality: Obsidian specs sync, vault links, size ceilings, skills catalog, governance.
  3. audit-hardware-quality: Motherboard bus topology, Agnus DMA mastership, passive chips, CCK stepping.
"""

import argparse
import os
import subprocess
import sys
import time
from pathlib import Path

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

REPO_ROOT = Path(__file__).resolve().parents[2]


def run_subcommand(name: str, script_path: Path, extra_args: list[str]) -> tuple[int, float, str]:
    """Execute a python audit script and return (returncode, elapsed_time, combined_output)."""
    cmd = [sys.executable, str(script_path)] + extra_args
    start_time = time.time()
    try:
        proc = subprocess.run(
            cmd,
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
        elapsed = time.time() - start_time
        combined_output = proc.stdout + ("\n" + proc.stderr if proc.stderr else "")
        return proc.returncode, elapsed, combined_output
    except Exception as exc:
        elapsed = time.time() - start_time
        return 1, elapsed, f"Execution failed: {exc}"


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Amiga 500 Emulator: Unified Quality Audit Master Suite (audit-all)"
    )
    parser.add_argument(
        "--all",
        action="store_true",
        default=True,
        help="Run the complete triad of quality suites (code, docs, hardware). Default: True",
    )
    parser.add_argument(
        "--code",
        action="store_true",
        help="Run only the Rust Code Quality suite (audit_code_quality.py)",
    )
    parser.add_argument(
        "--docs",
        action="store_true",
        help="Run only the Documentation & Governance Quality suite (audit_docs_quality.py)",
    )
    parser.add_argument(
        "--hardware",
        action="store_true",
        help="Run only the Physical Hardware & Topology Quality suite (audit_hardware_quality.py)",
    )
    parser.add_argument(
        "--pre-flight",
        action="store_true",
        help="Also execute the pre_flight.py repository validation gate",
    )
    parser.add_argument(
        "--quiet",
        "-q",
        action="store_true",
        help="Suppress detailed sub-suite logs and show only the executive dashboard",
    )

    args = parser.parse_args()

    selective = args.code or args.docs or args.hardware
    run_code = args.code or (not selective)
    run_docs = args.docs or (not selective)
    run_hw = args.hardware or (not selective)
    run_preflight = args.pre_flight

    print("=" * 80)
    print(" AMIGA 500 EMULATOR: COMPREHENSIVE QUALITY AUDIT (audit-all)")
    print("=" * 80)
    print(f"Repository Root: {REPO_ROOT.name}")
    print(f"Target Suites: Code={run_code}, Docs={run_docs}, Hardware={run_hw}, Pre-Flight={run_preflight}")
    print("=" * 80)
    print()

    suites_to_run = []
    if run_code:
        suites_to_run.append(("Rust Code Quality", REPO_ROOT / "tools" / "harness" / "audit_code_quality.py", ["--all"]))
    if run_docs:
        suites_to_run.append(("Documentation & Governance", REPO_ROOT / "tools" / "harness" / "audit_docs_quality.py", ["--all"]))
    if run_hw:
        suites_to_run.append(("Hardware Silicon Compliance", REPO_ROOT / "tools" / "harness" / "audit_hardware_quality.py", ["--all"]))
    if run_preflight:
        suites_to_run.append(("Pre-Flight Quality Gate", REPO_ROOT / "tools" / "harness" / "pre_flight.py", []))

    results = []
    overall_start = time.time()

    for name, script_path, extra_args in suites_to_run:
        print(f">> Executing {name} ({script_path.name})...")
        code, elapsed, output = run_subcommand(name, script_path, extra_args)
        status_str = "[PASS]" if code == 0 else "[FAIL]"
        print(f"   Status: {status_str} (in {elapsed:.2f}s)")
        if not args.quiet or code != 0:
            for line in output.strip().splitlines():
                print(f"   | {line}")
            print()
        results.append((name, script_path.name, code, elapsed))

    total_time = time.time() - overall_start

    print("=" * 80)
    print(" QUALITY AUDIT EXECUTIVE DASHBOARD")
    print("=" * 80)
    print(f"{'Audit Suite':<32} {'Script':<28} {'Duration':<10} {'Result'}")
    print("-" * 80)

    any_failed = False
    for name, script_file, code, elapsed in results:
        status_tag = "[PASS] OK" if code == 0 else "[FAIL] ISSUES DETECTED"
        if code != 0:
            any_failed = True
        print(f"{name:<32} {script_file:<28} {elapsed:>6.2f}s    {status_tag}")

    print("-" * 80)
    print(f"Total Audit Time: {total_time:.2f}s")
    if any_failed:
        print("OVERALL STATUS: [FAIL] One or more quality audit suites detected violations.")
        print("=" * 80)
        return 1
    else:
        print("OVERALL STATUS: [PASS] All quality audit suites PASSED cleanly with 0 violations!")
        print("=" * 80)
        return 0


if __name__ == "__main__":
    sys.exit(main())
