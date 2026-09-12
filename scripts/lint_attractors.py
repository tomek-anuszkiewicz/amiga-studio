#!/usr/bin/env python3
"""
Convenience trampoline forwarding to the packaged attractor discipline skill linter:
.agents/skills/attractor-discipline/scripts/lint_attractors.py
"""
import sys
import runpy
from pathlib import Path

SKILL_SCRIPT = (
    Path(__file__).resolve().parent.parent
    / ".agents"
    / "skills"
    / "attractor-discipline"
    / "scripts"
    / "lint_attractors.py"
)

if not SKILL_SCRIPT.exists():
    sys.stderr.write(f"Error: Target linter script not found at {SKILL_SCRIPT}\n")
    sys.exit(1)

runpy.run_path(str(SKILL_SCRIPT), run_name="__main__")
