---
name: prune-dead-code
description: Scan and eliminate obsolete constants, unused functions, and superseded scaffolding across workspace crates.
---

# Recipe: Dead Code Elimination & Scaffolding Pruning

This skill provides a systematic procedure for identifying and purging dead code across the Rust workspace after completing major roadmap milestones.

---

## 1. When to Trigger This Skill

- **Mandatory Trigger:** Completion of a major milestone in [ROADMAP.md](../../../ROADMAP.md).
- **Goal:** Prevent codebase rot, eliminate cognitive clutter, and ensure every remaining constant, type, and method serves an active architectural purpose.


---

## 2. Step-by-Step Audit & Pruning Workflow

### Step 1: Compiler Diagnostic Scan
Run a comprehensive check across all workspace targets, enabling dead code and unused lint diagnostics:
```powershell
cargo check --all-targets --workspace
```
Inspect the output for:
- `dead_code` warnings (unused structs, enum variants, or helper functions).
- `unused_imports` (lingering imports from refactored modules).
- `unused_variables` / `unused_mut`.

### Step 2: Unreferenced Constant & Symbol Audit
Audit domain-specific constants and internal methods that may be unreferenced outside their defining module:
- Search for constants in `crates/*/src/` that are no longer referenced by active execution paths (e.g., historical memory sizing constants or superseded mask definitions).
- Check `pub use` re-exports in crate root files (`crates/*/src/lib.rs`) against the 3-Tier Re-Export Strategy in [`.agents/rules/workspace-structure-and-reexports.md`](../../rules/workspace-structure-and-reexports.md).

### Step 3: Scaffolding & Test Cleanup
- Audit `tests/` and helper harnesses for superseded test fixtures or mock objects no longer invoked by current integration tests.
- Verify whether obsolete benchmark catalog rows or legacy dispatch shims are present.

### Step 4: Safe Removal & Refactoring
- Remove confirmed dead symbols directly.
- Avoid introducing speculative replacements.
- If a method was deprecated, eliminate it completely rather than leaving dead stub wrappers.

### Step 5: Regression & Correctness Verification
Validate that no necessary symbols were inadvertently removed:
```powershell
cargo test --workspace
cargo test -p test_runner --test test_architecture_rules
cargo fmt --all -- --check
```
Ensure all workspace tests pass with 100% green status.

---

## 4. Execution Mode: Subagent Delegation

- **Execution Host:** **Isolated Subagent** (child context sandbox).
- **Model Tier:** `Gemini Flash Low`
- **Context Savings:** Isolates workspace-wide grep searches, unused code warning scans, and dead symbol checks from the main conversation.
- **Subagent Task Template:**
  - `TaskName`: "Pruning Dead Code: <subsystem_or_crate>"
  - `TaskSummary`: "Scans workspace for obsolete constants, unreferenced functions, and superseded scaffolding."
  - `Prompt`:
    ```markdown
    Scan and prune dead code across `<WORKSPACE_OR_CRATE>`.
    Follow .agents/skills/prune-dead-code/SKILL.md:
    1. Scan compiler dead-code warnings: `cargo check --workspace`.
    2. Search for unused `pub(crate)` functions and unreferenced constants.
    3. Remove confirmed dead symbols.
    4. Run `cargo test --workspace` and `cargo fmt --all -- --check`.
    5. Return strictly the Dead Code Pruning Report below.
    ```
- **Return Contract (Mandatory Structured Output):**
  The subagent must conclude with this exact markdown block:
  ```markdown
  ### ✂️ Dead Code Pruning Report
  - **Scope Scanned:** `<scope>`
  - **Pruning Status:** [PRUNED | CLEAN (NO DEAD CODE)]
  - **Pruned Symbols & Locations:**
    | Dead Symbol | File Path | Line Range | Verified Zero Callers |
    | :--- | :--- | :--- | :--- |
    | `fn old_helper` | `crates/.../lib.rs` | L45-L60 | Confirmed via grep |
  - **Verification:** `cargo test --workspace` (PASS).
  ```
