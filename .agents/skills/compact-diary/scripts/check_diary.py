#!/usr/bin/env python3
"""Deterministic checks owned by compact-diary."""

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

def check_diary_structure_and_chronology():
    """Verifies that DIARY.md exists, contains Section 10, and entries are strictly chronological per diary-maintenance.md."""
    diary_path = REPO_ROOT / "DIARY.md"
    if not diary_path.exists():
        return {"total_entries": 0, "issues": [{"message": "DIARY.md does not exist in repository root"}]}

    content = diary_path.read_text(encoding="utf-8", errors="ignore")
    section10 = re.split(r"^##\s+10\.\s+", content, flags=re.MULTILINE)
    if len(section10) < 2:
        return {"total_entries": 0, "issues": [{"message": "DIARY.md is missing Section 10 (`## 10. Living Chronological Engineering Log...`)"}]}

    timestamps = re.findall(r"###\s+\[(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2})", section10[1])
    issues = []
    for i in range(1, len(timestamps)):
        if timestamps[i] < timestamps[i - 1]:
            issues.append({"message": f"DIARY.md Section 10 chronological order violation: `{timestamps[i - 1]}` followed by earlier `{timestamps[i]}`"})

    return {
        "total_entries": len(timestamps),
        "issues": issues,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    result = check_diary_structure_and_chronology()
    print(f"Diary timestamps inspected: {result['total_entries']}")
    for issue in result["issues"]:
        print(f"[FAIL] {issue['message']}")
    if not result["issues"]:
        print("[PASS] Check completed without issues.")
    return 1 if result["issues"] else 0


if __name__ == "__main__":
    sys.exit(main())
