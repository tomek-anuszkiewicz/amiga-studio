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

## 2. The Nine Code Quality Pillars

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

### Pillar 3: Single Responsibility Principle (SRP) & Structural Cohesion
- **Source File Ceilings:** Files in `crates/*/src/` exceeding the **800-line ceiling** (per [`.agents/rules/file-size-and-cohesion.md`](../../rules/file-size-and-cohesion.md)).
- **Unencapsulated "God Structs":** Structs declaring $> 12$ public fields, signaling mixed concerns or lack of domain groupings.

### Pillar 4: Method Inlining Guidelines (`--inlining`)
- **`#[inline(always)]`**: Reserved for hot arithmetic/logic and CCR flag calculations ($X, N, Z, V, C$) executed on every single clock cycle per [`.agents/rules/method-inlining.md`](../../rules/method-inlining.md).
- **`#[inline(never)]`**: Mandatory on cold exception and trap triggers (`trigger_address_error`, `trigger_bus_error`, `trigger_illegal_instruction`). Keeping complex frame construction out-of-line keeps hot paths clean.

### Pillar 5: Macro & Const-Generic Prohibitions (`--antipatterns`)
- **Prohibition of User-Defined Macros:** Custom `macro_rules!` are strictly forbidden across workspace crates per [`.agents/rules/performance-and-readability.md`](../../rules/performance-and-readability.md).
- **Prohibition of Const-Generics for Opcodes:** Instruction handlers and decoding must not use `<const N: ...>` generic templates in `m68000`.

### Pillar 6: Dedicated External Test Suites & Parity (`--tests`)
- **Dedicated External Tests:** All tests must reside strictly in `crates/<crate>/tests/` with canonical `test_<name>.rs` filenames per [`.agents/rules/unit-testing-policy.md`](../../rules/unit-testing-policy.md).
- **Zero Inline Tests:** `#[cfg(test)] mod tests` in production `src/` files is strictly forbidden.
- **1:1 Multi-Module Parity:** Multi-module crates maintain dedicated unit test files mirroring submodules.

### Pillar 7: Path Privacy & Host Isolation (`--path-privacy`)
- **Zero Hardcoded Paths:** Verifies zero host/user paths (e.g. `D:\...`, `/home/...`) in workspace code per [`.agents/rules/no-external-paths.md`](../../rules/no-external-paths.md).

### Pillar 8: Condition Soup & Explaining Variables (`--conditions`)
- **Self-Documenting Boolean Logic:** Scans for dense compound conditionals (`if (a || b) && c && d`) that should be decomposed into named explaining variables (`let is_ready = ...;`) or domain predicate methods per [`.agents/rules/performance-and-readability.md`](../../rules/performance-and-readability.md).
- **Prohibition of Multi-Clause Clutter:** Flags conditionals with mixed nested operators or $\ge 3$ connectives to ensure code reads like declarative hardware specification prose.
- **Short-Circuit Preservation Invariant:** Explaining variables must never eagerly evaluate sub-expressions or function calls that would otherwise be avoided via boolean short-circuit evaluation (`&&`, `||`) or branched execution (`match`).

### Pillar 9: Struct Encapsulation & Accessor Discipline (--accessors)
- **Zero Raw Public Fields:** All struct fields must remain strictly private per [`.agents/rules/rust-best-practices.md`](../../rules/rust-best-practices.md). Raw public fields leak internal representation and bypass domain invariants.
- **Category A (Value Objects / POD Structs):** Pure data structs (primitives, raw numbers, small `Copy` types):
  - Make all fields private.
  - Provide `pub const fn new(...) -> Self`.
  - Add `#[inline(always)] pub const fn <field>(&self)` getters (return `T` if `Copy`, else `&T`).
  - Add `#[inline(always)] pub const fn set_<field>(&mut self, val: T)` setters.
  - Derive `Debug, Clone, Copy, PartialEq, Eq` when possible.
- **Category B (All Remaining / Complex Structs):** Structs with allocations, handles, non-primitive state, or business logic invariants:
  - All fields must remain strictly private.
  - Provide appropriate constructors (`new` or fallible `try_new` with validation).
  - Accessors: `pub const fn <field>(&self) -> &T` if compile-time evaluatable, otherwise `pub fn <field>(&self) -> &T`.
  - Setters: Only provide if explicitly required by domain logic, enforcing necessary invariants and validations.
- **Method Naming & Accessor Conventions:**
  1. **Standard Getters:** Must exactly match the field name (do NOT use a `get_` prefix). Pattern: `pub const fn <field>(&self) -> T` (or `&T` if non-Copy).
  2. **Boolean Getters:** Must start with the `is_` prefix (or retain natural boolean prefixes like `has_`, `can_` if already present in the field name). If field is named `enabled: bool` -> getter is `pub const fn is_enabled(&self) -> bool`. If field already has `is_` (e.g. `is_active: bool`), do not duplicate it (`pub const fn is_active(&self) -> bool`).
  3. **Setters:** Must start with the `set_` prefix followed by the field name. Pattern: `pub const fn set_<field>(&mut self, value: T)`.
- **Verification Method:** Verified directly via `python tools/harness/audit_code_quality.py --accessors` alongside Agent cognitive inference for domain-specific invariant validation.

---

## 3. CLI Audit Workflow

```powershell
# Full workspace deep code quality audit
python tools/harness/audit_code_quality.py --all

# Audit dead code & zombies in a specific crate
python tools/harness/audit_code_quality.py --dead-code --crate paula

# Audit only visibility leaks across the workspace
python tools/harness/audit_code_quality.py --visibility

# Audit SRP and file sizes
python tools/harness/audit_code_quality.py --srp

# Audit method inlining compliance
python tools/harness/audit_code_quality.py --inlining

# Audit macro and const-generic prohibitions
python tools/harness/audit_code_quality.py --antipatterns

# Audit external test suite organization
python tools/harness/audit_code_quality.py --tests

# Audit condition soup and boolean clarity
python tools/harness/audit_code_quality.py --conditions

# Audit struct accessors and method naming conventions
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

---

## 5. Struct Encapsulation & Accessor Remediation Playbook (--accessors)

Follow this systematic procedure when remediating unencapsulated structs and public fields:

### Step 1: Classify Struct Category
1. **Category A (Value Objects / POD Structs):** Pure data structs (primitives, coordinates, RGB/audio samples, raw numeric pairs).
2. **Category B (Complex / Invariant Structs):** Structs containing allocations (`Vec`), handles, non-primitive state, or business logic invariants.

### Step 2: Safe Encapsulation Remediation
1. Make all fields private (remove `pub` from field declarations).
2. For Category A:
   - Provide `pub const fn new(...) -> Self`.
   - Provide standard getters matching field name without `get_` prefix (`#[inline(always)] pub const fn <field>(&self)`).
   - Provide boolean getters starting with `is_` (e.g. `is_enabled(&self) -> bool`, retaining `has_`/`can_`).
   - Provide setters starting with `set_` (`#[inline(always)] pub const fn set_<field>(&mut self, val: T)`).
   - Derive `Debug, Clone, Copy, PartialEq, Eq` when possible.
3. For Category B:
   - Provide `new` or `try_new` constructors enforcing domain invariants.
   - Provide `pub const fn <field>(&self) -> &T` or `pub fn <field>(&self) -> &T` reference getters matching field name without `get_` prefix.
   - Add setters only when explicitly required by domain logic, prefixed with `set_<field>`, enforcing necessary validations.
4. Update all call sites across `crates/*/src/` and `crates/*/tests/` to use accessors and constructors.
5. Verify via `python tools/harness/audit_code_quality.py --accessors`, `cargo check --workspace`, and `python tools/harness/pre_flight.py`.


