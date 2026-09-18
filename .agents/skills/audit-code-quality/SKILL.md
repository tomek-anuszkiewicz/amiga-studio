---
name: audit-code-quality
description: On-demand deep architectural code quality audit covering dead code, test-only zombies, minimum visibility leaks, and SRP cohesion across workspace crates.
---

# Recipe: On-Demand Architectural Code Quality Auditor

This skill provides a deep, comprehensive on-demand audit across the Rust workspace to detect code rot, information-hiding degradation, and structural cohesion drift that naturally accumulates during rapid feature work and refactoring.

---

## 1. When to Trigger This Skill

- **On-Demand Execution:** Run periodically during refactoring sprints, before milestone reviews (`/code-review`), or when inspecting visibility and architectural hygiene.
- **Why On-Demand:** Unlike instant pre-flight gates (`pre_flight.py`), deep whole-workspace reference indexing across all production and test files takes several seconds. Running it on-demand prevents slowing down micro-commit loops while ensuring a thorough safety net.

---

## 2. The 3 Quality Audit Pillars

### Pillar 1: Dead Code & Test-Only Zombies
- **Completely Dead Code:** Declared symbols with 0 callers anywhere across `crates/*/src/` and `crates/*/tests/`.
- **Test-Only Zombies:** Symbols uncalled by production code (`src/`), called solely in integration tests (`tests/`). These often indicate obsolete APIs kept alive only to satisfy their own unit tests.

### Pillar 2: Principle of Minimum Visibility (Least Privilege)
- **Over-Exposed `pub` Functions:** Symbols declared `pub` whose production callers are strictly confined to their own crate. These should be demoted to `pub(crate)`.
- **Over-Exposed `pub(crate)` / `pub` Functions:** Symbols whose callers reside strictly within their own defining file. These should be demoted to private `fn`.
- **Encapsulated Internal Modules:** Detects `pub mod` declarations for internal worker submodules (e.g. `instructions`, `decoders`, internal callbacks) that should be `pub(crate) mod`.

### Pillar 3: Single Responsibility Principle (SRP) & Structural Cohesion
- **File Size Violations:** Rust source files in `crates/*/src/` exceeding the constitutional **800-line ceiling** (per `file-size-and-cohesion.md`).
- **Unencapsulated "God Structs":** Structs declaring $> 12$ public fields, signaling uncoordinated state dumps or lack of domain groupings.

---

## 3. CLI Audit Workflow

Execute the auditor via Python harness:

### A. Full Workspace Deep Audit
```powershell
python tools/harness/audit_code_quality.py --all
```

### B. Targeted Subsystem Audits
```powershell
# Audit only visibility leaks across the entire workspace
python tools/harness/audit_code_quality.py --visibility

# Audit dead code & zombies in a specific crate
python tools/harness/audit_code_quality.py --dead-code --crate paula

# Audit SRP and file sizes
python tools/harness/audit_code_quality.py --srp
```

### C. Machine-Readable JSON Export
```powershell
python tools/harness/audit_code_quality.py --all --json > quality_report.json
```

---

## 4. Semantic SRP Review (The Agent's Cognitive Role)

While static scripts and regex scanners measure quantitative metrics (line counts > 800, public fields > 12, or caller counts), **evaluating the Single Responsibility Principle (SRP) requires semantic domain reasoning by the Agent**:

1. **Hotspot Inspection:**
   When `audit_code_quality.py` or architecture tests flag an oversized file or a struct with mixed responsibilities, the Agent must read the source code and identify the distinct conceptual domains.
2. **Domain Boundary Identification:**
   For example, in `crates/physical_memory/src/map.rs`:
   - **Responsibility A (Dispatch Infrastructure):** 64 KB memory bank callback dispatch table (`MemoryBank`, `BankHandler`, direct function pointers).
   - **Responsibility B (System Topology Presets):** Precalculated machine preset topologies (`build_preset_bank_map`, `BANK_MAP_BARE`, `BANK_MAP_STANDARD`, `BANK_MAP_EXPANDED`).
3. **Decomposition Proposal & Execution:**
   The Agent formulates a concrete decomposition plan:
   - Proposes extracting Responsibility B into a dedicated cohesive submodule (`presets.rs`).
   - Maintains 3-tier re-exports at the crate root (`src/<crate>.rs`) so downstream consumers experience zero breaking changes.
   - Adds 1:1 modular unit test parity (`tests/test_presets.rs`).
   - Delegates execution to the [`refactor-split-module`](../refactor-split-module/SKILL.md) skill.

---

## 5. Execution Mode: Subagent Delegation

- **Execution Host:** **Isolated Subagent** (child context sandbox).
- **Model Tier:** `Gemini Flash Low` / `Medium`
- **Context Savings:** Shields the main conversation from thousands of lines of workspace scan logs, callers lists, and AST grep outputs.
- **Subagent Task Template:**
  - `TaskName`: "Code Quality Audit: <workspace_or_crate>"
  - `TaskSummary`: "Runs deep on-demand code quality audit for dead code, visibility leaks, and SRP."
  - `Prompt`:
    ```markdown
    Execute on-demand code quality audit across `<SCOPE>`.
    Follow .agents/skills/audit-code-quality/SKILL.md:
    1. Run `python tools/harness/audit_code_quality.py --all`.
    2. Analyze findings across: Dead Code, Minimum Visibility Leaks, and SRP anomalies.
    3. Formulate concrete refactoring recommendations.
    4. Return strictly the Code Quality Audit Report below.
    ```
- **Return Contract (Mandatory Structured Output):**
  ```markdown
  ### 🛡️ Code Quality Audit Report
  - **Scope Scanned:** `<scope>`
  - **Dead Code Count:** <count> symbols
  - **Test-Only Zombies:** <count> symbols
  - **Visibility Leaks:** <count> symbols (`pub` -> `pub(crate)` / private)
  - **SRP / Cohesion Issues:** <count> files/structs
  - **Top Remediation Targets:**
    | Subsystem | Symbol / File | Issue | Recommended Action |
    | :--- | :--- | :--- | :--- |
    | `physical_memory` | `read_chip_ram` | Leaked `pub` | Demote to private `fn` |
    | `m68000` | `instructions/move_b.rs` | 1,980 lines | Split into submodules |
  ```
