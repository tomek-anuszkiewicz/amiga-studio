---
name: test-runner
description: Execute test suites across tiers, snapshot results to disk, and track automated regression diffs.
---

# Recipe: Test Runner & Regression Tracker

Use this skill to select relevant existing tests and report their evidence. The aggregate SingleStep test saves snapshots and finalizes regression reports once after group execution in `.agent/tmp/singlestep/`; ordinary crate tests do not create those reports. Direct Cargo execution also finalizes reports inside the aggregate test; the [verification reference](../../../tools/verification/README.md#saved-singlestep-reports) owns group selection and report isolation.

---

## 1. Testing Tiers & Commands

| Tier | Scope | Target Command | Typical Runtime |
| :---: | :--- | :--- | :---: |
| **Tier 1** | Unit tests (isolated crates) | `python tools/verification/verify.py test rust unit` | $< 2$s |
| **Tier 2** | Machine loop integration | `python tools/verification/verify.py test rust integration` | $3$–$15$s |
| **Tier 3** | SingleStep CPU reference vectors | `python tools/verification/verify.py test runner singlestep` | Depends on build cache, host and report history |
| **Tier 4** | vAmigaTS whole-machine captures | Unavailable; requires a future capture execution entry point | Not measured |
| **Arch** | Architecture rules & gates | `python tools/verification/verify.py check architecture` | $< 2$s |

---

## Focused Defect Reproduction

Follow [defect resolution](../../rules/structural-root-cause.md#defect-resolution) when repairing behavior:

1. Identify existing regression protection, including authoritative vectors. Extend it or add a focused test only for a demonstrated gap. State the checked behavior and expected-result source under [unit-testing-policy.md](../../rules/unit-testing-policy.md). For Rust, use `crates/<crate>/tests/`; Python tool contracts use the owning tool's `tests/` directory, listed in [tools/verification/README.md](../../../tools/verification/README.md).
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
   The commit profile runs standard suites, RAG and verification contracts, and Bootstrap contracts when Git reports changes under `tools/bootstrap/`, without full SingleStep.
   Full `test runner singlestep` runs remain available for explicit diagnosis or user requests.
   After the commit verification above, run `python tools/verification/verify.py milestone` for stage completion.
   Milestone runs full SingleStep and the language audit, including on clean trees, without repeating other commit checks. Use `--plan` to inspect either set.
   Focused commands validate ordinary tasks; they do not replace the full pre-commit profile.

## Background Checks and Result Collection

- Validate coherent changes with targeted tests. Do not run tests after every edit or repeat passed checks without affected input changes, failures or unresolved concerns. Collect full results only at the required commit or milestone trigger.
- Start long checks with a short supported initial yield, retain returned session IDs, and resume only running sessions when their result is needed.
- Parallelize independent checks; Cargo commands sharing a target directory may serialize. Do not edit the validated source snapshot while a check runs.
- Keep successful output concise with a single quiet-flag placement, preserving exit codes and failure diagnostics. Report checks not run separately from passing results.

## 2. Directory Structure for Snapshots (`.agent/tmp/singlestep/`)

SingleStep executions maintain rotating state to detect regressions across runs:
- `.agent/tmp/singlestep/latest/<suite_name>.json`: Machine-readable results of the most recent run.
- `.agent/tmp/singlestep/previous/<suite_name>.json`: Results from the previous baseline for differential comparison.
- `.agent/tmp/singlestep/summary.json`: Saved SingleStep suite totals and percentages, which may include results from earlier runs.
- `.agent/tmp/singlestep/summary.md`: Automatically generated readable summary and all failing cases.
- `.agent/tmp/singlestep/diff.md`: Automatically generated regressions, improvements, and missing baselines.

Dedicated commands use optimized `verification` artifacts with debug assertions
and overflow checks preserved. An explicit generic Rust `test_singlestep` target
uses unoptimized `test` artifacts. Both execute one aggregate Rust test which finalizes reports once before its
final assertion, including group failures. `case` selects internal group names;
libtest sees one test. Direct Cargo group filtering uses `SINGLESTEP_CASE`. Current group completion and report/vector counts are separate from saved
history totals; unmatched or interrupted execution is incomplete. Full CMP and
filtered CMPM have distinct identities. Reports missing required fields fail to
load; explicit empty selections and different sample limits are not comparable baselines.

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

Report the executed command, result, fixture scope and material limitations.
For SingleStep, sampling changes input count only: every executed vector compares
state, instruction clocks and timed bus callbacks. Older saved results from weaker
verification are not a comparable regression baseline.
Use the [CPU verification specification](../../../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md)
for current comparison limits. Report saved diffs only when their input scope and
freshness are comparable; do not present old suite totals as the current run.
