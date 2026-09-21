---
name: audit-code-quality
description: Full-workspace code quality audit covering dead code, test zombies, visibility leaks, SRP, inlining, test parity, and on-demand manual diff inspection
---

# Workflow: Architectural Code Quality Audit & Pruning

Use this workflow to execute a comprehensive, on-demand code quality audit across the entire Rust workspace, triage dead code and test zombies, enforce the Principle of Minimum Visibility, audit inlining annotations, eliminate anti-patterns, verify external test suite parity, and inspect git diffs for the non-automatable checks not covered by Clippy or `pre_flight.py`.

---

## 1. Zero-Parameter Run (`/audit-code-quality`)
When invoked without parameters:
1. **Execute Universal Code Quality Audit:**
   ```powershell
   python tools/harness/audit_code_quality.py --all
   ```
2. **Execute Workspace Compiler & Clippy Lints:**
   ```powershell
   cargo clippy --workspace --all-targets
   ```
3. **Execute Pre-Flight Quality Gate (includes automated architecture tests):**
   ```powershell
   python tools/harness/pre_flight.py
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
- **Audit Compound Condition Soup & Boolean Clarity:**
   ```powershell
   python tools/harness/audit_code_quality.py --conditions
   ```
- **Audit Method Naming & Accessor Conventions:**
   ```powershell
   python tools/harness/audit_code_quality.py --accessors
   ```
- **Audit Workspace Clippy & Compiler Invariants:**
   ```powershell
   cargo clippy --workspace --all-targets
   ```
- **Audit Automated Architecture Rules (inlining, macros, const-generics, test layouts, file sizes):**
   ```powershell
   cargo test -p test_runner --test test_architecture_rules -- --quiet
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
4. **Inlining Alignment:** Add `#[inline(always)]` to hot leaf ALU/CCR functions and `#[inline(never)]` to cold exception handlers per [`.agents/rules/method-inlining.md`](../rules/method-inlining.md).
5. **Anti-Pattern Elimination:** Replace `macro_rules!` and const-generic templates with concrete functions (enforced via `test_architecture_rules`).
6. **Test Organization:** Move inline tests from `src/` to `tests/`; maintain 1:1 file parity per [`.agents/rules/unit-testing-policy.md`](../rules/unit-testing-policy.md).
7. **Accessors & Collection Slices:** Standard getters match field name (no `get_` prefix); boolean getters use `is_`/`has_`/`can_`; setters use `set_<field>`; collection getters return `&[T]` not `&Vec<T>`.
8. **Compiler-Grade AST Invariants:** Borrow views (`&[T]`, `&str`), `Default` delegation, zero unwraps/panics in production code, mandatory `Debug` derives — all verified via `cargo clippy --workspace --all-targets` and `check_clippy_invariants` in `pre_flight.py`.

---

## 4. Manual Diff Checklist (Non-Automated)

Run when explicitly requested or before a major architectural branch merge.
Automated gates cover ~85% of checks — these 7 require direct `git diff` inspection.

```powershell
git diff
```

### A. Endianness & Systems Safety
- [ ] All multi-byte guest values use explicit `from_be_bytes` / `to_be_bytes` — no implicit host-endian reinterpretation.
- [ ] ALU and cycle counter operations use wrapping arithmetic (`wrapping_add`, `wrapping_sub`) — no silent overflow.

> `transmute_ptr_to_ptr`, `cast_ptr_alignment`, `Rc`/`RefCell`/`Arc`/`Mutex` are already `deny` in `Cargo.toml` — Clippy catches them at compile time.

### B. WASM Portability (Core Crates Only)
- [ ] Zero `std::time::Instant`, `std::thread`, `std::fs` in `crates/cpu/`, `crates/memory_bus/`, `crates/agnus/`, `crates/denise/`, `crates/paula/`, `crates/cia/`.
  *(Clippy catches this only when cross-compiling for `wasm32`.)*

### C. Architecture Boundaries
- [ ] New subsystem state structs implement `serde::Serialize` and `serde::Deserialize` (save-state contract).

### D. Defect Retrospection (Bug Fixes & Refactors Only)
- [ ] Root cause documented: *"Why did this happen at the hardware model level?"*
- [ ] Regression test added covering the exact failure mode — committed as a permanent sentinel.
- [ ] Institutional prevention evaluated: does a Clippy lint, architecture test, or design doc update prevent this class of defect from recurring?

---

## 5. The Verbal Double-Check (Self-Audit & Heuristic Verification)

Beyond mechanical script passes, explicitly review the **5 Non-Negotiable Conscience Questions**:
1. 🧠 **Spec Freshness Review:** Did code refactoring or pruning introduce behavior changes not yet updated in `Obsidian/Amiga/Design/*.md`?
2. 🚫 **Anti-Nudge Review (`structural-root-cause.md`):** Are all clock delays, cycle counts, and beam offsets silicon-verified rather than empirical $\pm 1$ / $\pm 2$ symptom nudges?
3. 🔬 **Assertion Density & Genuine Test Review (`unit-testing-policy.md`):** Do unit tests genuinely verify chip behavior and state changes, or do they only assert trivial boilerplate?
4. 📢 **Spec Conflict Escalation (`spec-compliance.md`):** Were any conflicts between reference test suites and internal design specs escalated to the user before changing code?
5. 🧹 **Clean-Break Refactoring (`clean-break-refactoring.md`):** Were old methods, legacy aliases, and temporary shims completely deleted rather than left behind?

---

## 6. Output Contract
Conclude with the standardized summary report:
```markdown
### 🛡️ Code Quality Audit & Pruning Report
- **Scope Scanned:** Workspace (all crates)
- **Dead Code Pruned:** <count> symbols
- **Test-Only Zombies Handled:** <count> retained (Host I/O) / <count> pruned
- **Visibility Demoted:** <count> symbols (`pub` -> `pub(crate)` / private)
- **Condition Soup Anomalies:** <count> compound conditions
- **Method Naming & Accessor Conventions:** [PASS | <count> violations]
- **Workspace Clippy & Compiler Lints:** [PASS | <count> violations]
- **Architecture Rules (`test_architecture_rules`):** [PASS | 21/21 tests passed]
- **Verbal Double-Check Conscience Review:** [CONFIRMED - 5/5 heuristics verified]
- **Verification:** `pre_flight.py` (PASS), `cargo clippy` (PASS)
```
