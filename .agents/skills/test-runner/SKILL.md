---
name: test-runner
description: Execute test suites across tiers, snapshot results to disk, and track automated regression diffs.
---

# Recipe: Test Runner & Regression Tracker

This skill standardizes test execution across the 4 testing tiers, persists execution snapshots into `.test_results/`, and automatically computes regression diffs (🔴 newly failing tests vs 🟢 newly passing tests).

---

## 1. Testing Tiers & Commands

| Tier | Scope | Target Command | Typical Runtime |
| :---: | :--- | :--- | :---: |
| **Tier 1** | Unit tests (isolated crates) | `python tools/harness/run_tests.py --unit` | $< 2$s |
| **Tier 2** | Machine loop integration | `python tools/harness/run_tests.py --integration` | $3$–$15$s |
| **Tier 3** | SingleStep CPU silicon vectors | `cargo test -p test_runner --test test_singlestep` | $6$–$15$s |
| **Tier 4** | vAmigaTS whole-machine captures | `target/release/test_runner.exe vamiga --category <cat>` | $10$–$60$s |
| **Arch** | Architecture rules & gates | `cargo test -p test_runner --test test_architecture_rules` | $< 2$s |

---

## Focused Defect Reproduction

Follow [repro-first.md](../../rules/repro-first.md) when repairing behavior:

1. Identify an existing isolated regression or add a focused test in the owning suite. For Rust, use `crates/<crate>/tests/`; harness fixtures use `tests/`.
2. Run it before the repair and confirm the expected failure, rather than a missing fixture or unrelated setup error:
   ```powershell
   cargo test -p <crate> --test <suite> -- <filter>
   python -m unittest discover -s tests -p <fixture_file.py> -q
   ```
3. Repair the demonstrated mechanism, rerun the focused case, and retain its regression protection.
4. Run adjacent domain tests and the per-commit gates. Before completing CPU changes, run full silicon vectors; CPU/bus changes also require full Cartesian DMA verification:
   ```powershell
   $env:SINGLESTEP_FULL = "1"
   cargo test -p test_runner --test test_singlestep -- --quiet
   cargo test -p test_runner --test test_dma_cartesian -- --quiet
   ```
   GUI interaction regressions use `cargo test -p gui --test test_interactions`.

## Background Checks and Result Collection

- Use targeted filters during iteration; collect required full results before completion.
- Start long checks with a short supported initial yield, retain returned session IDs, and resume only running sessions when their result is needed.
- Parallelize independent checks; Cargo commands sharing a target directory may serialize. Do not edit the validated source snapshot while a check runs.
- Keep successful output concise with a single quiet-flag placement, preserving exit codes and failure diagnostics. Report checks not run separately from passing results.

## 2. Directory Structure for Snapshots (`.test_results/`)

Test executions maintain rotating state to detect regressions across runs:
- `.test_results/latest/<suite_name>.json`: Machine-readable results of the most recent run.
- `.test_results/previous/<suite_name>.json`: Results from the previous baseline for differential comparison.
- `.test_results/summary.json`: Global repository-wide pass/fail matrix, totals, and percentages.

---

## 3. Invocation & Zero-Parameter Execution

When invoked without parameters (e.g. `$test-runner`):
1. Runs the quick regression gate:
   ```powershell
   cargo run -p test_runner -- --diff
   ```
2. Displays the tabular summary across opcodes and subsystems:
   ```powershell
   cargo run -p test_runner -- --summary
   ```
3. If the working tree is dirty or no snapshots exist, run Tier 1 and machine
   integration verification:
   ```powershell
   python tools/harness/run_tests.py --unit
   cargo test -p machine_loop
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
