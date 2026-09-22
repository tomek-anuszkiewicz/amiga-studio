#!/usr/bin/env python3
"""
Amiga 500 Emulator: Test Tier Runner CLI
Provides unified execution for the 3-Tier Testing Architecture:
  - Tier 1 (--unit): Fast, isolated unit tests (< 2s).
  - Tier 2 (--integration): Multi-crate integration & headless UI interaction tests.
  - Tier 3 (--harness): Silicon verification, Cartesian DMA sweeps & benchmark tests.
  - All (--all): Tier 1 + Tier 2 suites.
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

# Tier 1: Pure Unit Tests (isolated crate / algorithmic modules)
TIER1_UNIT_CRATES = [
    "agnus",
    "audio",
    "blitter",
    "cia",
    "config",
    "copper",
    "denise",
    "disassembler",
    "dma",
    "floppy",
    "frame_builder",
    "game_ports",
    "interrupts",
    "joystick",
    "keyboard",
    "cpu",
    "mouse",
    "paula",
    "physical_memory",
    "rtc",
    "sprites",
]

TIER1_TEST_RUNNER_TESTS = [
    "test_anomaly",
    "test_builder",
    "test_golden_row_hashes",
    "test_persistence",
    "test_platform",
    "test_prng",
    "test_stats",
]

# Tier 2: Multi-Crate Subsystem Integration
TIER2_INTEGRATION_CRATES = [
    "memory_bus",
    "machine_loop",
    "debugger",
    "gui",
]

# Tier 3: Verification Harness & Silicon Ground Truth
TIER3_HARNESS_TESTS = [
    ("test_runner", "test_architecture_rules"),
    ("test_runner", "test_dma_cartesian"),
    ("test_runner", "test_benchmark_smoke"),
]


def run_cmd(cmd, cwd=REPO_ROOT):
    start = time.time()
    res = subprocess.run(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    elapsed = time.time() - start
    return res.returncode, res.stdout, res.stderr, elapsed


def run_tier1_unit(quiet=False):
    print("=== Running Tier 1: Isolated Unit Tests ===")
    total_start = time.time()
    failed = []

    # Run unit crates
    cmd = ["cargo", "test"]
    for c in TIER1_UNIT_CRATES:
        cmd.extend(["-p", c])
    if quiet:
        cmd.extend(["--", "--quiet"])

    code, stdout, stderr, elapsed = run_cmd(cmd)
    if code != 0:
        print(stdout)
        print(stderr, file=sys.stderr)
        return False, elapsed

    # Run test_runner unit suites
    cmd_tr = ["cargo", "test", "-p", "test_runner"]
    for t in TIER1_TEST_RUNNER_TESTS:
        cmd_tr.extend(["--test", t])
    if quiet:
        cmd_tr.extend(["--", "--quiet"])

    code_tr, stdout_tr, stderr_tr, elapsed_tr = run_cmd(cmd_tr)
    if code_tr != 0:
        print(stdout_tr)
        print(stderr_tr, file=sys.stderr)
        return False, elapsed + elapsed_tr

    total_elapsed = time.time() - total_start
    print(
        f"[PASS] Tier 1 Unit Tests: 23 crates + 7 test_runner unit suites ({total_elapsed:.2f}s)"
    )
    return True, total_elapsed


def run_tier2_integration(quiet=False):
    print("=== Running Tier 2: Multi-Crate Integration Tests ===")
    total_start = time.time()

    cmd = ["cargo", "test"]
    for c in TIER2_INTEGRATION_CRATES:
        cmd.extend(["-p", c])
    if quiet:
        cmd.extend(["--", "--quiet"])

    code, stdout, stderr, elapsed = run_cmd(cmd)
    if code != 0:
        print(stdout)
        print(stderr, file=sys.stderr)
        return False, elapsed

    total_elapsed = time.time() - total_start
    print(
        f"[PASS] Tier 2 Integration Tests: {', '.join(TIER2_INTEGRATION_CRATES)} ({total_elapsed:.2f}s)"
    )
    return True, total_elapsed


def run_tier3_harness(quiet=False):
    print("=== Running Tier 3: Verification Harness & Architecture Tests ===")
    total_start = time.time()

    cmd = ["cargo", "test", "-p", "test_runner"]
    for _, t in TIER3_HARNESS_TESTS:
        cmd.extend(["--test", t])
    if quiet:
        cmd.extend(["--", "--quiet"])

    code, stdout, stderr, elapsed = run_cmd(cmd)
    if code != 0:
        print(stdout)
        print(stderr, file=sys.stderr)
        return False, elapsed

    total_elapsed = time.time() - total_start
    print(f"[PASS] Tier 3 Harness Tests ({total_elapsed:.2f}s)")
    return True, total_elapsed


def main():
    parser = argparse.ArgumentParser(
        description="Amiga 500 Emulator 3-Tier Test Runner"
    )
    parser.add_argument(
        "--unit",
        "-u",
        action="store_true",
        help="Run Tier 1 isolated unit tests (< 2s)",
    )
    parser.add_argument(
        "--integration",
        "-i",
        action="store_true",
        help="Run Tier 2 multi-crate integration tests",
    )
    parser.add_argument(
        "--harness",
        "-H",
        action="store_true",
        help="Run Tier 3 verification harness tests",
    )
    parser.add_argument(
        "--all",
        "-a",
        action="store_true",
        help="Run Tier 1 unit + Tier 2 integration tests",
    )
    parser.add_argument(
        "--quiet",
        "-q",
        action="store_true",
        help="Zero-noise output on success",
    )

    args = parser.parse_args()

    # Default to --unit if no tier specified
    if not (args.unit or args.integration or args.harness or args.all):
        args.unit = True

    overall_pass = True
    start = time.time()

    if args.unit or args.all:
        ok, _ = run_tier1_unit(quiet=args.quiet)
        if not ok:
            overall_pass = False

    if args.integration or args.all:
        ok, _ = run_tier2_integration(quiet=args.quiet)
        if not ok:
            overall_pass = False

    if args.harness:
        ok, _ = run_tier3_harness(quiet=args.quiet)
        if not ok:
            overall_pass = False

    elapsed = time.time() - start
    if overall_pass:
        print(f"\n[OK] All requested test tiers passed in {elapsed:.2f}s.")
        sys.exit(0)
    else:
        print(f"\n[FAIL] Test failures detected in {elapsed:.2f}s.", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
