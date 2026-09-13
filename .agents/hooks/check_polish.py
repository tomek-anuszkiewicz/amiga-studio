#!/usr/bin/env python3
"""
Antigravity lifecycle hook runner for check_polish.py
"""
import sys
from pathlib import Path
import runpy

script_dir = Path(__file__).resolve().parent
repo_root = script_dir.parent
target = repo_root / "tools" / "check_polish.py"

if not target.exists():
    # Fallback search up
    for parent in script_dir.parents:
        cand = parent / "tools" / "check_polish.py"
        if cand.exists():
            target = cand
            break

if not target.exists():
    print(f"Error: {target} not found", file=sys.stderr)
    sys.exit(1)

runpy.run_path(str(target), run_name="__main__")
