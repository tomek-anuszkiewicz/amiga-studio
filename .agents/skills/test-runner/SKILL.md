---
name: test-runner
description: Execute test suites across tiers, snapshot results to disk, and track automated regression diffs.
---

# Recipe: Test Runner & Regression Tracker

This skill standardizes test execution across the 4 testing tiers, persists execution snapshots into `.agent/tmp/singlestep/`, and automatically computes regression diffs (🔴 newly failing tests vs 🟢 newly passing tests).

---

## 1. Testing Tiers & Commands

| Tier | Scope | Target Command | Typical Runtime |
| :---: | :--- | :--- | :---: |
| **Tier 1** | Unit tests (isolated crates) | `python tools/verification/verify.py test rust unit` | $< 2$s |
| **Tier 2** | Machine loop integration | `python tools/verification/verify.py test rust integration` | $3$–$15$s |
| **Tier 3** | SingleStep CPU silicon vectors | `python tools/verification/verify.py test runner singlestep` | $6$–$15$s |
| **Tier 4** | vAmigaTS whole-machine captures | `python tools/verification/verify.py runner vamiga category <cat>` | $10$–$60$s |
| **Arch** | Architecture rules & gates | `python tools/verification/verify.py check architecture` | $< 2$s |

---

## Focused Defect Reproduction

Follow [repro-first.md](../../rules/repro-first.md) when repairing behavior:

1. Identify an existing isolated regression or add a focused test in the owning suite. For Rust, use `crates/<crate>/tests/`; Python tool contracts use the owning tool's `tests/` directory, listed in [tools/verification/README.md](../../../tools/verification/README.md).
2. Run it before the repair and confirm the expected failure, rather than a missing fixture or unrelated setup error:
   ```powershell
   python tools/verification/verify.py test rust crate <crate> suite <suite> case <filter>
   python tools/verification/verify.py test python directory <tool_directory>/tests file <fixture_file.py>
   ```
3. Repair the demonstrated mechanism, rerun the focused case, and retain its regression protection.
4. For ordinary task completion without a commit, run relevant focused tests and checks. Use `python tools/verification/verify.py test python` to select Python tool contracts affected by staged, unstaged and nonignored new paths, including shared dependencies. Unchanged tool suites are skipped; an explicit directory selector remains available for diagnosis.
5. Before an authorized commit, run the completion verifier on the intended working-tree source; staging is optional:
   ```powershell
   python tools/verification/verify.py commit
   ```
   The commit profile runs standard suites and all Python contracts, without full DMA/SingleStep suites.
   Full `test runner dma` or `test runner singlestep` runs remain available for explicit diagnosis or user requests.
   After the commit verification above, run `python tools/verification/verify.py milestone` for stage completion.
   Milestone runs both full CPU suites and the language audit, including on clean trees, without repeating other commit checks. Use `--plan` to inspect either set.
   Focused commands validate ordinary tasks; they do not replace the full pre-commit profile.

## Background Checks and Result Collection

- Validate coherent changes with targeted tests. Do not run tests after every edit or repeat passed checks without affected input changes, failures or unresolved concerns. Collect full results only at the required commit or milestone trigger.
- Start long checks with a short supported initial yield, retain returned session IDs, and resume only running sessions when their result is needed.
- Parallelize independent checks; Cargo commands sharing a target directory may serialize. Do not edit the validated source snapshot while a check runs.
- Keep successful output concise with a single quiet-flag placement, preserving exit codes and failure diagnostics. Report checks not run separately from passing results.

## 2. Directory Structure for Snapshots (`.agent/tmp/singlestep/`)

Test executions maintain rotating state to detect regressions across runs:
- `.agent/tmp/singlestep/latest/<suite_name>.json`: Machine-readable results of the most recent run.
- `.agent/tmp/singlestep/previous/<suite_name>.json`: Results from the previous baseline for differential comparison.
- `.agent/tmp/singlestep/summary.json`: Global repository-wide pass/fail matrix, totals, and percentages.
- `.agent/tmp/singlestep/summary.md`: Automatically generated readable summary and all failing cases.
- `.agent/tmp/singlestep/diff.md`: Automatically generated regressions, improvements, and missing baselines.

---

## 3. Invocation & Zero-Parameter Execution

When invoked without parameters (e.g. `$test-runner`):
1. Read `.agent/tmp/singlestep/diff.md` when present to inspect saved regressions and improvements.
2. Read `.agent/tmp/singlestep/summary.md` when present for saved suite statistics and failing cases. Report the freshness of this evidence.
3. Identify the current task's changed behavior and select its owning crate,
   integration suite or Python directory. A dirty tree or missing snapshots
   alone do not justify broad unit and integration runs:
   ```powershell
   python tools/verification/verify.py test rust crate <crate> suite <suite>
   python tools/verification/verify.py test python directory <tool_directory>/tests
   ```
   Unit and machine integration tests do not create SingleStep snapshots. Full SingleStep tests run automatically at milestones; standalone runs require explicit diagnosis or a user request. Their results automatically generate the reports.

---

## 4. Automated Regression Telemetry

When comparing the current run against the previous baseline:
- 🔴 **Regressions (Broken):** Tests that previously PASSED but now FAIL.
  ```text
  ⚠️  [REGRESSION DETECTED] Suite 'Real68k::ADD.b': 1 test(s) that previously PASSED now FAILED!
     🔴 Broken: "001 [ADD.b D1, (d8, A4, Xn)] d334"
  ```
- 🟢 **Improvements (Fixed):** Tests that previously FAILED but now PASS.
  ```text
  🎉 [PROGRESS / FIX] Suite 'Real68k::MOVEA.w': 1 test(s) that previously FAILED now PASSED!
     🟢 Fixed:  "3c7a [MOVEA.w (d16, PC), A6] 19"
  ```

---

## 5. Output Contract

Conclude every test run with a concise executive summary:
- **Suite Run:** `<tier_or_suite_name>`
- **Total Tests:** `<total_run>`
- **Pass Rate:** `<passed> / <total> (<percentage>%)`
- **Diff vs Baseline:** `+<fixed> improved, -<broken> regressions`
- **Regression Alert:** Identify newly failing tests immediately.
- **Action Required:** `[CLEAN | REGRESSIONS NEED TRIAGE]`
