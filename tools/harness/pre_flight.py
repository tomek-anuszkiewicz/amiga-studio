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

REPO_ROOT = Path(__file__).resolve().parent.parent
AGENTS_MD_PATH = REPO_ROOT / "AGENTS.md"
MAX_AGENTS_MD_BYTES = 14000
LINTER_SCRIPT = REPO_ROOT / ".agents" / "skills" / "attractor-discipline" / "scripts" / "lint_attractors.py"

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

def check_attractors():
    if not LINTER_SCRIPT.exists():
        return False, f"Linter script not found at {LINTER_SCRIPT}", 0.0
    code, stdout, stderr, elapsed = run_cmd([sys.executable, str(LINTER_SCRIPT)])
    if code != 0:
        output = stdout.strip() or stderr.strip()
        return False, f"Attractor discipline linter failed:\n{output}", elapsed
    
    count = 0
    for line in stdout.splitlines():
        if "Scanned" in line and "files" in line:
            parts = line.split()
            for p in parts:
                if p.isdigit():
                    count = int(p)
                    break
    msg = f"{count} files clean (0 violations)" if count else "Clean (0 violations)"
    return True, msg, elapsed

def check_agents_md():
    start = time.time()
    if not AGENTS_MD_PATH.exists():
        return False, "AGENTS.md not found", 0.0
    size = os.path.getsize(AGENTS_MD_PATH)
    elapsed = time.time() - start
    if size > MAX_AGENTS_MD_BYTES:
        return False, f"{size:,} bytes exceeds constitutional ceiling ({MAX_AGENTS_MD_BYTES:,} bytes) by {size - MAX_AGENTS_MD_BYTES:,} bytes", elapsed
    return True, f"{size:,} bytes (<= {MAX_AGENTS_MD_BYTES:,} limit)", elapsed

TEST_COUPLING_SCRIPT = REPO_ROOT / "tools" / "check_test_coupling.py"
API_COVERAGE_SCRIPT = REPO_ROOT / "tools" / "audit_api_coverage.py"

def check_test_coupling():
    if not TEST_COUPLING_SCRIPT.exists():
        return False, f"Test coupling script not found at {TEST_COUPLING_SCRIPT}", 0.0
    code, stdout, stderr, elapsed = run_cmd([sys.executable, str(TEST_COUPLING_SCRIPT)])
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

def main():
    quick_mode = "--quick" in sys.argv
    print(f">> Running {'Quick ' if quick_mode else ''}Pre-Flight Quality Gates...")
    
    gates = [
        ("Formatting", check_formatting),
        ("Attractor Discipline", check_attractors),
        ("AGENTS.md Ceiling", check_agents_md),
        ("Test Coupling", check_test_coupling),
        ("API Coverage", check_api_coverage),
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
        print(f"[FAIL] Pre-Flight FAILED on {len(failed)} gate(s): {', '.join(failed)}")
        sys.exit(1)
    else:
        print("[OK] All Pre-Flight Quality Gates PASSED cleanly!")
        sys.exit(0)

if __name__ == "__main__":
    main()
