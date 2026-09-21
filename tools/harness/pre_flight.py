#!/usr/bin/env python3
"""
Amiga 500 Emulator: Pre-Flight Gate & Output Compression Checker
Runs all 4 mandatory architectural quality gates and returns a condensed,
zero-noise summary on success. Dumps diagnostics strictly on failure.
"""

import os
import sys
import subprocess
import time
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

def run_cmd(cmd, cwd=REPO_ROOT):
    start = time.time()
    res = subprocess.run(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace"
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

TEST_COUPLING_SCRIPT = Path(__file__).resolve().parent / "check_test_coupling.py"
API_COVERAGE_SCRIPT = Path(__file__).resolve().parent / "audit_api_coverage.py"

CODE_QUALITY_SCRIPT = Path(__file__).resolve().parent / "audit_code_quality.py"
HARDWARE_QUALITY_SCRIPT = Path(__file__).resolve().parent / "audit_hardware_quality.py"
DOCS_QUALITY_SCRIPT = Path(__file__).resolve().parent / "audit_docs_quality.py"

def check_test_coupling(staged=False):
    if not TEST_COUPLING_SCRIPT.exists():
        return False, f"Test coupling script not found at {TEST_COUPLING_SCRIPT}", 0.0
    cmd = [sys.executable, str(TEST_COUPLING_SCRIPT)]
    if staged:
        cmd.append("--staged")
    code, stdout, stderr, elapsed = run_cmd(cmd)
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"Test coupling check failed:\n{output}", elapsed
    msg = stdout.strip().replace("[PASS] Change-Coupling Gate: ", "")
    return True, msg, elapsed

def check_api_coverage():
    if not API_COVERAGE_SCRIPT.exists():
        return False, f"API coverage script not found at {API_COVERAGE_SCRIPT}", 0.0
    code, stdout, stderr, elapsed = run_cmd([sys.executable, str(API_COVERAGE_SCRIPT), "--strict"])
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"API coverage audit failed:\n{output}", elapsed
    return True, "100% peripheral/utility public APIs tested", elapsed

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

def check_docs_quality():
    if not DOCS_QUALITY_SCRIPT.exists():
        return False, f"Docs quality script not found at {DOCS_QUALITY_SCRIPT}", 0.0
    code, stdout, stderr, elapsed = run_cmd([sys.executable, str(DOCS_QUALITY_SCRIPT)])
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"Docs quality audit failed:\n{output}", elapsed
    return True, "10/10 pillars 100% compliant", elapsed

def main():
    quick_mode = "--quick" in sys.argv
    milestone_mode = "--milestone" in sys.argv
    staged_mode = "--staged" in sys.argv or quick_mode

    if milestone_mode:
        print(">> Running Minor Roadmap Point Milestone Quality Gates...")
        gates = [
            ("Formatting", check_formatting),
            ("AGENTS.md Ceiling", check_agents_md),
            ("Test Coupling", lambda: check_test_coupling(staged=False)),
            ("API Coverage", check_api_coverage),
            ("Clippy Invariants", check_clippy_invariants),
            ("Architecture Rules", check_architecture_rules),
            ("Hardware Quality (Per-Commit: Pillars 1 & 2)", check_hardware_quality_per_commit),
            ("Code Quality (Per-Commit: Pillars 1 & 2)", check_code_quality_per_commit),
            ("Hardware Quality (Milestone: Pillars 3-5)", check_hardware_quality_milestone),
            ("Code Quality (Milestone: Pillars 3 & 4)", check_code_quality_milestone),
            ("Docs Quality & Governance (10 Pillars)", check_docs_quality),
        ]
    else:
        print(f">> Running {'Quick ' if quick_mode else ''}Per-Commit Quality Gates...")
        gates = [
            ("Formatting", check_formatting),
            ("AGENTS.md Ceiling", check_agents_md),
            ("Test Coupling", lambda: check_test_coupling(staged=staged_mode)),
            ("API Coverage", check_api_coverage),
            ("Clippy Invariants", check_clippy_invariants),
            ("Hardware Quality (Pillars 1 & 2)", check_hardware_quality_per_commit),
            ("Code Quality (Pillars 1 & 2)", check_code_quality_per_commit),
        ]
        if not quick_mode:
            gates.append(("Architecture Rules", check_architecture_rules))
        
    failed = []
    results = []
    
    for name, fn in gates:
        ok, detail, elapsed = fn()
        if ok:
            results.append(f"  [PASS] {name}: {detail} ({elapsed:.2f}s)")
        else:
            results.append(f"  [FAIL] {name}: {detail}")
            failed.append(name)
            
    print("\n" + "\n".join(results) + "\n")
    
    if failed:
        print(f"[FAIL] Quality Gates FAILED on {len(failed)} gate(s): {', '.join(failed)}")
        sys.exit(1)
    else:
        mode_label = "Milestone" if milestone_mode else "Per-Commit"
        print(f"[OK] All {mode_label} Quality Gates PASSED cleanly!")
        sys.exit(0)

if __name__ == "__main__":
    main()
