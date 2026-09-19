---
name: audit-docs-quality
description: Comprehensive documentation, Obsidian vault linking, constitutional size limits, and agent governance quality audit playbook.
---

# Recipe: Documentation & Agent Governance Quality Auditor Playbook

This skill provides on-demand verification across the repository's documentation ecosystem: synchronizing design specs with Rust commits, validating Obsidian vault graph integrity, enforcing constitutional size limits (`AGENTS.md` <= 14KB, rules <= 23KB), and verifying agent governance symmetry.

---

## 1. When to Trigger This Skill

- **Major Milestone Completion:** Audit documentation and skills catalog integrity upon milestone sign-off.
- **Specification Updates:** Verify that code changes across `crates/` have not left `Obsidian/Amiga/Design/` in a drifted state.
- **Governance Review:** Audit script locality between `tools/harness/` and skills, ensuring zero dark skills in `docs/ai_agents.md`.

---

## 2. The Seven Documentation Quality Pillars

### Pillar 1: Design Documentation & Code Drift Detection (`--design-sync`)
- **Deterministic Git Checkpoints:** Every code-backed design specification in `Obsidian/Amiga/Design/` records `tracked_paths` and `last_synced_commit` in its YAML frontmatter.
- **Automated Drift Detection:** Computes `git rev-list --count <last_synced_commit>..HEAD -- <tracked_paths>`.
- **Differential Inspection:** Inspect the exact code diff since the last synchronization via `--design-diff <doc>`.
- **Checkpoint Stamping:** Once verified or updated, bump to HEAD via `--design-bump <doc>`.

### Pillar 2: Obsidian Vault Linking & Graph Integrity (`--vault-links`)
- **Zero Broken Links:** All markdown links `[text](path.md)` under `Obsidian/Amiga/Design/` must resolve to existing files.
- **Dual-Layer Linking:** Contextual inline links accompanied by bottom structural reference links per [`.agents/rules/vault-linking-and-graph-integrity.md`](../../rules/vault-linking-and-graph-integrity.md).

### Pillar 3: Constitutional Byte Size Ceilings (`--size-limits`)
- **AGENTS.md Ceiling ($\le 14,000$ bytes):** Must serve strictly as a lean architectural constitution and index; zero duplicated rule bodies per [`.agents/rules/information-hierarchy.md`](../../rules/information-hierarchy.md).
- **Rule Files Safety Ceiling ($\le 23,000$ bytes):** Individual files under `.agents/rules/*.md` and `GEMINI.md` must stay under 23 KB to eliminate silent prompt truncation at ~24 KB.

### Pillar 4: Agent Skills Catalog Synchronization (`--skills`)
- **Complete Skill Index Integrity:** Every active skill directory under `.agents/skills/` containing a `SKILL.md` must be cataloged in [`docs/ai_agents.md`](../../../docs/ai_agents.md).
- **Zero Phantom References:** Every skill linked in `docs/ai_agents.md` must actually exist on disk.

### Pillar 5: Two-Way Script Locality & Harness Governance (`--scripts`)
- **Harness Reservation:** `tools/harness/` is reserved strictly for universal, shared infrastructure used across multiple subsystems (pre-flight gates, git hooks, universal test runners, global rules).
- **Rule A (Specialized Locality):** Any script in `tools/harness/` referenced by $\le 1$ skill or workflow (and not part of global pre-commit/pre-flight) must be relocated to `.agents/skills/<skill>/scripts/`.
- **Rule B (Shared Promotion):** Any script inside `.agents/skills/<skill>/scripts/` referenced by $> 1$ distinct skills or workflows must be promoted into `tools/harness/` to avoid cross-skill leakage.

### Pillar 6: Workflow, Skill & Rule Governance (`--governance`)
- **Workflow-to-Skill Backing:** Every workflow in `.agents/workflows/` must have a companion specialized skill in `.agents/skills/` or explicitly declare its underlying skills.
- **Skill-to-Workflow Promotion Candidates:** Milestone, batch, or multi-step maintenance procedures that operate across the repository are candidate workflows deserving dedicated `/slash-command` entrypoints in `.agents/workflows/`.
- **Rule-to-Skill Governance:**
  - Active remediation rules must have corresponding executable skills in `.agents/skills/`.
  - Passive invariant rules must remain lean architectural constraints without redundant companion skills.

### Pillar 7: Frontmatter & Inverted Pyramid Structure (`--frontmatter`)
- **Line 1 Frontmatter:** Every specification under `Obsidian/Amiga/Design/` must define YAML frontmatter starting on Line 1 with `tags: [spec, ...]`.
- **Inverted Pyramid:** Decisive architectural conclusions, invariants, and memory maps presented in opening 20–50 lines before implementation details.

### Pillar 8: Design Docs to Agent Rules Reflection & Delegation (`--rules-delegation`)
- **Coding Delegation Invariant:** Every design specification in `Obsidian/Amiga/Design/` must be explicitly reflected in at least one agent rule in `.agents/rules/*.md` or `AGENTS.md`.
- **Operational Linkage:** Guarantees that AI pair-programming agents executing coding tasks in any subsystem (custom chips, CPU, memory bus, peripherals, GUI, testing) are governed by and delegated to the authoritative design documentation.

### Pillar 9: Semantic Documentation-to-Code Parity (`--semantic-sync`)
- **Deterministic Field Validator (The Double-Check Engine):** Validates five deep semantic dimensions against live Rust source code:
  1. *Custom Register Matrix:* 100+ offsets, R/W permissions, and chip ownership (`Agnus`, `Denise`, `Paula`) vs `crates/config/src/registers.rs`.
  2. *Memory Map Boundaries:* 24-bit physical ranges and Gary bank constants vs `crates/memory_bus/src/memory_bus.rs`.
  3. *Crate Topology Sync:* 100% bidirectional parity between Mermaid graph in `General Architecture.md` and `Cargo.toml`.
  4. *Cross-Chip Signal Parity:* All action methods and `poll_*` queries in `Cross-Chip Signals Catalog` confirmed in `crates/*/src/`.
  5. *Silicon Quirks Coverage:* All 13 hardware errata in `Platform Quirks Catalog` covered by active regression test sentinels.

---

## 3. The Verbal Double-Check Protocol (Heuristic Verification)

Automated Python scripts guarantee syntactic and boundary correctness, but cannot detect semantic drift caused by speculative coding or forgotten rules. Conclude every audit with the **5 Heuristic Questions**:
1. 🧠 **Spec Freshness Review:** Did recent code changes alter chip behavior or registers without updating `Obsidian/Amiga/Design/*.md`?
2. 🚫 **Anti-Nudge Review (`structural-root-cause.md`):** Are all beam coordinates and delays silicon-verified rather than empirical $\pm 1$ / $\pm 2$ symptom patches?
3. 🔬 **Assertion Density & Genuine Test Review (`unit-testing-policy.md`):** Do unit tests genuinely verify chip behavior and state changes, or do they only assert trivial boilerplate?
4. 📢 **Spec Conflict Escalation (`spec-compliance.md`):** Were any conflicts between reference test suites and internal design specs escalated to the user before changing code?
5. 🧹 **Clean-Break Refactoring (`clean-break-refactoring.md`):** Were old methods, legacy aliases, and temporary shims completely deleted rather than left behind?

---

## 4. CLI Audit Workflow

```powershell
# Run all documentation and governance audits
python tools/harness/audit_docs_quality.py --all

# Audit only design specifications drift
python tools/harness/audit_docs_quality.py --design-sync

# Inspect diff for a specific drifted specification
python tools/harness/audit_docs_quality.py --design-diff Denise.md

# Bump checkpoint to current HEAD
python tools/harness/audit_docs_quality.py --design-bump Denise.md

# Audit vault link integrity
python tools/harness/audit_docs_quality.py --vault-links

# Audit constitutional size ceilings
python tools/harness/audit_docs_quality.py --size-limits

# Audit skills catalog in docs/ai_agents.md
python tools/harness/audit_docs_quality.py --skills

# Audit script placement governance
python tools/harness/audit_docs_quality.py --scripts

# Audit workflow and skill governance
python tools/harness/audit_docs_quality.py --governance

# Audit design docs reflection and delegation in agent rules
python tools/harness/audit_docs_quality.py --rules-delegation

# Audit semantic documentation-to-code parity (The Double-Check Engine)
python tools/harness/audit_docs_quality.py --semantic-sync
```
