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
    {
        "name": "Heading Sloganization ('mechanical sympathy' in Markdown headings)",
        "pattern": re.compile(r"^#+\s+.*mechanical sympathy", re.IGNORECASE),
        "suggestion": "Do not sloganize 'mechanical sympathy' in Markdown headings. Use 'Host Hardware Efficiency', 'Physical Execution Reality', or 'Host Pipeline Optimization'.",
        "scope": ["Obsidian/Amiga/Design", ".agents/rules", ".agents/skills", "ROADMAP.md"],
        "allow_files": ["scripts/lint_attractors.py", "DIARY.md", "test_architecture_rules.rs", "attractor-discipline.md"],
    },
    {
        "name": "Inflated Slogan Catchphrase ('mechanical sympathy invariant' / 'guardian of mechanical sympathy')",
        "pattern": re.compile(r"\b(guardian of mechanical sympathy|outlawing mechanical sympathy|mechanical sympathy invariant)\b", re.IGNORECASE),
        "suggestion": "Avoid sloganized mechanical sympathy catchphrases. Use 'guardian of hardware reality', 'hardware-aligned execution', etc.",
        "scope": ["Obsidian/Amiga/Design", ".agents/rules", ".agents/skills", "crates"],
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


def scan_file(file_path: Path, repo_root: Path, is_custom_path: bool = False):
    try:
        rel_path_str = file_path.relative_to(repo_root).as_posix()
    except ValueError:
        rel_path_str = file_path.as_posix()

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

        # Check if file falls into scope (bypassed if scanning custom target paths)
        if not is_custom_path:
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


def apply_replacements(content: str) -> tuple[str, int]:
    replacements = [
        (re.compile(r"\bepistemic drift\b", re.IGNORECASE), "knowledge drift"),
        (re.compile(r"\bepistemic debt\b", re.IGNORECASE), "context decay"),
        (re.compile(r"\bepistemic anchor\b", re.IGNORECASE), "authoritative reference"),
        (re.compile(r"\bepistemic\b", re.IGNORECASE), "contextual"),
        (re.compile(r"\bteleological intent\b", re.IGNORECASE), "design intent"),
        (re.compile(r"\bteleological\b", re.IGNORECASE), "purpose-driven"),
        (re.compile(r"\bzero[- ]friction trap\b", re.IGNORECASE), "unverified code generation"),
        (re.compile(r"\bzero cognitive friction\b", re.IGNORECASE), "intuitive workflow"),
        (re.compile(r"\bThe Invariance Invariant\b"), "Machine System Invariants"),
        (re.compile(r"\bthe invariance invariant\b", re.IGNORECASE), "machine system invariants"),
        (re.compile(r"\btesting oracle\b", re.IGNORECASE), "test verification reference"),
        (re.compile(r"\boracle verification\b", re.IGNORECASE), "test verification"),
        (re.compile(r"\btest oracle\b", re.IGNORECASE), "test verification reference"),
        (re.compile(r"\bL1i cache density\b", re.IGNORECASE), "compact execution paths"),
        (re.compile(r"\bL1i density\b", re.IGNORECASE), "compact execution paths"),
        (re.compile(r"\bL1 cache footprint\b", re.IGNORECASE), "compact memory footprint"),
        (re.compile(r"\bL1i cache thrashing\b", re.IGNORECASE), "instruction cache stalls"),
        (re.compile(r"\bguardian of mechanical sympathy\b", re.IGNORECASE), "guardian of hardware reality"),
        (re.compile(r"\bmechanical sympathy invariant\b", re.IGNORECASE), "hardware reality invariant"),
        (re.compile(r"\boutlawing mechanical sympathy\b", re.IGNORECASE), "outlawing direct hardware optimization"),
        (re.compile(r"(^#+\s+.*)\bmechanical sympathy\b", re.IGNORECASE | re.MULTILINE), r"\1host hardware efficiency"),
    ]

    total_fixed = 0
    new_content = content

    for pattern, rep in replacements:
        def rep_fn(match):
            nonlocal total_fixed
            total_fixed += 1
            text = match.group(0)
            if text.isupper():
                return rep.upper()
            elif text[0].isupper():
                return rep[0].upper() + rep[1:]
            return rep.lower()

        new_content = pattern.sub(rep_fn, new_content)

    return new_content, total_fixed


def main():
    args = sys.argv[1:]
    fix_mode = "--fix" in args
    dry_run = "--dry-run" in args
    custom_paths = [a for a in args if not a.startswith("--")]

    repo_root = find_repo_root()
    print(f"=== Running Attractor & Vocabulary Discipline Linter ===")
    print(f"Repository Root: {repo_root}")
    if fix_mode:
        print("Mode: Automated Cleaning (--fix enabled)\n")
    elif dry_run:
        print("Mode: Dry Run Preview (--dry-run enabled)\n")
    else:
        print("Mode: Validation Linting\n")

    files_to_scan = []

    if custom_paths:
        for p_str in custom_paths:
            p = Path(p_str).resolve()
            if p.is_file():
                files_to_scan.append(p)
            elif p.is_dir():
                for root, _, files in os.walk(p):
                    for file in files:
                        ext = Path(file).suffix.lower()
                        if ext in [".md", ".rs"]:
                            files_to_scan.append(Path(root) / file)
    else:
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
                        files_to_scan.append(Path(root) / file)

    total_violations = []
    files_with_violations = []

    for file_path in files_to_scan:
        v = scan_file(file_path, repo_root, is_custom_path=bool(custom_paths))
        if v:
            total_violations.extend(v)
            files_with_violations.append(file_path)

    if not total_violations:
        print(f"PASS: Scanned {len(files_to_scan)} files.")
        print("Zero synthetic linguistic attractors or leaked hardware buzzwords found.")
        sys.exit(0)

    print(f"Found {len(total_violations)} attractor violation(s) across {len(files_with_violations)} file(s):\n")
    for idx, err in enumerate(total_violations, 1):
        print(f"[{idx}] {err['file']}:{err['line']}")
        print(f"    Violation : {err['name']}")
        print(f"    Matched   : \"{err['matched']}\"")
        print(f"    Line      : {err['snippet']}")
        print(f"    Remedy    : {err['suggestion']}\n")

    if fix_mode:
        print(f"=== Applying Automated Cleaning to {len(files_with_violations)} file(s) ===")
        total_substitutions = 0
        for fpath in files_with_violations:
            try:
                with open(fpath, "r", encoding="utf-8") as f:
                    old_text = f.read()
                new_text, count = apply_replacements(old_text)
                if count > 0:
                    with open(fpath, "w", encoding="utf-8") as f:
                        f.write(new_text)
                    rel_p = fpath.relative_to(repo_root) if fpath.is_relative_to(repo_root) else fpath
                    print(f"  CLEANED: {rel_p} ({count} replacement(s))")
                    total_substitutions += count
            except Exception as e:
                print(f"  ERROR cleaning {fpath}: {e}")

        print(f"\nSuccessfully cleaned {total_substitutions} attractor(s) across {len(files_with_violations)} file(s).")
        sys.exit(0)
    else:
        print("Action Required: Run with '--fix' to automatically clean attractors, or edit files manually.")
        sys.exit(1)


if __name__ == "__main__":
    main()
