---
name: prune-dead-code
description: >-
  Use this skill when completing major roadmap milestones to identify and safely eliminate dead code, unused functions, obsolete constants, superseded scaffolding, and unreferenced crate exports across the workspace.
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
