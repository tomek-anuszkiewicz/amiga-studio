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

---

## 3. CLI Audit Workflow

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
```
