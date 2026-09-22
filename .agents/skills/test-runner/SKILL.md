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

## 2. Directory Structure for Snapshots (`.test_results/`)

Test executions maintain rotating state to detect regressions across runs:
- `.test_results/latest/<suite_name>.json`: Machine-readable results of the most recent run.
- `.test_results/previous/<suite_name>.json`: Results from the previous baseline for differential comparison.
- `.test_results/summary.json`: Global repository-wide pass/fail matrix, totals, and percentages.

---

## 3. Invocation & Zero-Parameter Execution

When invoked without parameters (e.g. `/test-runner`):
1. Runs the quick regression gate:
   ```powershell
   cargo run -p test_runner -- --diff
   ```
2. Displays the tabular summary across opcodes and subsystems:
   ```powershell
   cargo run -p test_runner -- --summary
   ```
3. If no snapshots exist, runs a Tier 1 + Tier 2 smoke sweep to establish the initial baseline snapshot.

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
- **Action Required:** `[CLEAN | REGRESSIONS NEED TRIAGE]`
