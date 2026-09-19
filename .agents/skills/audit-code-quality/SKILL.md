---
name: audit-code-quality
description: Deep architectural code quality audit and remediation covering dead code, test-only zombies, minimum visibility leaks, and SRP cohesion across workspace crates.
---

# Recipe: Architectural Code Quality Auditor & Pruning Playbook

This skill provides a comprehensive, on-demand procedure across the Rust workspace to audit code rot, prune dead and zombie code, enforce the Principle of Minimum Visibility, and maintain structural cohesion (Single Responsibility Principle).

---

## 1. When to Trigger This Skill

- **Major Milestone Completion:** Mandatory clean-up after completing milestones in [ROADMAP.md](../../../ROADMAP.md) to purge superseded scaffolding, unreferenced helpers, and test zombies.
- **Pre-Review Quality Gate:** Run prior to executing [`/code-review`](../../workflows/code-review.md) to eliminate cognitive clutter and visibility leaks before architectural reviews.
- **Refactoring Sprints:** Run whenever restructuring crate boundaries, decomposing oversized files, or auditing information hiding.

---

## 2. The Five Quality Audit Pillars

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

### Pillar 4: Agent Skills Catalog Synchronization (`docs/ai_agents.md`)
- **Complete Skill Index Integrity:** Every active skill directory under `.agents/skills/` containing a `SKILL.md` must be cataloged in [`docs/ai_agents.md`](../../../docs/ai_agents.md).
- **Zero Phantom References:** Every skill linked in `docs/ai_agents.md` must actually exist on disk.
- **Audit Verification:** Verified automatically via `--skills` or `--all`. When drift is detected, add missing skills to the appropriate domain section in `docs/ai_agents.md` or prune deleted skills.

### Pillar 5: Two-Way Script Locality & Harness Governance
- **Harness Reservation:** `tools/harness/` is reserved strictly for universal, shared infrastructure used across multiple subsystems (pre-flight gates, git hooks, universal test runners, global rules).
- **Rule A (Specialized Locality):** Any script in `tools/harness/` referenced by $\le 1$ skill or workflow (and not part of global pre-commit/pre-flight) must be relocated to `.agents/skills/<skill>/scripts/`.
- **Rule B (Shared Promotion):** Any script inside `.agents/skills/<skill>/scripts/` referenced by $> 1$ distinct skills or workflows must be promoted into `tools/harness/` to avoid cross-skill leakage.
- **Audit Verification:** Verified automatically via `--scripts` or `--all`.

### Pillar 6: Design Documentation & Code Drift Detection (`--design-sync`)
- **Deterministic Git Checkpoints:** Every code-backed design specification in `Obsidian/Amiga/Design/` records `tracked_paths` and `last_synced_commit` in its YAML frontmatter.
- **Automated Drift Detection:** `audit_code_quality.py` computes `git rev-list --count <last_synced_commit>..HEAD -- <tracked_paths>` to identify specifications whose underlying Rust crates have moved forward without review.
- **Differential Inspection:** Inspect the exact code diff since the last synchronization via `--design-diff <doc>`.
- **Checkpoint Stamping:** Once the specification is updated (or verified to still be accurate), stamp the HEAD commit via `--design-bump <doc>`.
- **Audit Verification:** Verified automatically via `--design-sync` or `--all`.

---

## 3. CLI Audit Workflow

Execute the unified code quality auditor via Python harness:

### A. Full Workspace Deep Audit
```powershell
python tools/harness/audit_code_quality.py --all
```

### B. Targeted Subsystem Audits
```powershell
# Audit dead code & zombies in a specific crate
python tools/harness/audit_code_quality.py --dead-code --crate paula

# Audit only visibility leaks across the workspace
python tools/harness/audit_code_quality.py --visibility

# Audit SRP and file sizes
python tools/harness/audit_code_quality.py --srp

# Audit agent skills catalog synchronization in docs/ai_agents.md
python tools/harness/audit_code_quality.py --skills

# Audit two-way script locality and harness placement governance
python tools/harness/audit_code_quality.py --scripts

# Audit design specifications drift against code crates
python tools/harness/audit_code_quality.py --design-sync

# Inspect git diff for a drifted design specification
python tools/harness/audit_code_quality.py --design-diff Denise.md

# Bump checkpoint of a verified design specification to HEAD
python tools/harness/audit_code_quality.py --design-bump Denise.md
```

### C. Machine-Readable JSON Export
```powershell
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
2. Delete orphaned test assertions/cases in `crates/<crate>/tests/`.
3. Verify that test deletions do not violate the unit testing density invariant in [`.agents/rules/unit-testing-policy.md`](../../rules/unit-testing-policy.md) ($\ge 2$ tests, $\ge 10$ assertions per crate).

### Step 3: Visibility Demotion
1. Demote over-exposed `pub` functions unreferenced outside their crate to `pub(crate)`.
2. Demote over-exposed helpers unreferenced outside their defining file to private `fn`.
3. Encapsulate crate-internal submodules from `pub mod` to `pub(crate) mod`.

---

## 5. Semantic SRP Review (The Agent's Cognitive Role)

While static scripts flag quantitative metrics (lines > 800, public fields > 12), **evaluating SRP requires semantic domain reasoning by the Agent**:

1. **Hotspot Inspection:**
   When `audit_code_quality.py` flags an oversized file or a struct with mixed responsibilities, read the source to identify distinct conceptual domains.
2. **Domain Boundary Identification:**
   For example, in `crates/physical_memory/src/map.rs`:
   - **Responsibility A (Dispatch Infrastructure):** 64 KB memory bank callback dispatch table (`MemoryBank`, `BankHandler`).
   - **Responsibility B (System Topology Presets):** Machine preset topologies (`build_preset_bank_map`, `BANK_MAP_BARE`, `BANK_MAP_STANDARD`, `BANK_MAP_EXPANDED`).
3. **Decomposition Proposal & Execution:**
   - Extract secondary domain into a dedicated cohesive submodule (`presets.rs`).
   - Maintain 3-tier re-exports at the crate root (`src/<crate>.rs`) for zero downstream breaking changes.
   - Add 1:1 modular unit test parity (`tests/test_presets.rs`).
   - Delegate execution to [`refactor-split-module`](../refactor-split-module/SKILL.md).

---

## 6. Verification Gate & Definition of Done

After completing auditing, pruning, or visibility adjustments, always verify workspace integrity:
```powershell
cargo fmt --all -- --check
python tools/harness/pre_flight.py
python tools/harness/run_tests.py --unit
python tools/harness/run_tests.py --integration
```
Ensure all quality gates and architecture rules pass with 100% green status.

---

## 7. Execution Mode: Subagent Delegation

- **Execution Host:** **Isolated Subagent** (child context sandbox).
- **Model Tier:** `Gemini Flash Low` / `Medium`
- **Context Savings:** Shields the main conversation from thousands of lines of workspace scan logs, callers lists, and AST grep outputs.
- **Subagent Task Template:**
  - `TaskName`: "Code Quality Audit & Pruning: <scope>"
  - `TaskSummary`: "Audits dead code, test-only zombies, visibility leaks, and SRP cohesion."
  - `Prompt`:
    ```markdown
    Execute on-demand code quality audit and pruning across `<SCOPE>`.
    Follow .agents/skills/audit-code-quality/SKILL.md:
    1. Run `python tools/harness/audit_code_quality.py --all`.
    2. Triage zombies vs Host I/O boundaries.
    3. Prune confirmed dead symbols and demote leaked visibility.
    4. Verify via `python tools/harness/pre_flight.py` and unit tests.
    5. Return strictly the Code Quality Audit & Pruning Report below.
    ```
- **Return Contract (Mandatory Structured Output):**
  ```markdown
  ### 🛡️ Code Quality Audit & Pruning Report
  - **Scope Scanned:** `<scope>`
  - **Dead Code Pruned:** <count> symbols
  - **Test-Only Zombies Handled:** <count> retained (Host I/O) / <count> pruned
  - **Visibility Demoted:** <count> symbols (`pub` -> `pub(crate)` / private)
  - **SRP / Cohesion Decompositions:** <count> files/structs
  - **Skills Catalog Sync:** [PASS (all synchronized) | <count> discrepancies]
  - **Script Locality & Governance:** [PASS (all properly placed) | <count> anomalies]
  - **Design Specs Sync:** [PASS (all synchronized) | <count> drifted]
  - **Verification:** `pre_flight.py` (PASS), `cargo test` (PASS)
  ```
