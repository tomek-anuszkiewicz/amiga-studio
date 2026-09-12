---
name: attractor-discipline
description: >-
  Audit, detect, and automatically clean synthetic linguistic attractors, high-register academic jargon, theatrical testing vocabulary, and heading slogans across documentation, rules, skills, and code per .agents/rules/attractor-discipline.md.
---

# Recipe: Linguistic Attractor & Vocabulary Discipline Audit

This skill provides operational procedures and automated tooling to enforce [`.agents/rules/attractor-discipline.md`](../../rules/attractor-discipline.md) across the repository.

---

## 1. When to Trigger This Skill

- **Pre-Commit Verification:** Run before committing non-trivial documentation or code modifications to verify zero quarantined terms were introduced.
- **Code & Architecture Reviews:** Run during `/code-review` to audit pull requests, feature branches, or milestone completions.
- **Documentation Refactoring:** Run after drafting or updating specifications in `Obsidian/Amiga/Design/` or operating rules in `.agents/rules/`.

---

## 2. Tooling & Infrastructure

- **Packaged Linter:** [`scripts/lint_attractors.py`](scripts/lint_attractors.py)
  - Zero third-party dependencies (Python 3 standard library: `os`, `re`, `sys`, `pathlib`).
  - Scans Markdown (`.md`) and Rust (`.rs`) source files across `.agents/`, `Obsidian/Amiga/Design/`, and `crates/`.
  - Supports targeted file/folder scans, dry runs, and automated in-place fixes (`--fix`).
- **Root Forwarder:** [`scripts/lint_attractors.py`](../../../scripts/lint_attractors.py) (convenience forwarder for global CLI workflows).
- **Automated Architecture Test:** `test_zero_synthetic_attractors` in `crates/test_runner/tests/test_architecture_rules.rs` (enforces zero violations in `cargo test`).

---

## 3. Step-by-Step Execution Workflows

### Workflow A: Repository-Wide Validation (Standard Gate)
Run the linter in validation mode:
```powershell
python .agents/skills/attractor-discipline/scripts/lint_attractors.py
```
Or via the root wrapper:
```powershell
python scripts/lint_attractors.py
```
- **Exit Code 0:** All scanned files are clean.
- **Exit Code 1:** Attractor violations detected with line numbers, offending matched text, and grounded replacement suggestions.

### Workflow B: Automated In-Place Cleaning (`--fix`)
When violations are detected across one or more files, run the automated cleaner:
```powershell
python .agents/skills/attractor-discipline/scripts/lint_attractors.py --fix
```
- Automatically substitutes quarantined terms with approved grounded replacements while preserving casing (uppercase, title case, lowercase).
- Fixes heading sloganization patterns in Markdown headers.

### Workflow C: Targeted Path Scanning
To audit a specific modified file or directory without scanning the entire workspace:
```powershell
python .agents/skills/attractor-discipline/scripts/lint_attractors.py path/to/document.md
```

### Workflow D: Whitelisting & Exclusions
Historical engineering narratives (`DIARY.md`), automated test harnesses (`test_architecture_rules.rs`), and the rule/linter definition files themselves are whitelisted in `lint_attractors.py` to allow discussing the rules without self-triggering.
