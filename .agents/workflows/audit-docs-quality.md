---
name: audit-docs-quality
description: Run full-repository documentation and agent governance quality audit covering design sync, vault links, size limits, skills catalog, and script placement
---

# Workflow: Documentation & Agent Governance Quality Audit

Use this workflow to audit documentation health, Obsidian design specifications synchronization, vault link integrity, constitutional size ceilings, agent skills catalog alignment, script locality, and workflow-skill symmetry.

---

## 1. Zero-Parameter Run (`/audit-docs-quality`)
When invoked without parameters:
1. **Execute Universal Documentation & Governance Audit:**
   ```powershell
   python tools/harness/audit_docs_quality.py --all
   ```
2. **Execute Pre-Flight Quality Gate (includes automated architecture tests):**
   ```powershell
   python tools/harness/pre_flight.py
   ```

---

## 2. Targeted Audit Commands
- **Audit Design Specifications Drift:**
   ```powershell
   python tools/harness/audit_docs_quality.py --design-sync
   ```
- **Inspect Specific Spec Diff:**
   ```powershell
   python tools/harness/audit_docs_quality.py --design-diff <doc_name>
   ```
- **Bump Verified Spec Checkpoint to HEAD:**
   ```powershell
   python tools/harness/audit_docs_quality.py --design-bump <doc_name>
   ```
- **Audit Obsidian Vault Linking & Graph Integrity:**
   ```powershell
   python tools/harness/audit_docs_quality.py --vault-links
   ```
- **Audit Constitutional Byte Size Ceilings (`AGENTS.md` <= 14KB, rules <= 23KB):**
   ```powershell
   python tools/harness/audit_docs_quality.py --size-limits
   ```
- **Audit Agent Skills Catalog (`docs/ai_agents.md`):**
   ```powershell
   python tools/harness/audit_docs_quality.py --skills
   ```
- **Audit Script Locality & Harness Governance:**
   ```powershell
   python tools/harness/audit_docs_quality.py --scripts
   ```
- **Audit Workflow-Skill Symmetry & Rule Companion Coverage:**
   ```powershell
   python tools/harness/audit_docs_quality.py --governance
   ```
- **Audit YAML Frontmatter & Inverted Pyramid Structure:**
   ```powershell
   python tools/harness/audit_docs_quality.py --frontmatter
   ```
- **Audit Design Docs Reflection & Delegation in Agent Rules:**
   ```powershell
   python tools/harness/audit_docs_quality.py --rules-delegation
   ```
- **Audit Semantic Documentation-to-Code Parity (Double-Check Engine):**
   ```powershell
   python tools/harness/audit_docs_quality.py --semantic-sync
   ```

---

## 3. Remediation Procedure
Follow the detailed playbooks in [`.agents/skills/audit-docs-quality/SKILL.md`](../skills/audit-docs-quality/SKILL.md):
1. **Design Docs Drift:** Inspect code diffs via `--design-diff <doc>` and reconcile specifications under `Obsidian/Amiga/Design/`. Once verified, bump checkpoint via `--design-bump <doc>`.
2. **Vault Linking Discrepancies:** Resolve broken markdown links, missing target files, or unescaped paths per [`.agents/rules/vault-linking-and-graph-integrity.md`](../rules/vault-linking-and-graph-integrity.md).
3. **Size Limit Violations:** Modularize constitutional rules exceeding 14,000 bytes into `.agents/rules/`, and trim rule files exceeding 23,000 bytes per [`.agents/rules/information-hierarchy.md`](../rules/information-hierarchy.md).
4. **Skills Catalog Drift:** Add missing skills or prune deleted skills in [`docs/ai_agents.md`](../../docs/ai_agents.md).
5. **Script Locality & Harness Governance:** Relocate single-consumer scripts to `.agents/skills/<skill>/scripts/` and promote multi-consumer scripts to `tools/harness/`.
6. **Workflow & Rule Governance:** Provide companion slash-commands for procedural skills and ensure all active remediation rules maintain companion skills.
7. **Design Docs Reflection in Rules:** Ensure every living design specification in `Obsidian/Amiga/Design/` is explicitly referenced in its governing agent rule so pair-programming agents are delegated to the specification during coding.
8. **Semantic Parity Discrepancies:** Reconcile hexadecimal offsets, register names, memory boundaries, or crate topologies between `Obsidian/Amiga/Design/` and Rust code in `crates/` to eliminate subtle documentation bugs.

---

## 4. The Verbal Double-Check (Self-Audit & Heuristic Verification)

Beyond deterministic script passes, the auditor (agent or human) must explicitly verify the **5 Non-Negotiable Conscience Questions**:
1. 🧠 **Spec Freshness Review:** Did any implementation or refactoring in recent turns introduce subtle behavioral shifts, register side-effects, or new constants not yet recorded in `Obsidian/Amiga/Design/*.md`?
2. 🚫 **Anti-Nudge Review (`structural-root-cause.md`):** Are all beam coordinates, cycle counters, and branch thresholds derived from physical silicon specs rather than empirical $\pm 1$ / $\pm 2$ symptom nudges?
3. 🔬 **Assertion Density & Genuine Test Review (`unit-testing-policy.md`):** Do unit tests genuinely verify register mutations and state machine progressions, or do they contain trivial/placeholder assertions?
4. 📢 **Spec Conflict Escalation (`spec-compliance.md`):** If an external test vector or reference emulator diverged from our specifications, was it escalated to the user before changing code?
5. 🧹 **Clean-Break Refactoring (`clean-break-refactoring.md`):** Were old methods, legacy aliases, and temporary shims completely deleted rather than left behind?

---

## 5. Output Contract
Conclude with the standardized summary report:
```markdown
### 📚 Documentation & Governance Quality Audit Report
- **Design Specs Sync:** [PASS | <count> drifted]
- **Vault Linking & Graph Integrity:** [PASS | <count> broken links] (verified <count> links)
- **Constitutional Size Limits:** [PASS | <count> violations] (`AGENTS.md` <= 14KB, rules <= 23KB)
- **Skills Catalog Sync:** [PASS | <count> discrepancies]
- **Script Locality & Harness Governance:** [PASS | <count> anomalies]
- **Workflow & Skill Governance:** [PASS | <count> issues (<count> candidates)]
- **Frontmatter Compliance:** [PASS | <count> missing frontmatter]
- **Design Docs to Rules Reflection:** [PASS | <count> unreflected] (verified <count> specs)
- **Semantic Documentation-to-Code:** [PASS | <count> discrepancies] (registers, memory map, crate topology, signals, quirks)
- **Verbal Double-Check Conscience Review:** [CONFIRMED - 5/5 heuristics verified]
- **Verification:** `pre_flight.py` (PASS), `test_architecture_rules` (PASS)
```
