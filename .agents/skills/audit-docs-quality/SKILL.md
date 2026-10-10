---
name: audit-docs-quality
description: Review concise instructions and agent governance, and route design, vault, semantic and workflow checks to their owning skills.
---

# Recipe: Documentation & Agent Governance Quality Auditor Playbook

This skill reviews instruction brevity and audits agent governance. Design synchronization, vault integrity, semantic comparisons and workflow checks are owned and executed by the skills linked below; this manual review does not replace those domain checks.

---

## 1. When to Trigger This Skill

- **Major Milestone Completion:** Audit documentation governance upon milestone sign-off.
- **Specification Updates:** Verify that code changes across `crates/` have not left `Obsidian/Amiga/Design/` in a drifted state.
- **Governance Review:** Review script ownership and required procedure routing.

---

## 2. Documentation Quality Checks

### Pillar 1: Scoped Design Documentation Review
[sync-design-docs](../sync-design-docs/SKILL.md) owns the pre-commit review of intended changes and documents selected by `tracked_paths`.

### Pillar 2: Obsidian Vault Linking & Graph Integrity (`--vault-links`)
[obsidian-vault-linking](../obsidian-vault-linking/SKILL.md) owns link checks.

### Instruction Brevity and Script Ownership (Manual Review)
Follow [information hierarchy](../../rules/information-hierarchy.md): keep instructions concise, preserve required routing, and remove duplicated procedures. There are no fixed byte ceilings.
Review script and function ownership using actual callers and responsibilities, including individual switches in mixed scripts. Follow [agent tooling maintenance](../../references/agent-tooling.md#agent-tooling-maintenance); reference counts alone do not determine placement.

### Skill and Rule Ownership (Manual Review)
Review obligations in the affected rules and confirm that required procedures are
reachable through accurate links to their owning skills. Report broken routing
and duplicated instructions using the actual contents; no active/passive
classification registry or skill-name symmetry check is maintained.

### Pillar 7: Frontmatter & Inverted Pyramid Structure (`--frontmatter`)
[obsidian-vault-linking](../obsidian-vault-linking/SKILL.md) owns YAML frontmatter checks.

### Pillar 8: Semantic Documentation-to-Code Parity (`--semantic-sync`)
[audit-semantic-parity](../audit-semantic-parity/SKILL.md) owns static comparisons and manual semantic review.

### Pillar 9: Workflow Invariants (`--workflow-invariants`)
[compact-diary](../compact-diary/SKILL.md) owns diary structure/chronology checks; [roadmap-maintenance](../roadmap-maintenance/SKILL.md) owns roadmap retention checks.

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
python tools/verification/verify.py commit
```

Only after the scoped documentation review and verification succeed, use [`$index-amiga-rag`](../index-amiga-rag/SKILL.md)
to index the accepted documentation. Do not index failed audits or stale drafts.
For remediation, follow the owning skill to reconcile the intended code changes
and selected specifications, repair vault links and review actual script ownership.

## 5. Output Contract
Conclude with the standardized summary report:
Report observed results; mark checks that were not run instead of copying the example PASS values.
```markdown
### 📚 Documentation & Governance Quality Audit Report
- **Design Specs Sync:** [PASS | <count> drifted]
- **Vault Linking & Graph Integrity:** [PASS | <count> broken links] (verified <count> links)
- **Instruction Brevity and Script Ownership:** <manual review scope and findings; mark unrun>
- **Skill and Rule Ownership:** <manual routing review scope and findings; mark unrun>
- **Frontmatter Compliance:** [PASS | <count> missing frontmatter]
- **Semantic Documentation-to-Code:** [PASS | <count> discrepancies] (registers, memory map, crate topology, signals, quirks)
- **Workflow Invariants:** [PASS | <count> issues] (diary structure/timestamps, roadmap file/checkboxes)
- **Manual Review:** <inspected scope and unresolved questions; mark unrun reviews>
- **Verification:** `verify.py` (PASS), `check_architecture.py` (PASS)
```
