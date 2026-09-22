---
trigger: model_decision
description: Maintaining and synchronizing technical design documentation under Obsidian/Amiga/Design/ with active emulator code and architecture.
---

# Design Documentation Maintenance & Code Synchronization Rule

This rule governs the continuous synchronization, cleanup, and maintenance of technical design documentation under [`Obsidian/Amiga/Design/`](../../Obsidian/Amiga/Design/).

---

## 1. Living Design Synchronization

Whenever implementing, refactoring, or modifying any subsystem in this repository:
- You **must update the corresponding design document in [`Obsidian/Amiga/Design/`](../../Obsidian/Amiga/Design/)** whenever any architectural decision, timing model, data structure, register bitfield, or hardware quirk has changed or was clarified.
- **Milestone & Architectural Change Synchronization:** Formal design documentation updates and checkpoint bumping (`last_synced_commit`) occur upon completing **minor roadmap points** (e.g. Step 1.1, 1.2, 2.1) or when a genuine architectural design change occurs. Routine intermediate commits (bug fixes, internal refactorings, unit test additions) are fully covered by the Tier 2 Grace Tolerance Window ($\le 100$ commits, $\le 30$ days) and do not require per-commit documentation editing.
- The design specifications in `Obsidian/Amiga/Design/` are living, permanent specifications and must always stay synchronized with the active code.

---

## 2. Post-Implementation Cleanup & Code Duplication Removal

Design specifications often contain tentative draft snippets, forward-looking proposals, or hypothetical code sketches written prior to implementation:
- Whenever completing a roadmap step or implementing a feature, you **must review and clean up the relevant design documents**:
  1. Remove obsolete speculative code and draft proposals.
  2. Ensure the document reflects the finalized, living architectural reality.
  3. **Design documents must never duplicate code that has already been written**: replace duplicate Rust code blocks with concise architectural descriptions, tables, and direct markdown links to living Rust source files.

---

## 3. No Line Limits on Technical Documentation

Design specifications and reference manuals in [`Obsidian/Amiga/Design/`](../../Obsidian/Amiga/Design/) have **no line count limits**. They should be as long, exhaustive, and detailed as necessary to serve as complete, living single-source-of-truth specifications. Never artificially split, truncate, or omit architectural details from markdown documentation.

---

## 4. Deterministic Git Commit Checkpoints (`last_synced_commit`) & Dual-Tier Synchronization

Every code-backed architectural specification under [`Obsidian/Amiga/Design/`](../../Obsidian/Amiga/Design/) maintains a cryptographic review trail in its YAML frontmatter:
- `tracked_paths`: list of relative repository directories whose Rust code backs the specification (e.g. `["crates/denise/src", "crates/sprites/src"]`).
- `last_synced_commit`: short or full Git commit hash at which the document was last audited and confirmed to match code reality.
- `last_synced_date`: ISO date (`YYYY-MM-DD`) of the last audit.

### A. Dual-Tier Synchronization Architecture:
1. **Tier 1: Atomic Substantive Synchronization (Immediate Bundling):**
   - Whenever an architectural specification is genuinely authored, updated, or clarified (e.g. registers added, timing tables adjusted, behavioral quirks documented), `last_synced_commit` and `last_synced_date` **must be updated within that exact same commit**.
   - **Zero Empty Sync Commits:** Standalone commits containing only frontmatter `last_synced_commit` changes without substantive markdown content improvements are strictly prohibited during normal development.
2. **Tier 2: Grace Tolerance Window & Periodic Staleness Heartbeat:**
   - Routine code changes in `tracked_paths` (refactorings, internal helpers, test additions) that do not alter documented architecture are normal and expected.
   - Code drift is permitted within an allowable grace window:
     - **Commit Threshold:** $\le 100$ commits since `last_synced_commit`.
     - **Calendar Threshold:** $\le 30$ calendar days since `last_synced_date`.
   - Tracked specifications within this tolerance window pass quality audits (`[PASS / TOLERATED]`) and do not fail `audit_docs_quality.py`.
   - Only when a specification exceeds either threshold ($>100$ commits or $>30$ days without an audit) is it marked as `[STALE]`, triggering an audit issue that prompts a formal architectural review.

### B. Audit & Checkpoint Bumping:
1. **Automated Drift Detection:** Verified by Pillar 1 of `audit-docs-quality`:
   ```powershell
   python tools/harness/audit_docs_quality.py --design-sync
   ```
2. **Differential Review:** When a document is stale or being updated, inspect the code diff since the checkpoint:
   ```powershell
   python tools/harness/audit_docs_quality.py --design-diff <doc_name>
   ```
3. **Checkpoint Stamping:** Once the document is updated with substantive changes (or verified after exceeding the 100-commit / 30-day threshold), stamp the new HEAD commit:
   ```powershell
   python tools/harness/audit_docs_quality.py --design-bump <doc_name>
   ```
4. **Meta Notes Exemption:** Conceptual design notes that do not map to concrete crates (e.g. `Rust Guidelines.md`, `Testing Strategy and Quality Assurance.md`) omit `tracked_paths` and are safely skipped by the drift detector.

---

## 5. Crate Dependency Graph Maintenance

Whenever crates or dependencies in `Cargo.toml` (new crates, added/removed inter-crate dependencies, or key external dependencies) are modified:
- You **must update the Crate Dependency Mermaid Graph in [`Obsidian/Amiga/Design/General Architecture.md`](../../Obsidian/Amiga/Design/General%20Architecture.md#2-workspace-crate-architecture--dependencies)**.

---

## 6. Knowledge Graph & Linking Compliance
 
All modifications to design specifications must adhere strictly to the YAML frontmatter properties, dual-layer linking, and inverted pyramid structure codified in [`.agents/rules/vault-linking-and-graph-integrity.md`](vault-linking-and-graph-integrity.md) and [`.agents/rules/information-hierarchy.md`](information-hierarchy.md), while maintaining the practitioner voice standard in [`.agents/rules/practitioner-voice-and-tone.md`](practitioner-voice-and-tone.md).

---

## 7. Design Documentation Reflection & Delegation in Agent Rules

Design specifications under [`Obsidian/Amiga/Design/`](../../Obsidian/Amiga/Design/) serve as the authoritative architectural ground truth. To guarantee that autonomous pair-programming agents strictly follow these specifications during coding tasks, every design specification and its architectural domain must be explicitly reflected in agent rules (`.agents/rules/*.md` and `AGENTS.md`) with explicit operational delegation and links:
- **Zero Unreflected Design Specs:** Every living design specification in `Obsidian/Amiga/Design/` must be referenced in its governing agent rule so agents are directed to the specification when implementing or refactoring code in that domain.
- **Continuous Audit:** Automated governance audits (`python tools/harness/audit_docs_quality.py --rules-delegation`) enforce 100% reflection and alert on any unreflected design documents with actionable remediation recommendations.

---

## 8. Execution Skills: `sync-design-docs` & `audit-semantic-parity`

- Follow the operational procedure in [`sync-design-docs`](../skills/sync-design-docs/SKILL.md) to inspect git diffs, update affected living specifications, prune speculative draft snippets, update Mermaid dependency graphs, stamp git checkpoints, and verify linking integrity.
- Follow the operational procedure in [`audit-semantic-parity`](../skills/audit-semantic-parity/SKILL.md) to conduct inference-driven bidirectional audits evaluating code-to-docs parity (blind spots, undocumented logic) and docs-to-code parity (hallucinations, ghost features, spec drift).

---

## 9. Two-Way Script Locality & Harness Placement Governance

To prevent script sprawl and maintain clear boundaries between global infrastructure and specialized procedures:
- **`tools/harness/` Reservation:** `tools/harness/` is reserved strictly for universal, shared infrastructure used across multiple subsystems (pre-flight gates, pre-commit hooks, universal test runners, and workspace-wide rules).
- **Rule A (Specialized Locality):** Any script in `tools/harness/` referenced by $\le 1$ skill or workflow (and not part of global pre-commit/pre-flight) must be relocated to `.agents/skills/<skill>/scripts/`.
- **Rule B (Shared Promotion):** Any script inside `.agents/skills/<skill>/scripts/` referenced by $> 1$ distinct skills or workflows must be promoted into `tools/harness/` to avoid cross-skill coupling.
- **Continuous Audit:** Enforced via `python tools/harness/audit_docs_quality.py --scripts`.

---

## 10. Agent Skills Catalog Synchronization & Workflow-Skill Symmetry

The agent tooling catalog must stay synchronized with repository disk reality at all times:
- **Skills Catalog Integrity (`docs/ai_agents.md`):** Every active skill directory under `.agents/skills/` containing a `SKILL.md` must be documented in [`docs/ai_agents.md`](../../docs/ai_agents.md). Adding or removing a skill requires an immediate corresponding update to `docs/ai_agents.md` (zero phantom links permitted).
- **Workflow-to-Skill Backing:** Every workflow in `.agents/workflows/` must have a companion specialized skill in `.agents/skills/` or explicitly declare its underlying backing skills.
- **Continuous Audit:** Enforced via `python tools/harness/audit_docs_quality.py --skills` and `python tools/harness/audit_docs_quality.py --governance`.
