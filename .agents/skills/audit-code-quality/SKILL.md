---
name: audit-code-quality
description: Interpret code-quality verification results and review Rust architecture, dead-code candidates, visibility, cohesion, and requested diffs; guide scoped remediation.
---

# Recipe: Architectural Code Quality Auditor & Pruning Playbook

This skill interprets deterministic verification results and reviews code-quality decisions that require source and architectural context. Automatic audits belong to [tools/verification/verify.py](../../../tools/verification/verify.py); this skill does not launch the Python auditor or repeat completion verification. Apply fixes only within the requested scope.

---

## 1. When to Trigger This Skill

- **Requested Audit or Major Branch Merge:** Follow [git-commits.md](../../rules/git-commits.md). At milestone closure, use existing verification results when this review is requested; milestone verification alone does not invoke this skill.
- **Review Preparation:** When requested, assess code-quality findings before the architectural review in [`code-review`](../code-review/SKILL.md).
- **Refactoring Sprints:** Run whenever restructuring crate boundaries, decomposing oversized files, or auditing information hiding.

---

## 2. The Six Code Quality Pillars

### Pillar 1: Dead Code & Test-Only Zombies
1. **Completely Dead Symbols (💀):**
   - Declared symbols with **zero callers anywhere** across `crates/*/src/` and `crates/*/tests/`.
   - *Remediation:* Treat scanner results as candidates. Confirm actual symbol identity, callers, re-exports, conditional compilation and external API obligations before proposing deletion.
2. **Test-Only Zombie Code (🧟):**
   - Symbols with **zero callers in production code (`crates/*/src/`)**, but referenced in unit/integration tests (`crates/*/tests/`).
   - *Origin:* A helper was written, tested, and subsequently bypassed in the main machine loop or bus. The test kept passing, creating the false impression that the code was active.
   - *Remediation:* Triage against external Host I/O boundaries (see Section 4). If internal scaffolding, purge both the method and its orphaned test.
3. **Internal Unused Symbols (The "Visibility Downgrade" Indicator):**
   - In Rust library crates (`[lib]`), `rustc` treats all `pub` items as public API and suppresses `dead_code` warnings.
   - Demoting `pub` to `pub(crate)` unmasks `rustc`'s built-in reachability analysis via `python tools/verification/verify.py check build`.

### Pillar 2: Principle of Minimum Visibility (Least Privilege)
- **Over-Exposed `pub` Items:** Symbols declared `pub` whose callers are strictly confined to their own crate. Demote to `pub(crate)`.
- **Over-Exposed `pub(crate)` / `pub` Items:** Symbols whose callers reside strictly within their defining file. Demote to private `fn`.
- **Encapsulated Internal Modules:** Submodules (e.g. `instructions`, `decoders`, internal callbacks) declared `pub mod` that should be `pub(crate) mod`.

### Pillar 3: Readability Review
Review names, function responsibilities and control flow against
[performance-and-readability.md](../../rules/performance-and-readability.md).
Assess condition meaning and context without counting operators. Preserve
short-circuit evaluation, side effects and hardware timing in any proposed fix.
This is an agent review; the deterministic auditor has no condition scanner.

### Pillar 4: Method Naming & Accessor Conventions
- **Method Naming & Accessor Conventions:**
  1. **Standard Getters:** Must exactly match the field name (do NOT use a `get_` prefix). Pattern: `pub const fn <field>(&self) -> T` (or `&T` if non-Copy).
  2. **Boolean Getters:** Must start with the `is_` prefix (or retain natural boolean prefixes like `has_`, `can_` if already present in the field name). If field is named `enabled: bool` -> getter is `pub const fn is_enabled(&self) -> bool`. If field already has `is_` (e.g. `is_active: bool`), do not duplicate it (`pub const fn is_active(&self) -> bool`).
  3. **Setters:** Must start with the `set_` prefix followed by the field name. Pattern: `pub const fn set_<field>(&mut self, value: T)`.
  4. **Collection Getters (Slice Views):** Getters exposing internal buffers or sequences must return borrowed slices (`&[T]` or `&mut [T]`), never references to concrete containers (`&Vec<T>`). Example: Field `data: Vec<i16>` -> getter `pub fn data(&self) -> &[i16]`.
- **Review Method:** Interpret accessor findings from existing verification results; assess naming and return types against actual semantics before recommending changes.

### Pillar 5: Compiler-Grade AST Invariants & Workspace Lints
- **Zero Host Panics on Guest Code:** Denied in production code via `clippy::unwrap_used = "deny"`, `clippy::expect_used = "deny"`, and `clippy::panic = "deny"`. (Integration tests exempt via `#![allow(...)]`).
- **Disallowed Abstractions & Concurrency:** Blocked via `clippy::disallowed_types` (`Rc`, `RefCell`, `Arc`, `Mutex`, `RwLock`, `mpsc::Sender`, `mpsc::Receiver`) and `clippy::disallowed_methods` (`std::thread::spawn`).
- **Borrow Views over Containers:** Functions inspecting buffers or sequences must accept borrowed slices (`&[T]`, `&mut [T]`) rather than concrete heap containers (`&Vec<T>`, `&mut Vec<T>`), and accept `&str` instead of `&String`. Enforced via `clippy::ptr_arg = "deny"`.
- **Constructor & Derives Discipline:** Enforced via `clippy::new_without_default = "deny"`, `clippy::new_ret_no_self = "deny"`, `clippy::expl_impl_clone_on_copy = "deny"`, and `missing_debug_implementations = "warn"`.
- **Verification Ownership:** Commit verification owns workspace Clippy. Review its existing diagnostics without repeating the gate here.

### Pillar 6: Automated Architecture Guardrails
- **Method Inlining Guidelines:** Verified via `test_inlining_guidelines_compliance` in `check_architecture.py`.
- **Macro & Const-Generic Prohibitions:** Verified via `test_zero_user_defined_macros` and `test_zero_const_generic_handlers`.
- **Dedicated External Test Suites & Parity:** Verified via `test_every_crate_has_dedicated_external_tests_suite`, `test_canonical_test_file_naming_convention`, and `test_zero_inline_tests_in_crates_src`.
- **Path Privacy & Host Isolation:** Verified via `test_no_external_hardcoded_paths`.
- **Source File Size Limits (800 lines):** Verified via `test_file_size_limits` and `test_no_stale_line_count_exceptions`.
- **Verification Ownership:** Commit verification owns the architecture suite. Review its existing results and inspect gaps that require architectural judgment.

---

## 3. Analytical Review Workflow

1. Establish the requested scope and inspect the available verification results.
   Commit verification runs the complete dead-code, visibility and accessor audit;
   milestone verification does not repeat it. Do not call the Python auditor, Clippy,
   architecture suite or completion verifier from this skill merely to repeat
   those checks. If results are unavailable or stale, report that limitation;
   a manual review can still proceed without claiming automatic checks passed.
2. Validate reported candidates against actual callers, external interfaces,
   specification obligations and conditional execution. The Python scanner uses
   textual references and is not authoritative compiler reachability analysis.
3. Inspect the relevant source or diff for cohesion, information hiding,
   short-circuit behavior and meaningful test coverage. For an explicitly
   requested diff review or major architectural branch merge, read
   [the manual diff checklist](references/manual-diff-checklist.md).
4. Report confirmed issues, rejected candidates and proposed remedies. Perform
   edits only when remediation is within the requested scope.
5. After an authorized visibility change or deletion, run
   `python tools/verification/verify.py check build` to check production callers. Integration tests may
   require public API; a successful production check does not establish their
   validity. Completion verification belongs to the enclosing task's workflow
   and must cover the final source after any edits.

---

## 4. Dead Code & Zombie Pruning Playbook

Follow this systematic procedure when remediating dead code and zombies:

### Step 1: Triage Test-Only Zombies
For each symbol reported under `[TEST-ONLY ZOMBIES]`:
1. Check if it represents an **external Host I/O boundary**:
   - Host input injection (e.g. `keyboard::key_down`, `game_ports::plug_port1`, `floppy::insert_disk`). These are intentional public API hooks for frontend GUI / CLI runners.
   - Retain these methods and add doc comments clarifying their Host I/O purpose.
2. If it does NOT represent an external host interface:
   - Check whether it is obsolete scaffolding or a legitimate behavior exercised only by current tests. Recommend removal only after confirming the code is superseded and the test is orphaned.

### Step 2: Safe Clean-Break Deletion
1. Delete confirmed dead symbols in `crates/<crate>/src/`.
2. Delete orphaned test assertions/functions in `crates/<crate>/tests/`.
3. Run `python tools/verification/verify.py check build` to detect broken production callers.
4. Return the changed scope to the enclosing completion workflow for verification of the final source; do not repeat the full verification set inside this skill.


## 5. The Verbal Double-Check (Self-Audit & Heuristic Verification)

Using the available evidence, explicitly review the **5 Non-Negotiable Conscience Questions**:
1. 🧠 **Spec Freshness Review:** Did code refactoring or pruning introduce behavior changes not yet updated in `Obsidian/Amiga/Design/*.md`?
2. 🚫 **Anti-Nudge Review (`structural-root-cause.md`):** Are all clock delays, cycle counts, and beam offsets silicon-verified rather than empirical $\pm 1$ / $\pm 2$ symptom nudges?
3. 🔬 **Assertion Density & Genuine Test Review (`unit-testing-policy.md`):** Do unit tests genuinely verify chip behavior and state changes, or do they only assert trivial boilerplate?
4. 📢 **Spec Conflict Escalation (`spec-compliance.md`):** Were any conflicts between reference test suites and internal design specs escalated to the user before changing code?
5. 🧹 **Clean-Break Refactoring (`clean-break-refactoring.md`):** Were old methods, legacy aliases, and temporary shims completely deleted rather than left behind?

## 6. Output Contract
Conclude with the standardized summary report:
Report observed results and counts; mark checks that were not run instead of copying the example PASS values.
```markdown
### 🛡️ Code Quality Audit & Pruning Report
- **Scope Reviewed:** <inspected crates/files/diff>
- **Dead Code Pruned:** <count> symbols
- **Test-Only Zombies Handled:** <count> retained (Host I/O) / <count> pruned
- **Visibility Demoted:** <count> symbols (`pub` -> `pub(crate)` / private)
- **Readability Review:** <concrete findings and inspected scope; no operator-count threshold>
- **Method Naming & Accessor Conventions:** [PASS | <count> violations]
- **Workspace Clippy & Compiler Lints:** [PASS | <count> violations]
- **Architecture Rules (`check_architecture.py`):** [PASS | <passed>/<total> tests passed]
- **Manual Review:** <observations and unresolved questions>
- **Verification Evidence:** <existing results, source freshness, compiler checks actually run, and missing checks>
```
