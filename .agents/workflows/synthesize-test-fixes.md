---
name: synthesize-test-fixes
description: Audit recent test patches across git history, build 3-column diagnostic matrix, and synthesize scattered fixes into an upstream root cause
---

# Workflow: Synthesize Test Fixes

Use this workflow to consolidate fragmented, local test fixes across multiple commits into a single upstream architectural solution.

---

## 1. Zero-Parameter Run
When invoked without parameters (typing `/synthesize-test-fixes`):
1. **Analyze Working State:**
   - Checks active uncommitted changes via `git diff`.
   - If clean, inspects the last 5 commits:
     ```powershell
     git log -n 5 --oneline
     git diff HEAD~5..HEAD
     ```
2. **Build the 3-Column Diagnostic Matrix:**
   - Maps each change to: `[Test / Symptom] | [Patch Location] | [Mechanism Applied]`.
3. **Propose the Upstream Common Denominator:**
   - Identifies the shared root cause in the central substrate (`memory_bus`, `machine_loop`, `agnus`).
   - Outlines which local patches will be eradicated.

---

## 2. Execution Runbook
Follow the operational procedure in [`synthesize-test-fixes` skill](../skills/synthesize-test-fixes/SKILL.md):

1. **Extraction:** Gather diffs across the target sprint or commit range.
2. **Tabulation:** Construct the 3-column diagnostic table.
3. **Root-Cause Synthesis:** Identify the single physical hardware law explaining the disparate symptoms.
4. **Upstream Refactoring:** Implement the unified rule in the core substrate and delete all downstream workarounds.
5. **Regression Verification:** Run `/test-runner` to confirm zero regressions.

---

## 3. Output Contract
- **Analyzed Scope:** `<commit_range_or_working_tree>`
- **3-Column Diagnostic Table:** Formatted breakdown.
- **Common Denominator:** Physical circuit or timing rule identified.
- **Patches Eradicated:** List of cleaned files and deleted ad-hoc logic.
- **Regression Verdict:** `/test-runner` pass/fail delta.
