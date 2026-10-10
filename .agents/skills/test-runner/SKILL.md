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
| **Tier 4** | vAmigaTS whole-machine captures | `python tools/verification/verify.py runner vamiga -- --category <cat>` | $10$–$60$s |
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
4. Run the completion verifier on the intended working-tree source; staging is optional:
   ```powershell
   python tools/verification/verify.py commit
   ```
   The verifier always runs standard suites; only DMA/SingleStep depend on changed sources.
   Use `test runner dma` or `test runner singlestep` for an independent full suite.
   Append `dma` or `singlestep` to `commit` or `milestone` to force inclusion.
   After the commit verification above, run `python tools/verification/verify.py milestone` for stage completion.
   The milestone set does not repeat commit checks; use `--plan` to inspect either set.
   Individual suite commands below are for development and diagnosis, not alternative completion gates.

## Background Checks and Result Collection

- Use targeted filters during iteration; collect required full results before completion.
- Start long checks with a short supported initial yield, retain returned session IDs, and resume only running sessions when their result is needed.
- Parallelize independent checks; Cargo commands sharing a target directory may serialize. Do not edit the validated source snapshot while a check runs.
- Keep successful output concise with a single quiet-flag placement, preserving exit codes and failure diagnostics. Report checks not run separately from passing results.

## 2. Directory Structure for Snapshots (`.agent/tmp/singlestep/`)

Test executions maintain rotating state to detect regressions across runs:
- `.agent/tmp/singlestep/latest/<suite_name>.json`: Machine-readable results of the most recent run.
- `.agent/tmp/singlestep/previous/<suite_name>.json`: Results from the previous baseline for differential comparison.
- `.agent/tmp/singlestep/summary.json`: Global repository-wide pass/fail matrix, totals, and percentages.

---

## 3. Invocation & Zero-Parameter Execution

When invoked without parameters (e.g. `$test-runner`):
1. Runs the quick regression gate:
   ```powershell
   python tools/verification/verify.py runner diff
   ```
2. Displays the tabular summary across opcodes and subsystems:
   ```powershell
   python tools/verification/verify.py runner summary
   ```
3. If the working tree is dirty or no snapshots exist, run Tier 1 and machine
   integration verification:
   ```powershell
   python tools/verification/verify.py test rust unit
   python tools/verification/verify.py test rust crate machine_loop
   ```
   Establish an initial snapshot when none exists; report freshness of existing results.

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
