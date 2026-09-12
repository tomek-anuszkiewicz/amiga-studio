#!/usr/bin/env python3
"""
Automated Linguistic Attractor & Vocabulary Discipline Linter.

Scans documentation, rules, skills, and code in the repository to prevent:
1. Synthetic academic jargon monoculture (e.g. 'epistemic', 'teleological').
2. Theatrical testing phrases (e.g. 'testing oracle', 'oracle verification').
3. Inflated attractor catchphrases (e.g. 'zero-friction trap', 'zero cognitive friction', 'The Invariance Invariant').
4. Hardware microarchitecture term leaks into pure documentation/skills (e.g. 'L1i cache density', 'L1 instruction cache footprint').

Exits with:
  0 - Clean repository (zero attractors found)
  1 - Attractor violations detected
"""

import os
import re
import sys
from pathlib import Path

# Quarantined terms across all public documentation, rules, and skills
ATTRACTOR_PATTERNS = [
    {
        "name": "Academic Jargon ('epistemic')",
        "pattern": re.compile(r"\bepistemic\b", re.IGNORECASE),
        "suggestion": "Use 'knowledge drift', 'context', 'cognitive state', or 'architectural understanding'.",
        "scope": ["Obsidian/Amiga/Design", ".agents/rules", ".agents/skills", "crates"],
        "allow_files": ["scripts/lint_attractors.py", "DIARY.md", "test_architecture_rules.rs", "attractor-discipline.md"],  # Historical logs and linter itself allowed
    },
    {
        "name": "Academic Jargon ('teleological')",
        "pattern": re.compile(r"\bteleological\b", re.IGNORECASE),
        "suggestion": "Use 'purpose-driven', 'goal-oriented', or 'design intent'.",
        "scope": ["Obsidian/Amiga/Design", ".agents/rules", ".agents/skills", "crates"],
        "allow_files": ["scripts/lint_attractors.py", "DIARY.md", "test_architecture_rules.rs", "attractor-discipline.md"],
    },
    {
        "name": "Attractor Catchphrase ('zero-friction trap')",
        "pattern": re.compile(r"\bzero[- ]friction trap\b", re.IGNORECASE),
        "suggestion": "Use 'unverified generation', 'false sense of velocity', or 'unanchored changes'.",
        "scope": ["Obsidian/Amiga/Design", ".agents/rules", ".agents/skills", "crates"],
        "allow_files": ["scripts/lint_attractors.py", "DIARY.md", "test_architecture_rules.rs", "attractor-discipline.md"],
    },
    {
        "name": "Attractor Catchphrase ('zero cognitive friction')",
        "pattern": re.compile(r"\bzero cognitive friction\b", re.IGNORECASE),
        "suggestion": "Use 'straightforward', 'unambiguous', or 'intuitive'.",
        "scope": ["Obsidian/Amiga/Design", ".agents/rules", ".agents/skills", "crates"],
        "allow_files": ["scripts/lint_attractors.py", "DIARY.md", "test_architecture_rules.rs", "attractor-discipline.md"],
    },
    {
        "name": "Theatrical Testing Phrasing ('testing oracle' / 'the oracle')",
        "pattern": re.compile(r"\b(testing oracle|oracle verification|test oracle)\b", re.IGNORECASE),
        "suggestion": "Use 'test verification reference', 'ground truth vector', or 'hardware capture'.",
        "scope": ["Obsidian/Amiga/Design", ".agents/rules", ".agents/skills", "crates"],
        "allow_files": ["scripts/lint_attractors.py", "DIARY.md", "test_architecture_rules.rs", "attractor-discipline.md"],
    },
    {
        "name": "Inflated Catchphrase ('The Invariance Invariant')",
        "pattern": re.compile(r"\bThe Invariance Invariant\b", re.IGNORECASE),
        "suggestion": "Use 'Machine System Invariants', 'State Assertions', or 'Architectural Consistency'.",
        "scope": ["Obsidian/Amiga/Design", ".agents/rules", ".agents/skills", "crates"],
        "allow_files": ["scripts/lint_attractors.py", "DIARY.md", "test_architecture_rules.rs", "attractor-discipline.md"],
    },
    {
        "name": "Misplaced Physical Hardware Term in Documentation ('L1/L1i density/footprint')",
        "pattern": re.compile(r"\b(L1[i]? (cache )?(density|footprint)|L1i cache thrashing)\b", re.IGNORECASE),
        "suggestion": "Physical CPU cache density is not a concern for markdown skills/rules. Use 'compact', 'concise', or 'focused'.",
        "scope": [".agents/rules", ".agents/skills", "Obsidian/Amiga/Design"],
        "allow_files": ["scripts/lint_attractors.py", "DIARY.md", "test_architecture_rules.rs", "attractor-discipline.md"],
    },
]


def find_repo_root() -> Path:
    cwd = Path.cwd().resolve()
    if (cwd / "ROADMAP.md").exists():
        return cwd
    for parent in cwd.parents:
        if (parent / "ROADMAP.md").exists():
            return parent
    return cwd


def scan_file(file_path: Path, repo_root: Path):
    rel_path_str = file_path.relative_to(repo_root).as_posix()
    violations = []

    try:
        with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
            lines = f.readlines()
    except Exception as e:
        return [f"Could not read {rel_path_str}: {e}"]

    for item in ATTRACTOR_PATTERNS:
        # Check if file is whitelisted for this pattern
        if any(rel_path_str.endswith(allowed) for allowed in item["allow_files"]):
            continue

        # Check if file falls into scope
        in_scope = any(rel_path_str.startswith(s) for s in item["scope"])
        if not in_scope:
            continue

        for line_num, line in enumerate(lines, 1):
            match = item["pattern"].search(line)
            if match:
                matched_text = match.group(0)
                violations.append({
                    "file": rel_path_str,
                    "line": line_num,
                    "name": item["name"],
                    "matched": matched_text,
                    "snippet": line.strip(),
                    "suggestion": item["suggestion"],
                })

    return violations


def main():
    repo_root = find_repo_root()
    print(f"=== Running Attractor & Vocabulary Discipline Linter ===")
    print(f"Repository Root: {repo_root}\n")

    files_to_scan = []

    # Directories to scan
    scan_dirs = [
        repo_root / ".agents",
        repo_root / "Obsidian" / "Amiga" / "Design",
        repo_root / "crates",
    ]

    for d in scan_dirs:
        if not d.exists():
            continue
        for root, _, files in os.walk(d):
            for file in files:
                ext = Path(file).suffix.lower()
                if ext in [".md", ".rs"]:
                    full_path = Path(root) / file
                    files_to_scan.append(full_path)

    total_violations = []

    for file_path in files_to_scan:
        v = scan_file(file_path, repo_root)
        if v:
            total_violations.extend(v)

    if not total_violations:
        print(f"PASS: Scanned {len(files_to_scan)} files across repository.")
        print("Zero synthetic linguistic attractors or leaked hardware buzzwords found.")
        sys.exit(0)
    else:
        print(f"FAIL: Found {len(total_violations)} attractor violation(s) across {len(files_to_scan)} scanned files:\n")
        for idx, err in enumerate(total_violations, 1):
            print(f"[{idx}] {err['file']}:{err['line']}")
            print(f"    Violation : {err['name']}")
            print(f"    Matched   : \"{err['matched']}\"")
            print(f"    Line      : {err['snippet']}")
            print(f"    Remedy    : {err['suggestion']}\n")

        print("Action Required: Replace synthetic attractors with grounded, unpretentious terminology.")
        sys.exit(1)


if __name__ == "__main__":
    main()
