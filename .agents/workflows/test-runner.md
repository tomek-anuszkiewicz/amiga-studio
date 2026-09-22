---
name: test-runner
description: Run test suites across tiers, snapshot results to .test_results, and report regression diffs
---

# Workflow: Test Runner & Regression Diffing

Use this workflow to execute tests, establish result snapshots, and check for regressions across runs.

---

## 1. Zero-Parameter Run
When invoked without parameters (typing `/test-runner`):
1. **Check Latest Differential Status:**
   ```powershell
   cargo run -p test_runner -- --diff
   cargo run -p test_runner -- --summary
   ```
2. **Execute Fast Tier 1 + 2 Verification (if dirty tree or no baseline):**
   ```powershell
   python tools/harness/run_tests.py --unit
   cargo test -p machine_loop
   ```

---

## 2. Targeted Execution Commands
- **Run All Unit Tests:** `python tools/harness/run_tests.py --unit`
- **Run Machine Loop Integration:** `cargo test -p machine_loop`
- **Run Single-Step Silicon Vectors:** `cargo test -p test_runner --test test_singlestep`
- **Run vAmigaTS Whole-Machine Suite:** `target/release/test_runner.exe vamiga --category <cat>`
- **Run Automated Architecture Rules:** `cargo test -p test_runner --test test_architecture_rules`

---

## 3. Output Contract
- **Executed Suite:** `<tier_or_suite>`
- **Total:** `<N>` | **Passed:** `<P>` | **Failed:** `<F>`
- **Diff:** `+<improvements> fixed, -<regressions> broken`
- **Regression Alert:** Highlight any red regressions immediately with failing test identifiers.
