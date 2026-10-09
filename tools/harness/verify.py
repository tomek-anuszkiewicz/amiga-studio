#!/usr/bin/env python3
"""
Amiga 500 change-aware commit and milestone verification.
"""

import argparse
import os
import sys
import subprocess
import time
from verify_selection import collect_changes, select_tests, snapshot
from pathlib import Path

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

REPO_ROOT = Path(__file__).resolve().parents[2]
AGENTS_MD_PATH = REPO_ROOT / "AGENTS.md"
MAX_AGENTS_MD_BYTES = 14000

def run_cmd(cmd, cwd=REPO_ROOT, env=None):
    start = time.time()
    res = subprocess.run(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace", env=env
    )
    elapsed = time.time() - start
    return res.returncode, res.stdout, res.stderr, elapsed

def check_formatting():
    code, stdout, stderr, elapsed = run_cmd(["cargo", "fmt", "--all", "--", "--check"])
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"Code formatting check failed:\n{output}", elapsed
    return True, "100% compliant", elapsed


def check_agents_md():
    start = time.time()
    if not AGENTS_MD_PATH.exists():
        return False, "AGENTS.md not found", 0.0
    size = os.path.getsize(AGENTS_MD_PATH)
    elapsed = time.time() - start
    if size > MAX_AGENTS_MD_BYTES:
        return False, f"{size:,} bytes exceeds constitutional ceiling ({MAX_AGENTS_MD_BYTES:,} bytes) by {size - MAX_AGENTS_MD_BYTES:,} bytes", elapsed
    return True, f"{size:,} bytes (<= {MAX_AGENTS_MD_BYTES:,} limit)", elapsed

API_COVERAGE_SCRIPT = Path(__file__).resolve().parent / "audit_api_coverage.py"

CODE_QUALITY_SCRIPT = Path(__file__).resolve().parent / "audit_code_quality.py"
HARDWARE_QUALITY_SCRIPT = Path(__file__).resolve().parent / "audit_hardware_quality.py"
DOCS_QUALITY_SCRIPT = Path(__file__).resolve().parent / "audit_docs_quality.py"

def check_api_coverage():
    if not API_COVERAGE_SCRIPT.exists():
        return False, f"API coverage script not found at {API_COVERAGE_SCRIPT}", 0.0
    code, stdout, stderr, elapsed = run_cmd([sys.executable, str(API_COVERAGE_SCRIPT), "--strict"])
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"API coverage audit failed:\n{output}", elapsed
    return True, "Public API name references checked (not execution coverage)", elapsed

def check_clippy_invariants():
    cmd = ["cargo", "clippy", "--workspace", "--all-targets"]
    code, stdout, stderr, elapsed = run_cmd(cmd)
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"Clippy workspace invariants check failed:\n{output}", elapsed
    return True, "100% compliant (workspace lints & compiler gates)", elapsed

def check_architecture_rules():
    cmd = ["cargo", "test", "-p", "test_runner", "--test", "test_architecture_rules", "--", "--quiet"]
    code, stdout, stderr, elapsed = run_cmd(cmd)
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"Architecture test suite failed:\n{output}", elapsed
    
    summary = "all tests passed"
    for line in stdout.splitlines():
        if "test result:" in line:
            summary = line.strip()
            break
    return True, summary, elapsed

def check_code_quality_per_commit():
    if not CODE_QUALITY_SCRIPT.exists():
        return False, f"Code quality script not found at {CODE_QUALITY_SCRIPT}", 0.0
    code, stdout, stderr, elapsed = run_cmd([sys.executable, str(CODE_QUALITY_SCRIPT), "--per-commit", "--strict"])
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"Code quality per-commit check failed:\n{output}", elapsed
    return True, "Pillars 1 & 2 compliant (zero dead code, zero visibility leaks)", elapsed

def check_hardware_quality_per_commit():
    if not HARDWARE_QUALITY_SCRIPT.exists():
        return False, f"Hardware quality script not found at {HARDWARE_QUALITY_SCRIPT}", 0.0
    code, stdout, stderr, elapsed = run_cmd([sys.executable, str(HARDWARE_QUALITY_SCRIPT), "--per-commit"])
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"Hardware quality per-commit check failed:\n{output}", elapsed
    return True, "Pillars 1 & 2 compliant (topology isolation & DMA mastership)", elapsed

def check_code_quality_milestone():
    if not CODE_QUALITY_SCRIPT.exists():
        return False, f"Code quality script not found at {CODE_QUALITY_SCRIPT}", 0.0
    code, stdout, stderr, elapsed = run_cmd([sys.executable, str(CODE_QUALITY_SCRIPT), "--milestone", "--strict"])
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"Code quality milestone check failed:\n{output}", elapsed
    return True, "Pillars 3 & 4 compliant (condition soup & accessors)", elapsed

def check_hardware_quality_milestone():
    if not HARDWARE_QUALITY_SCRIPT.exists():
        return False, f"Hardware quality script not found at {HARDWARE_QUALITY_SCRIPT}", 0.0
    code, stdout, stderr, elapsed = run_cmd([sys.executable, str(HARDWARE_QUALITY_SCRIPT), "--milestone"])
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"Hardware quality milestone check failed:\n{output}", elapsed
    return True, "Pillars 3, 4, 5 compliant (CCK timing, quirks, Tier 2 integration)", elapsed

POLISH_CHECK_SCRIPT = Path(__file__).resolve().parent / "check_polish.py"

def check_polish_language():
    if not POLISH_CHECK_SCRIPT.exists():
        return False, f"Polish check script not found at {POLISH_CHECK_SCRIPT}", 0.0
    code, stdout, stderr, elapsed = run_cmd([sys.executable, str(POLISH_CHECK_SCRIPT), "--staged"])
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"Language policy check failed:\n{output}", elapsed
    return True, "100% compliant (strict English across additions)", elapsed

def check_docs_quality():
    if not DOCS_QUALITY_SCRIPT.exists():
        return False, f"Docs quality script not found at {DOCS_QUALITY_SCRIPT}", 0.0
    code, stdout, stderr, elapsed = run_cmd([sys.executable, str(DOCS_QUALITY_SCRIPT)])
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"Docs quality audit failed:\n{output}", elapsed
    return True, "Configured documentation and workflow checks passed", elapsed


def quality_gates(milestone):
    gates = [
        ("Formatting", check_formatting),
        ("AGENTS.md Ceiling", check_agents_md),
        ("API References", check_api_coverage),
        ("Clippy Invariants", check_clippy_invariants),
        ("Architecture Rules", check_architecture_rules),
        ("Hardware Quality", check_hardware_quality_per_commit),
        ("Code Quality", check_code_quality_per_commit),
    ]
    if milestone:
        return [
            ("Hardware Quality (Milestone)", check_hardware_quality_milestone),
            ("Code Quality (Milestone)", check_code_quality_milestone),
            ("Docs Quality & Governance", check_docs_quality),
            ("Strict English Policy", check_polish_language),
        ]
    return gates


def execute_test(name, command, overrides):
    if "test_singlestep" in command or "test_dma_cartesian" in command:
        if not any((REPO_ROOT / "ref_src/SingleStepTests-680x0/68000/v1").glob("*.json")):
            return False, "Silicon vectors missing; provision external sources before verification", 0
    if any("test_vamiga_" in arg or arg == "test_chipset_benchmark" for arg in command) or name in (
        "All runnable vAmigaTS captures", "Chipset throughput"
    ):
        root = REPO_ROOT / "ref_src/vAmigaTS"
        if not any(root.rglob("*.adf")) or not any(root.rglob("*.raw")):
            return False, "vAmigaTS assets missing; skipped fixtures cannot satisfy verification", 0
    env = os.environ.copy()
    env.update(overrides)
    code, stdout, stderr, elapsed = run_cmd(command, env=env)
    detail = f"command exited with code {code}" if code else "passed"
    # vAmiga CLI currently prints failures without setting a failing process exit code.
    # Require a nonempty, complete summary instead of trusting that exit code.
    if name == "All runnable vAmigaTS captures":
        import re
        def field(label):
            match = re.search(r"^" + re.escape(label) + r"\s*(\d+)", stdout, re.MULTILINE)
            return int(match.group(1)) if match else None
        executed = field("Executed:")
        active = field("Active Baseline:")
        failed = field("Failed / Regressed:")
        if not executed or executed != active or failed != 0:
            code = 1
            detail = "missing/incomplete vAmiga summary or pixel failures"
    if code:
        return False, f"{detail}\n{stdout}\n{stderr}", elapsed
    return True, detail, elapsed


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--commit", action="store_true", help="Run all standard tests; select DMA/SingleStep from working-tree changes")
    mode.add_argument("--milestone", action="store_true", help="Run only milestone audits, chipset throughput and full vAmiga captures")
    parser.add_argument("--verify-dma", action="store_true", help="Run the Cartesian DMA set, alone or alongside a completion profile")
    parser.add_argument("--verify-singlestep", action="store_true", help="Run the full SingleStepTests set, alone or alongside a completion profile")
    parser.add_argument("--plan", action="store_true", help="Print selected commands without executing checks")
    args = parser.parse_args(argv)
    if not any((args.commit, args.milestone, args.verify_dma, args.verify_singlestep)):
        parser.error("Select --commit, --milestone, --verify-dma or --verify-singlestep")
    selected_mode = "milestone" if args.milestone else "commit" if args.commit else "standalone"
    try:
        changes = collect_changes(REPO_ROOT) if selected_mode != "standalone" else set()
        before = snapshot(REPO_ROOT)
        commands = select_tests(REPO_ROOT, changes, selected_mode,
                                verify_dma=args.verify_dma, verify_singlestep=args.verify_singlestep)
    except (RuntimeError, OSError, ValueError) as error:
        print(f"[FAIL] Cannot select verification: {error}", file=sys.stderr)
        return 1
    gates = quality_gates(args.milestone) if selected_mode != "standalone" else []
    scope = f"current working tree; changed paths: {len(changes)}" if selected_mode != "standalone" else "current working tree"
    print(f"Verification mode: {selected_mode}; {scope}", flush=True)
    for name, _ in gates:
        print(f"  Quality: {name}")
    for name, command, overrides in commands:
        print(f"  Tests: {name}\n    {' '.join(command)}")
        if overrides:
            print(f"    Environment: {overrides}")
    if args.plan:
        return 0
    results = []
    for name, check in gates:
        print(f"[RUN] {name}", flush=True)
        ok, detail, elapsed = check()
        results.append(ok)
        print(f"[{'PASS' if ok else 'FAIL'}] {name} ({elapsed:.2f}s): {detail}", flush=True)
    for name, command, overrides in commands:
        print(f"[RUN] {name}", flush=True)
        ok, detail, elapsed = execute_test(name, command, overrides)
        results.append(ok)
        print(f"[{'PASS' if ok else 'FAIL'}] {name} ({elapsed:.2f}s): {detail}", flush=True)
    try:
        unchanged = before == snapshot(REPO_ROOT)
    except (RuntimeError, OSError):
        unchanged = False
    if not unchanged:
        print("[FAIL] Source or index changed during verification. Verify the current snapshot again.")
        results.append(False)
    print(f"[{'PASS' if all(results) else 'FAIL'}] {selected_mode} verification")
    return 0 if all(results) else 1


if __name__ == "__main__":
    sys.exit(main())
