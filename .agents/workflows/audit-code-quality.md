---
name: audit-code-quality
description: Run full-workspace architectural code quality audit covering dead code, test zombies, visibility leaks, SRP, inlining, and test parity
---

# Workflow: Architectural Code Quality Audit & Pruning

Use this workflow to execute a comprehensive, on-demand code quality audit across the entire Rust workspace, triage dead code and test zombies, enforce the Principle of Minimum Visibility, audit inlining annotations, eliminate anti-patterns, and verify external test suite parity.

---

## 1. Zero-Parameter Run (`/audit-code-quality`)
When invoked without parameters:
1. **Execute Universal Code Quality Audit:**
   ```powershell
   python tools/harness/audit_code_quality.py --all
   ```
2. **Execute Pre-Flight Quality Gate:**
   ```powershell
   python tools/harness/pre_flight.py
   ```
3. **Execute Architecture Rules:**
   ```powershell
   cargo test -p test_runner --test test_architecture_rules -- --quiet
   ```

---

## 2. Targeted Audit Commands
- **Audit Dead Code & Test Zombies:**
   ```powershell
   python tools/harness/audit_code_quality.py --dead-code
   ```
- **Audit Visibility Leaks (`pub` vs `pub(crate)` vs private):**
   ```powershell
   python tools/harness/audit_code_quality.py --visibility
   ```
- **Audit SRP & 800-Line File Ceilings:**
   ```powershell
   python tools/harness/audit_code_quality.py --srp
   ```
- **Audit Method Inlining Guidelines (`#[inline(always)]` / `#[inline(never)]`):**
   ```powershell
   python tools/harness/audit_code_quality.py --inlining
   ```
- **Audit Prohibited Macros & Const-Generics:**
   ```powershell
   python tools/harness/audit_code_quality.py --antipatterns
   ```
- **Audit Dedicated External Test Suites & Submodule Parity:**
   ```powershell
   python tools/harness/audit_code_quality.py --tests
   ```
- **Audit Specific Crate:**
   ```powershell
   python tools/harness/audit_code_quality.py --all --crate paula
   ```

---

## 3. Remediation Procedure
Follow the detailed playbooks in [`.agents/skills/audit-code-quality/SKILL.md`](../skills/audit-code-quality/SKILL.md):
1. **Test-Only Zombies:** Distinguish external Host I/O boundaries from dead internal scaffolding. Prune dead symbols and orphaned test cases.
2. **Visibility Demotion:** Demote over-exposed symbols to `pub(crate)` or private `fn`.
3. **SRP Decompositions:** Decompose oversized files (> 800 lines) into submodules using [`.agents/skills/refactor-split-module/SKILL.md`](../skills/refactor-split-module/SKILL.md).
4. **Inlining Alignment:** Add mandatory `#[inline(always)]` to hot leaf ALU/CCR functions and `#[inline(never)]` to cold exception trigger handlers per [`.agents/rules/method-inlining.md`](../rules/method-inlining.md).
5. **Anti-Pattern Elimination:** Replace any ad-hoc macros (`macro_rules!`) and const-generic templates with concrete, explicit functions.
6. **Test Organization:** Move any inline tests in `src/` to `tests/` and maintain 1:1 test file parity in multi-module crates per [`.agents/rules/unit-testing-policy.md`](../rules/unit-testing-policy.md).

---

## 4. The Verbal Double-Check (Self-Audit & Heuristic Verification)

Beyond mechanical script passes, explicitly review the **5 Non-Negotiable Conscience Questions**:
1. 🧠 **Spec Freshness Review:** Did code refactoring or pruning introduce behavior changes not yet updated in `Obsidian/Amiga/Design/*.md`?
2. 🚫 **Anti-Nudge Review (`structural-root-cause.md`):** Are all clock delays, cycle counts, and beam offsets silicon-verified rather than empirical $\pm 1$ / $\pm 2$ symptom nudges?
3. 🔬 **Assertion Density & Genuine Test Review (`unit-testing-policy.md`):** Do unit tests genuinely verify chip behavior and state changes, or do they only assert trivial boilerplate?
4. 📢 **Spec Conflict Escalation (`spec-compliance.md`):** Were any conflicts between reference test suites and internal design specs escalated to the user before changing code?
5. 🧹 **Clean-Break Refactoring (`clean-break-refactoring.md`):** Were old methods, legacy aliases, and temporary shims completely deleted rather than left behind?

---

## 5. Output Contract
Conclude with the standardized summary report:
```markdown
### 🛡️ Code Quality Audit & Pruning Report
- **Scope Scanned:** Workspace (all crates)
- **Dead Code Pruned:** <count> symbols
- **Test-Only Zombies Handled:** <count> retained (Host I/O) / <count> pruned
- **Visibility Demoted:** <count> symbols (`pub` -> `pub(crate)` / private)
- **SRP / Cohesion Decompositions:** <count> files/structs
- **Inlining Guidelines:** [PASS | <count> anomalies]
- **Anti-Pattern Prohibitions:** [PASS | <count> violations]
- **External Test Suites & Parity:** [PASS | <count> issues]
- **Verbal Double-Check Conscience Review:** [CONFIRMED - 5/5 heuristics verified]
- **Verification:** `pre_flight.py` (PASS), `test_architecture_rules` (PASS)
```
