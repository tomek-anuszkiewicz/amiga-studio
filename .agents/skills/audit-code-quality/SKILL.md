---
name: audit-code-quality
description: Deep architectural code quality audit and remediation covering dead code, test-only zombies, minimum visibility leaks, SRP cohesion, inlining, and test parity across workspace crates.
---

# Recipe: Architectural Code Quality Auditor & Pruning Playbook

This skill provides a comprehensive, on-demand procedure across the Rust workspace to audit code rot, prune dead and zombie code, enforce the Principle of Minimum Visibility, maintain structural cohesion (Single Responsibility Principle), verify method inlining, and enforce external test suite parity.

---

## 1. When to Trigger This Skill

- **Major Milestone Completion:** Mandatory clean-up after completing milestones in [ROADMAP.md](../../../ROADMAP.md) to purge superseded scaffolding, unreferenced helpers, and test zombies.
- **Pre-Review Quality Gate:** Run prior to executing [`/code-review`](../../workflows/code-review.md) to eliminate cognitive clutter and visibility leaks before architectural reviews.
- **Refactoring Sprints:** Run whenever restructuring crate boundaries, decomposing oversized files, or auditing information hiding.

---

## 2. The Seven Code Quality Pillars

### Pillar 1: Dead Code & Test-Only Zombies
1. **Completely Dead Symbols (💀):**
   - Declared symbols with **zero callers anywhere** across `crates/*/src/` and `crates/*/tests/`.
   - *Remediation:* Safe to delete immediately.
2. **Test-Only Zombie Code (🧟):**
   - Symbols with **zero callers in production code (`crates/*/src/`)**, but referenced in unit/integration tests (`crates/*/tests/`).
   - *Origin:* A helper was written, tested, and subsequently bypassed in the main machine loop or bus. The test kept passing, creating the false impression that the code was active.
   - *Remediation:* Triage against external Host I/O boundaries (see Section 4). If internal scaffolding, purge both the method and its orphaned test.
3. **Internal Unused Symbols (The "Visibility Downgrade" Indicator):**
   - In Rust library crates (`[lib]`), `rustc` treats all `pub` items as public API and suppresses `dead_code` warnings.
   - Demoting `pub` to `pub(crate)` unmasks `rustc`'s built-in reachability analysis via `cargo check --workspace`.

### Pillar 2: Principle of Minimum Visibility (Least Privilege)
- **Over-Exposed `pub` Items:** Symbols declared `pub` whose callers are strictly confined to their own crate. Demote to `pub(crate)`.
- **Over-Exposed `pub(crate)` / `pub` Items:** Symbols whose callers reside strictly within their defining file. Demote to private `fn`.
- **Encapsulated Internal Modules:** Submodules (e.g. `instructions`, `decoders`, internal callbacks) declared `pub mod` that should be `pub(crate) mod`.

### Pillar 3: Struct Cohesion & Single Responsibility
- **Unencapsulated "God Structs":** Structs declaring $> 12$ public fields, signaling mixed concerns or lack of domain groupings.
- **Source File Ceilings:** Files in `crates/*/src/` exceeding the **800-line ceiling** (mechanically enforced via `cargo test -p test_runner --test test_architecture_rules`).

### Pillar 4: Condition Soup & Explaining Variables (`--conditions`)
- **Self-Documenting Boolean Logic:** Scans for dense compound conditionals (`if (a || b) && c && d`) that should be decomposed into named explaining variables (`let is_ready = ...;`) or domain predicate methods per [`.agents/rules/performance-and-readability.md`](../../rules/performance-and-readability.md).
- **Prohibition of Multi-Clause Clutter:** Flags conditionals with mixed nested operators or $\ge 3$ connectives to ensure code reads like declarative hardware specification prose.
- **Short-Circuit Preservation Invariant:** Explaining variables must never eagerly evaluate sub-expressions or function calls that would otherwise be avoided via boolean short-circuit evaluation (`&&`, `||`) or branched execution (`match`).

### Pillar 5: Method Naming & Accessor Conventions (`--accessors`)
- **Method Naming & Accessor Conventions:**
  1. **Standard Getters:** Must exactly match the field name (do NOT use a `get_` prefix). Pattern: `pub const fn <field>(&self) -> T` (or `&T` if non-Copy).
  2. **Boolean Getters:** Must start with the `is_` prefix (or retain natural boolean prefixes like `has_`, `can_` if already present in the field name). If field is named `enabled: bool` -> getter is `pub const fn is_enabled(&self) -> bool`. If field already has `is_` (e.g. `is_active: bool`), do not duplicate it (`pub const fn is_active(&self) -> bool`).
  3. **Setters:** Must start with the `set_` prefix followed by the field name. Pattern: `pub const fn set_<field>(&mut self, value: T)`.
  4. **Collection Getters (Slice Views):** Getters exposing internal buffers or sequences must return borrowed slices (`&[T]` or `&mut [T]`), never references to concrete containers (`&Vec<T>`). Example: Field `data: Vec<i16>` -> getter `pub fn data(&self) -> &[i16]`.
- **Verification Method:** Verified directly via `python tools/harness/audit_code_quality.py --accessors`.

### Pillar 6: Compiler-Grade AST Invariants & Workspace Lints
- **Zero Host Panics on Guest Code:** Denied in production code via `clippy::unwrap_used = "deny"`, `clippy::expect_used = "deny"`, and `clippy::panic = "deny"`. (Integration tests exempt via `#![allow(...)]`).
- **Disallowed Abstractions & Concurrency:** Blocked via `clippy::disallowed_types` (`Rc`, `RefCell`, `Arc`, `Mutex`, `RwLock`, `mpsc::Sender`, `mpsc::Receiver`) and `clippy::disallowed_methods` (`std::thread::spawn`).
- **Borrow Views over Containers:** Functions inspecting buffers or sequences must accept borrowed slices (`&[T]`, `&mut [T]`) rather than concrete heap containers (`&Vec<T>`, `&mut Vec<T>`), and accept `&str` instead of `&String`. Enforced via `clippy::ptr_arg = "deny"`.
- **Constructor & Derives Discipline:** Enforced via `clippy::new_without_default = "deny"`, `clippy::new_ret_no_self = "deny"`, `clippy::expl_impl_clone_on_copy = "deny"`, and `missing_debug_implementations = "warn"`.
- **Verification Gate:** Enforced on every build via `cargo clippy --workspace --all-targets` and `check_clippy_invariants()` in `tools/harness/pre_flight.py`.

### Pillar 7: Automated Architecture Guardrails
- **Method Inlining Guidelines:** Verified via `test_inlining_guidelines_compliance` in `test_architecture_rules.rs`.
- **Macro & Const-Generic Prohibitions:** Verified via `test_zero_user_defined_macros` and `test_zero_const_generic_handlers`.
- **Dedicated External Test Suites & Parity:** Verified via `test_every_crate_has_dedicated_external_tests_suite`, `test_canonical_test_file_naming_convention`, and `test_zero_inline_tests_in_crates_src`.
- **Path Privacy & Host Isolation:** Verified via `test_no_external_hardcoded_paths`.
- **Source File Size Limits (800 lines):** Verified via `test_file_size_limits` and `test_no_stale_line_count_exceptions`.
- **Verification Gate:** Enforced via `cargo test -p test_runner --test test_architecture_rules`.

---

## 3. CLI Audit Workflow

```powershell
# Full workspace deep code quality audit
python tools/harness/audit_code_quality.py --all

# Workspace-wide Clippy and compiler invariants
cargo clippy --workspace --all-targets

# Automated architecture rules
cargo test -p test_runner --test test_architecture_rules -- --quiet

# Audit dead code & zombies in a specific crate
python tools/harness/audit_code_quality.py --dead-code --crate paula

# Audit only visibility leaks across the workspace
python tools/harness/audit_code_quality.py --visibility

# Audit struct cohesion (> 12 public fields)
python tools/harness/audit_code_quality.py --srp

# Audit condition soup and boolean clarity
python tools/harness/audit_code_quality.py --conditions

# Audit method naming and accessor conventions
python tools/harness/audit_code_quality.py --accessors

# Machine-readable JSON export
python tools/harness/audit_code_quality.py --all --json > quality_report.json
```

---

## 4. Dead Code & Zombie Pruning Playbook

Follow this systematic procedure when remediating dead code and zombies:

### Step 1: Triage Test-Only Zombies
For each symbol reported under `[TEST-ONLY ZOMBIES]`:
1. Check if it represents an **external Host I/O boundary**:
   - Host input injection (e.g. `keyboard::key_down`, `game_ports::plug_port1`, `floppy::insert_disk`). These are intentional public API hooks for frontend GUI / CLI runners.
   - Retain these methods and add doc comments clarifying their Host I/O purpose.
2. If it does NOT represent an external host interface:
   - It is obsolete scaffolding. Mark both the production method and its orphaned test for clean-break removal.

### Step 2: Safe Clean-Break Deletion
1. Delete confirmed dead symbols in `crates/<crate>/src/`.
2. Delete orphaned test assertions/functions in `crates/<crate>/tests/`.
3. Run `cargo check --workspace` to verify zero unbroken callers remain.
4. Run `python tools/harness/pre_flight.py` to confirm workspace compiles cleanly.




