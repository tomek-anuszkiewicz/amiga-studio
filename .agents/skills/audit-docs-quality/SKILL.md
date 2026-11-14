---
name: audit-docs-quality
description: Review concise instructions and agent governance on request or at major milestone completion; route conditional checks to their owning skills.
---

# Documentation and Agent Governance Review

This skill reviews instruction brevity and audits agent governance. Specification compliance review, vault integrity, semantic comparisons and workflow checks belong to the skills linked below and run only at their own triggers; this manual review does not replace those domain checks.

---

## 1. When to Trigger This Skill

- **Major Milestone Completion:** Audit documentation governance during milestone sign-off.
- **Requested Governance Review:** Review script ownership and required procedure routing.

Ordinary code work and specification updates do not invoke this skill. The
batch preparation workflow owns scoped specification compliance review;
milestone closure owns broader semantic parity.

---

## 2. Documentation Quality Checks

### Pillar 1: Scoped Design Documentation Review (Pre-Commit)
[verify-spec-compliance](../verify-spec-compliance/SKILL.md) owns the batch review against approved requirements and compatible enrichment of documents selected by `tracked_paths`.

### Pillar 2: Obsidian Vault Linking & Graph Integrity (`--vault-links`)
[obsidian-vault-linking](../obsidian-vault-linking/SKILL.md) owns link checks.

### Instruction Brevity and Script Ownership (Manual Review)
Follow [document and instruction ownership](../../rules/information-hierarchy.md): keep instructions concise, preserve required routing, and remove duplicated procedures. There are no fixed byte ceilings.
Review script and function ownership using actual callers and responsibilities, including individual switches in mixed scripts. Follow [agent tooling maintenance](../../references/agent-tooling.md#agent-tooling-maintenance); reference counts alone do not determine placement.

### Skill and Rule Ownership (Manual Review)
Review obligations in the affected rules and confirm that required procedures are
reachable through accurate links to their owning skills. Report broken routing
and duplicated instructions using the actual contents; no active/passive
classification registry or skill-name symmetry check is maintained.

### Pillar 7: Frontmatter & Inverted Pyramid Structure (`--frontmatter`)
[obsidian-vault-linking](../obsidian-vault-linking/SKILL.md) owns YAML frontmatter checks.

### Pillar 8: Semantic Documentation-to-Code Parity (Milestone)
[audit-semantic-parity](../audit-semantic-parity/SKILL.md) owns static comparisons and manual semantic review.

### Pillar 9: Workflow Invariants (`--workflow-invariants`)
[compact-diary](../compact-diary/SKILL.md) owns diary structure/chronology checks; [roadmap-maintenance](../roadmap-maintenance/SKILL.md) owns roadmap retention checks.

---

## Manual Review

Assess the risks relevant to the affected instructions: duplicated or conflicting
obligations, inaccurate procedure links, and triggers that cause unrelated work.
Automated checks establish only their configured coverage; use manual review for
the meaning and scope of instructions.

---

## Default Execution

When invoked without a narrower scope, inspect the affected governance files,
their procedure links, and actual script ownership. Use verification results from
an enclosing commit or milestone workflow when that workflow applies; this audit
does not start the commit profile, specification compliance review, semantic parity,
or RAG indexing on its own. For remediation, follow the owning skill for the
specific finding.

## Report

Lead with concrete findings and affected paths. Summarize the inspected scope,
checks actually run, their results, and material verification gaps. Include
enclosing commit or milestone results only when that workflow applies.
