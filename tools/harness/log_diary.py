#!/usr/bin/env python3
"""
tools/log_diary.py
Deterministic CLI Appender for DIARY.md Section 10.

Eliminates the need for the LLM to read 90+ KB of DIARY.md into its context window.
Directly formats and appends timestamped architectural entries adhering strictly
to .agents/rules/diary-maintenance.md.
"""

import sys
import argparse
import datetime
from pathlib import Path

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

REPO_ROOT = Path(__file__).resolve().parent.parent
DIARY_PATH = REPO_ROOT / "DIARY.md"

def get_current_timestamp():
    """Returns timestamp in format 'YYYY-MM-DD HH:MM CEST' (or CET depending on DST)."""
    now = datetime.datetime.now()
    # Approximate timezone identifier for Central European Time
    # In Europe, DST is roughly last Sunday of March to last Sunday of October
    month = now.month
    tz = "CEST" if 4 <= month <= 9 else "CET"
    return now.strftime(f"%Y-%m-%d %H:%M {tz}")

def split_items(value):
    """Parses a comma-separated or newline-separated string into a list of items."""
    if not value:
        return []
    if isinstance(value, list):
        items = []
        for v in value:
            items.extend(split_items(v))
        return items
    
    # Split by newlines or semicolons if present
    lines = value.splitlines()
    if len(lines) > 1:
        return [line.strip().lstrip("-* ").strip() for line in lines if line.strip()]
    
    # Otherwise split by semicolon or comma if semicolon not present
    if ";" in value:
        return [part.strip().lstrip("-* ").strip() for part in value.split(";") if part.strip()]
    elif "," in value:
        return [part.strip().lstrip("-* ").strip() for part in value.split(",") if part.strip()]
    else:
        return [value.strip().lstrip("-* ").strip()]

def format_entry(title, subsystems, changes, rationale, results, timestamp=None):
    if not timestamp:
        timestamp = get_current_timestamp()

    subsystem_list = split_items(subsystems)
    change_list = split_items(changes)
    rationale_list = split_items(rationale)
    result_list = split_items(results)

    lines = []
    lines.append("---")
    lines.append("")
    lines.append(f"### [{timestamp}] — {title.strip()}")
    
    lines.append("- **Affected Subsystems**:")
    for sub in subsystem_list:
        lines.append(f"  - `{sub}`" if not sub.startswith("`") and not sub.startswith("[") else f"  - {sub}")

    lines.append("- **What Was Changed (The Concrete Reality)**:")
    for ch in change_list:
        lines.append(f"  - {ch}")

    lines.append("- **Architectural Rationale & Trade-Offs**:")
    for rat in rationale_list:
        lines.append(f"  - {rat}")

    lines.append("- **Verification & Test Results**:")
    for res in result_list:
        lines.append(f"  - {res}")
    
    lines.append("")
    return "\n".join(lines)

def main():
    parser = argparse.ArgumentParser(
        description="Deterministic CLI Appender for DIARY.md Section 10."
    )
    parser.add_argument("--title", required=True, help="Title of the diary entry")
    parser.add_argument("--subsystems", required=True, help="Affected subsystems (comma/newline separated)")
    parser.add_argument("--changes", required=True, help="What was changed (bullet points or semicolon/newline separated)")
    parser.add_argument("--rationale", required=True, help="Architectural rationale & trade-offs")
    parser.add_argument("--results", required=True, help="Verification & test results")
    parser.add_argument("--date", help="Optional timestamp override (e.g. '2026-09-12 23:00 CEST')")
    parser.add_argument("--dry-run", action="store_true", help="Print entry without modifying DIARY.md")

    args = parser.parse_args()

    if not DIARY_PATH.exists():
        print(f"[ERROR] DIARY.md not found at {DIARY_PATH}", file=sys.stderr)
        sys.exit(1)

    entry_text = format_entry(
        title=args.title,
        subsystems=args.subsystems,
        changes=args.changes,
        rationale=args.rationale,
        results=args.results,
        timestamp=args.date
    )

    if args.dry_run:
        print(">> [DRY-RUN] Generated DIARY.md entry:")
        print(entry_text)
        sys.exit(0)

    # Read existing content to determine line count
    with open(DIARY_PATH, "r", encoding="utf-8") as f:
        existing = f.read()

    # Ensure trailing newline on existing content
    if not existing.endswith("\n"):
        existing += "\n"

    start_line = existing.count("\n") + 1
    new_content = existing + entry_text
    end_line = new_content.count("\n")

    with open(DIARY_PATH, "w", encoding="utf-8", newline="\n") as f:
        f.write(new_content)

    file_size_kb = DIARY_PATH.stat().st_size / 1024
    print(f"[PASS] Appended entry to DIARY.md: '{args.title}' (Lines {start_line}-{end_line}, {file_size_kb:.1f} KB)")

if __name__ == "__main__":
    main()
