#!/usr/bin/env python3
"""
Amiga 500 Emulator: Change-Coupling Gate
Enforces .agents/rules/unit-testing-policy.md by verifying that any changeset
touching production code (crates/<crate>/src/) also modifies or adds dedicated
test files (crates/<crate>/tests/).
"""

import sys
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

def run_cmd(cmd):
    res = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace"
    )
    return res.returncode, res.stdout.strip()

def get_modified_files(mode):
    if mode == "--staged":
        # Staged files in git index (for pre-commit hook)
        code, out = run_cmd(["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        return out.splitlines() if out else []
    elif mode == "--last-commit":
        # Inspect files in the latest commit
        code, out = run_cmd(["git", "diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"])
        return out.splitlines() if out else []
    else:
        # Working tree vs HEAD (staged + unstaged + untracked)
        files = set()
        code, out = run_cmd(["git", "diff", "--name-only", "HEAD"])
        if out:
            files.update(out.splitlines())
        code, status_out = run_cmd(["git", "status", "--porcelain"])
        if status_out:
            for line in status_out.splitlines():
                if len(line) > 3:
                    files.add(line[3:].strip())
        return list(files)

def check_test_coupling(mode="--working-tree"):
    files = get_modified_files(mode)
    if not files:
        return True, "No modified files detected."

    # Map crates to sets of modified categories
    # crate_name -> {"src": [...], "tests": [...]}
    crates_modified = {}

    for file_path_str in files:
        p = Path(file_path_str.replace("\\", "/"))
        parts = p.parts
        if len(parts) >= 3 and parts[0] == "crates":
            crate_name = parts[1]
            category = parts[2] # "src" or "tests"
            if category in ("src", "tests"):
                if crate_name not in crates_modified:
                    crates_modified[crate_name] = {"src": [], "tests": []}
                crates_modified[crate_name][category].append(file_path_str)

    violations = []
    for crate, paths in crates_modified.items():
        src_files = paths["src"]
        test_files = paths["tests"]

        # Only require tests if Rust source files (.rs) were modified under src/
        rs_src_files = [f for f in src_files if f.endswith(".rs")]
        if rs_src_files and not test_files:
            violations.append((crate, rs_src_files))

    if violations:
        msg_lines = [
            "Change-Coupling Violation: Production code modified without corresponding unit tests!",
            "Per .agents/rules/unit-testing-policy.md, every change to crates/<crate>/src/ must be accompanied by tests in crates/<crate>/tests/:\n"
        ]
        for crate, files_list in violations:
            msg_lines.append(f"  • crates/{crate}/src/ modified ({len(files_list)} file(s)):")
            for f in files_list[:3]:
                msg_lines.append(f"      - {f}")
            if len(files_list) > 3:
                msg_lines.append(f"      - ... and {len(files_list) - 3} more")
            msg_lines.append(f"    Missing updates in: crates/{crate}/tests/\n")
        return False, "\n".join(msg_lines)

    inspected_crates = [c for c, p in crates_modified.items() if p["src"]]
    if inspected_crates:
        return True, f"Coupling verified for {len(inspected_crates)} modified crate(s): {', '.join(inspected_crates)}"
    return True, "No crate source modifications detected."

def main():
    mode = "--working-tree"
    if "--staged" in sys.argv:
        mode = "--staged"
    elif "--last-commit" in sys.argv:
        mode = "--last-commit"

    ok, detail = check_test_coupling(mode)
    if not ok:
        print(f"\n[FAIL] {detail}\n")
        sys.exit(1)
    else:
        print(f"[PASS] Change-Coupling Gate: {detail}")
        sys.exit(0)

if __name__ == "__main__":
    main()
