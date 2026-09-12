---
name: amiga-scaffold-crate
description: Deterministic scaffolding of workspace crates in crates/ adhering to 3-tier architecture and rules via tools/scaffold_crate.py.
---

# Recipe: Deterministic Workspace Crate Scaffolding

This skill provides the operational procedure for scaffolding new crates in `crates/<crate_name>` conforming strictly to the repository's 3-tier layout, decoupled state architecture, and unit testing policies.

---

## 1. When to Use This Skill

Activate this skill whenever:
- Adding a new hardware peripheral, coprocessor, or subsystem to the workspace.
- Extracting a complex component from an oversized crate into a dedicated Tier 3 subcomponent.
- Creating isolated test utilities or benchmarking harnesses under `crates/`.

---

## 2. Core Architectural Invariants Enforced

1. **Flat on Disk, 3-Tier Hierarchy in Code:** All crates reside directly in `crates/*` (per [`.agents/rules/workspace-structure-and-reexports.md`](../../rules/workspace-structure-and-reexports.md)).
2. **Decoupled State Struct:** Every crate provides a `<Name>State` struct deriving `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default` (per [`.agents/rules/rust-best-practices.md`](../../rules/rust-best-practices.md)).
3. **Dedicated External Test Suite:** An external test harness is created at `crates/<crate>/tests/test_<crate>.rs`. Zero inline tests in `src/` (per [`.agents/rules/unit-testing-policy.md`](../../rules/unit-testing-policy.md)).
4. **Workspace Inheritance:** Crate `Cargo.toml` uses `version.workspace = true`, `edition.workspace = true`, and workspace dependencies.
5. **Automated Root Registration:** The crate is registered in root `Cargo.toml` `members` and `[workspace.dependencies]`.

---

## 3. CLI Execution Procedure

Run the deterministic CLI tool:

```powershell
# Tier 2 (Peer Subsystem): e.g. cia, agnus, paula
python tools/scaffold_crate.py <crate_name> --tier 2 --check

# Tier 3 (Contained Subcomponent): e.g. rtc under memory_bus, copper under agnus
python tools/scaffold_crate.py <crate_name> --tier 3 --parent <parent_crate> --check

# Tier 1 (Foundational Blueprint): e.g. config
python tools/scaffold_crate.py <crate_name> --tier 1 --check

# Preview without writing to disk:
python tools/scaffold_crate.py <crate_name> --tier <1|2|3> --dry-run
```

---

## 4. Execution Mode: Subagent Delegation

When crate creation is delegated to a child subagent:

### Task Invocation Template
```markdown
Run `tools/scaffold_crate.py` to scaffold new crate `crates/<crate_name>` with Tier <tier> and parent `<parent>`.
Implement the initial register map and data structures in `src/lib.rs`.
Verify clean compilation via `cargo check -p <crate_name> --quiet` and external tests via `cargo test -p <crate_name> --quiet`.
Return the Return Contract upon completion.
```

### Mandatory Return Contract
```markdown
### Subagent Return Contract: amiga-scaffold-crate
- **Crate Name & Tier**: `crates/<crate_name>` (Tier <1|2|3>)
- **Parent Re-export**: `crates/<parent_crate>` -> `pub use <crate_name>;`
- **State Struct**: `<PascalName>State` (Derives: `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default`)
- **External Test Suite**: `crates/<crate_name>/tests/test_<crate_name>.rs`
- **Verification**: `cargo check` and `cargo test` status (PASS/FAIL)
```
