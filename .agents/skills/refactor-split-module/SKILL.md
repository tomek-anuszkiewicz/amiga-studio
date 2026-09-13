---
name: refactor-split-module
description: Decompose oversized Rust files (> 800 lines) into cohesive submodules while preserving 3-tier re-exports.
---

# Recipe: Rust Module Decomposition & 800-Line Refactoring

This skill provides the operational procedure for refactoring and splitting oversized or low-cohesion Rust source files under `crates/*/src/` to strictly comply with the $\le 800$-line constitutional limit per [`.agents/rules/file-size-and-cohesion.md`](../../rules/file-size-and-cohesion.md).

---

## 1. When to Use This Skill

Activate this skill whenever:
- An existing `.rs` file under `crates/*/src/` approaches or exceeds 750–800 lines.
- The automated architecture test `test_file_size_limits` fails during verification.
- A single module accumulates multiple distinct responsibilities (e.g., mixing register state definition, bus routing, and display rendering).

---

## 2. Decomposition Principles

1. **Strict 800-Line Ceiling:** Every `.rs` file in `crates/*/src/` must remain $\le 800$ lines.
2. **Flat Instruction Hierarchy:** Under `crates/m68000/src/instructions/`, maintain a flat 1:1 opcode-to-file mapping with zero subdirectories.
3. **3-Tier Re-Export Preservation:** External callers must not experience breaking API changes. Re-export public types from the crate root (`src/lib.rs`) via `pub use submodule::TypeName;`.
4. **Disjoint Borrowing & Zero Allocations:** Ensure split submodules preserve independent field borrowing without requiring `Rc<RefCell<...>>` or heap allocations in hot paths.
5. **Zero Backward-Compatibility Shims (Atomic Refactoring):** Do not create dummy wrapper modules (`pub mod former { pub use new::*; }`) or import aliases (`use new as old;`) to delay updating callers. Update all consumers across the workspace directly to the new canonical path in the same task.

---

## 3. Step-by-Step Execution Workflow

### Step 1: Detect Oversized Files
Run the architecture rule test to identify files exceeding the limit:
```powershell
cargo test -p test_runner --test test_architecture_rules -- test_file_size_limits
```
Or check line counts directly:
```powershell
Get-ChildItem -Recurse -Filter "*.rs" crates/*/src | Select-Object FullName, @{Name="Lines";Expression={(Get-Content $_.FullName | Measure-Object -Line).Lines}} | Where-Object Lines -gt 700 | Sort-Object Lines -Descending
```

### Step 2: Analyze Cohesion & Identify Split Boundaries
Examine the oversized file and identify natural functional boundaries:
- **Type Definitions & State:** Extract structs, enums, bitflags, and `impl Default` into a dedicated `types.rs` or `state.rs`.
- **Parsing / Decoding Logic:** Extract decode tables, bitfield unpackers, or string formatters into `decode.rs` or `parser.rs`.
- **Internal Helper Algorithms:** Extract pure mathematical functions or LUT helpers into `helpers.rs`.
- **Panel / Component Separation (GUI):** Split monolithic UI code into distinct panel files under `crates/gui/src/panels/`.

### Step 3: Extract Submodule & Wire Visibility
1. Create the new submodule file (e.g., `crates/<crate>/src/<submodule>.rs`).
2. Move the cohesive types and functions into the new file.
3. Mark types and methods intended for crate-internal or public use with appropriate visibility (`pub(crate)` or `pub`).
4. In `src/lib.rs` (or parent module):
   ```rust
   mod submodule;
   pub use submodule::{ExportedType1, ExportedType2};
   ```
5. If types or submodules are relocated or split across crates, grep and update all call sites across the entire repository immediately. Never leave transitional dummy wrapper modules or backward-compatibility aliases.

### Step 4: Verify Zero Host Panics & Inlining
Ensure extracted code adheres to project invariants:
- Zero `.unwrap()` / `.expect()` in runtime emulation paths.
- Hot flag math retains `#[inline(always)]`; cold exception paths retain `#[inline(never)]`.
- Wrapping arithmetic (`wrapping_add`, `wrapping_sub`) preserved.

### Step 5: Verify Compilation & Architecture Gates
1. Run workspace compilation:
   ```powershell
   cargo check --workspace
   ```
2. Run test suites for the modified crate:
   ```powershell
   cargo test -p <modified_crate>
   ```
3. Run the automated architecture rule test:
   ```powershell
   cargo test -p test_runner --test test_architecture_rules
   ```
4. Format code:
   ```powershell
   cargo fmt --all -- --check
   ```

---

## 4. Execution Mode: Subagent Delegation

- **Execution Host:** **Isolated Subagent** (child context sandbox).
- **Model Tier:** `Gemini Flash Medium`
- **Context Savings:** Absorbs large source file inspections, iterative `cargo check` compile logs, and intermediate syntax errors during module extraction.
- **Subagent Task Template:**
  - `TaskName`: "Decomposing Module: <target_file>"
  - `TaskSummary`: "Splits an oversized Rust file into cohesive submodules while preserving 3-tier public re-exports and architecture tests."
  - `Prompt`:
    ```markdown
    Decompose oversized module: <TARGET_FILE> (currently > 800 lines).
    Follow .agents/skills/refactor-split-module/SKILL.md:
    1. Create submodules under `<target_dir>/<submodule>/`.
    2. Extract functions/types by domain cohesion.
    3. Maintain 3-tier `pub use` re-exports in parent `mod.rs` or `lib.rs`.
    4. Verify with `cargo check` and `cargo test -p test_runner --test test_architecture_rules`.
    5. Return strictly the 1:1 Symbol Relocation Table and line count report below.
    ```
- **Return Contract (Mandatory Structured Output):**
  The subagent must conclude with this exact markdown block:
  ```markdown
  ### 🧩 Module Decomposition Report
  - **Target File:** `<original_file_path>`
  - **Decomposition Status:** [COMPLETE | REVERTED]
  - **Resulting Submodules & Line Counts:**
    | Submodule Path | Line Count | Status ($\le 800$) |
    | :--- | :--- | :--- |
    | `crates/.../part1.rs` | 340 lines | ✅ PASS |
    | `crates/.../part2.rs` | 420 lines | ✅ PASS |
  - **1:1 Symbol Relocation Table:**
    | Original Symbol | New Definition Location | Public Re-Export Path |
    | :--- | :--- | :--- |
    | `pub struct Foo` | `crates/.../foo.rs` | `crates/.../lib.rs::Foo` |
    | `fn internal_bar` | `crates/.../bar.rs` | `pub(crate) use bar::internal_bar` |
  - **Architecture Validation:** `test_architecture_rules` passed (0 files exceeding 800 lines).
  ```
