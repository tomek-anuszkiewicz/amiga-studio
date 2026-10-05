---
name: audit-docs-quality
description: Comprehensive documentation, Obsidian vault linking, constitutional size limits, and agent governance quality audit playbook.
---

# Recipe: Documentation & Agent Governance Quality Auditor Playbook

This skill provides on-demand verification across the repository's documentation ecosystem: synchronizing design specs with Rust commits, validating Obsidian vault graph integrity, enforcing constitutional size limits (`AGENTS.md` <= 14KB, rules <= 23KB), and verifying skill and rule governance.

---

## 1. When to Trigger This Skill

- **Major Milestone Completion:** Audit documentation and skills catalog integrity upon milestone sign-off.
- **Specification Updates:** Verify that code changes across `crates/` have not left `Obsidian/Amiga/Design/` in a drifted state.
- **Governance Review:** Audit script locality between `tools/harness/` and skills, ensuring zero dark skills in `docs/ai_agents.md`.

---

## 2. Documentation Quality Checks

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
- **Rule Files Safety Ceiling ($\le 23,000$ bytes):** Individual referenced files under `.agents/rules/*.md` must stay under the repository's 23 KB instruction budget. Codex project-instruction discovery has its own configurable byte limit.

### Pillar 4: Agent Skills Catalog Synchronization (`--skills`)
- **Complete Skill Index Integrity:** Every active skill directory under `.agents/skills/` containing a `SKILL.md` must be cataloged in [`docs/ai_agents.md`](../../../docs/ai_agents.md).
- **Zero Phantom References:** Every skill linked in `docs/ai_agents.md` must actually exist on disk.

### Pillar 5: Two-Way Script Locality & Harness Governance (`--scripts`)
The canonical placement and catalog policy is [agent tooling maintenance](../../../docs/ai_agents.md#agent-tooling-maintenance).
- **Harness Reservation:** `tools/harness/` is reserved strictly for universal, shared infrastructure used across multiple subsystems (pre-flight gates, explicit validation commands, universal test runners, global rules).
- **Rule A (Specialized Locality):** Any script in `tools/harness/` referenced by $\le 1$ skill (and not part of global pre-commit/pre-flight) must be relocated to `.agents/skills/<skill>/scripts/`.
- **Rule B (Shared Promotion):** Any script inside `.agents/skills/<skill>/scripts/` referenced by $> 1$ distinct skills must be promoted into `tools/harness/` to avoid cross-skill leakage.

### Pillar 6: Skill & Rule Governance (`--governance`)
- Reusable procedures live in `.agents/skills/<name>/SKILL.md`, with `name` and `description` in YAML frontmatter.
- Invoke them as `$skill-name` or let Codex select them from the task and description.
- Active remediation rules require corresponding skills; passive invariants remain lean rules.
- Supporting references belong to their owning skill and count as the same script consumer.
- Presence checks confirm configured files and mappings, not correct skill activation or complete semantic coverage. Review rule brevity and procedure ownership separately.

### Pillar 7: Frontmatter & Inverted Pyramid Structure (`--frontmatter`)
- **Line 1 Frontmatter:** Every specification under `Obsidian/Amiga/Design/` must define YAML frontmatter starting on Line 1 with `tags: [spec, ...]`.
- **Inverted Pyramid:** Decisive architectural conclusions, invariants, and memory maps presented in opening 20–50 lines before implementation details.

### Pillar 8: Semantic Documentation-to-Code Parity (`--semantic-sync`)
- **Deterministic Field Validator (The Double-Check Engine):** Validates five deep semantic dimensions against live Rust source code:
  1. *Custom Register Matrix:* 100+ offsets, R/W permissions, and chip ownership (`Agnus`, `Denise`, `Paula`) vs `crates/config/src/registers.rs`.
  2. *Memory Map Boundaries:* 24-bit physical ranges and Gary bank constants vs `crates/memory_bus/src/memory_bus.rs`.
  3. *Crate Topology Sync:* 100% bidirectional parity between Mermaid graph in `General Architecture.md` and `Cargo.toml`.
  4. *Cross-Chip Signal Parity:* All action methods and `poll_*` queries in `Cross-Chip Signals Catalog` confirmed in `crates/*/src/`.
  5. *Silicon Quirks Coverage:* All 13 hardware errata in `Platform Quirks Catalog` covered by active regression test sentinels.

### Pillar 9: Workflow Invariants (`--workflow-invariants`)
- Verify that `DIARY.md` exists, contains Section 10, and has nondecreasing parsed entry timestamps.
- Verify that `ROADMAP.md` exists and contains no completed `[x]` task checkboxes.
- These checks do not verify an entry in every commit, milestone compaction, or full roadmap completion semantics.
- Rule files need no manual audit registration. Reach specifications through domain rules or the design index; do not duplicate a direct rule link for every document.

---

## 3. The Verbal Double-Check Protocol (Heuristic Verification)

Automated checks validate their configured patterns and boundaries. They do not establish complete semantic agreement or hardware fidelity; report the inspected scope and follow with manual review where required. Conclude every audit with the **5 Heuristic Questions**:
1. 🧠 **Spec Freshness Review:** Did recent code changes alter chip behavior or registers without updating `Obsidian/Amiga/Design/*.md`?
2. 🚫 **Anti-Nudge Review (`structural-root-cause.md`):** Are all beam coordinates and delays silicon-verified rather than empirical $\pm 1$ / $\pm 2$ symptom patches?
3. 🔬 **Assertion Density & Genuine Test Review (`unit-testing-policy.md`):** Do unit tests genuinely verify chip behavior and state changes, or do they only assert trivial boilerplate?
4. 📢 **Spec Conflict Escalation (`spec-compliance.md`):** Were any conflicts between reference test suites and internal design specs escalated to the user before changing code?
5. 🧹 **Clean-Break Refactoring (`clean-break-refactoring.md`):** Were old methods, legacy aliases, and temporary shims completely deleted rather than left behind?

---

## Default Execution

When invoked without a narrower scope, run:

```powershell
python tools/harness/audit_docs_quality.py --all
python tools/harness/pre_flight.py
```

Only after both checks succeed, use [`$index-amiga-rag`](../index-amiga-rag/SKILL.md)
to index the accepted documentation. Do not index failed audits or stale drafts.
For remediation, reconcile design diffs before bumping checkpoints, repair vault
links, maintain the skill catalog, and apply the script placement rules above.

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

# Audit skill and rule governance
python tools/harness/audit_docs_quality.py --governance

# Audit diary structure and timestamps, and retained roadmap checkboxes
python tools/harness/audit_docs_quality.py --workflow-invariants

# Audit semantic documentation-to-code parity (The Double-Check Engine)
python tools/harness/audit_docs_quality.py --semantic-sync
```


## 5. Output Contract
Conclude with the standardized summary report:
Report observed results; mark checks that were not run instead of copying the example PASS values.
```markdown
### 📚 Documentation & Governance Quality Audit Report
- **Design Specs Sync:** [PASS | <count> drifted]
- **Vault Linking & Graph Integrity:** [PASS | <count> broken links] (verified <count> links)
- **Constitutional Size Limits:** [PASS | <count> violations] (`AGENTS.md` <= 14KB, rules <= 23KB)
- **Skills Catalog Sync:** [PASS | <count> discrepancies]
- **Script Locality & Harness Governance:** [PASS | <count> anomalies]
- **Skill & Rule Governance:** [PASS | <count> issues]
- **Frontmatter Compliance:** [PASS | <count> missing frontmatter]
- **Semantic Documentation-to-Code:** [PASS | <count> discrepancies] (registers, memory map, crate topology, signals, quirks)
- **Workflow Invariants:** [PASS | <count> issues] (diary structure/timestamps, roadmap file/checkboxes)
- **Manual Review:** <inspected scope and unresolved questions; mark unrun reviews>
- **Verification:** `pre_flight.py` (PASS), `test_architecture_rules` (PASS)
```
