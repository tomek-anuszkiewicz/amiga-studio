#!/usr/bin/env python3
"""Deterministic checks owned by roadmap-maintenance."""

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

def check_roadmap_zero_retention():
    """Verifies that ROADMAP.md exists and contains zero completed tasks ([x]) per roadmap-maintenance.md."""
    roadmap_path = REPO_ROOT / "ROADMAP.md"
    if not roadmap_path.exists():
        return {"issues": [{"message": "ROADMAP.md does not exist in repository root"}]}

    content = roadmap_path.read_text(encoding="utf-8", errors="ignore")
    completed = re.findall(r"^\s*[-*]\s+\[x\]", content, flags=re.MULTILINE | re.IGNORECASE)

    issues = []
    if completed:
        issues.append({"message": f"ROADMAP.md contains {len(completed)} completed task(s) (`[x]`). All completed tasks must be pruned per roadmap-maintenance.md"})

    return {"issues": issues}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    result = check_roadmap_zero_retention()
    for issue in result["issues"]:
        print(f"[FAIL] {issue['message']}")
    if not result["issues"]:
        print("[PASS] Check completed without issues.")
    return 1 if result["issues"] else 0


if __name__ == "__main__":
    sys.exit(main())
