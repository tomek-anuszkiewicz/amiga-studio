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
2. **Execute Pre-Flight Quality Gate:**
   ```powershell
   python tools/harness/pre_flight.py
   ```
3. **Execute Architecture Rules:**
   ```powershell
   cargo test -p test_runner --test test_architecture_rules -- --quiet
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

---

## 3. Remediation Procedure
Follow the detailed playbooks in [`.agents/skills/audit-docs-quality/SKILL.md`](../skills/audit-docs-quality/SKILL.md):
1. **Design Docs Drift:** Inspect code diffs via `--design-diff <doc>` and reconcile specifications under `Obsidian/Amiga/Design/`. Once verified, bump checkpoint via `--design-bump <doc>`.
2. **Vault Linking Discrepancies:** Resolve broken markdown links, missing target files, or unescaped paths per [`.agents/rules/vault-linking-and-graph-integrity.md`](../rules/vault-linking-and-graph-integrity.md).
3. **Size Limit Violations:** Modularize constitutional rules exceeding 14,000 bytes into `.agents/rules/`, and trim rule files exceeding 23,000 bytes per [`.agents/rules/information-hierarchy.md`](../rules/information-hierarchy.md).
4. **Skills Catalog Drift:** Add missing skills or prune deleted skills in [`docs/ai_agents.md`](../../docs/ai_agents.md).
5. **Script Locality & Harness Governance:** Relocate single-consumer scripts to `.agents/skills/<skill>/scripts/` and promote multi-consumer scripts to `tools/harness/`.
6. **Workflow & Rule Governance:** Provide companion slash-commands for procedural skills and ensure all active remediation rules maintain companion skills.

---

## 4. Output Contract
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
- **Verification:** `pre_flight.py` (PASS), `test_architecture_rules` (PASS)
```
