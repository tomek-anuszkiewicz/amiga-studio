---
name: TA-WF-Step-review-docs
description: Review specifications, retained knowledge, instruction governance and Design link/metadata integrity for a scoped candidate or selected stage.
---

# Assess the documentation lane

Apply [the result contract](../../references/workflow-contract.md) and
[independent review contract](../../references/review-common.md).
Assess selected documents and instructions against the shared
[information hierarchy](../../references/information-hierarchy.md) and
[specification compliance guidance](../../references/spec-compliance.md).
Assess the shared [prose and narrative criteria](../../references/narrative-criteria.md)
according to each document's genre and the assigned review scope.
Role: `TA-code-reviewer`; lane: `documentation`.
Inputs: exact baseline/candidate, assigned scope, relevant requirements, available
evidence and proposed partition when preparing a batch.

For major milestone sign-off, take the stage-level scope required by
[milestone completion criteria](../../../Obsidian/Amiga/Design/Verification/Testing%20Strategy%20and%20Quality%20Assurance.md#roadmap-and-milestone-completion); retain matching prior
assessment instead of repeating covered work.

1. Check input identities and fresh reviewer context. Read only
   [this lane's criteria](references/documentation.md) plus relevant requirements.
2. Independently inspect the selected candidate and affected interactions.
   Select affected requirements through Design `tracked_paths` and explicit
   requirement changes using the shared exact path-boundary rules. Include
   selected new files and both rename/delete endpoints. Compare documentation
   against approved requirements, implementation/tests and source-confirmed
   hardware knowledge in both directions. Classify compatible enrichment,
   unsupported claims, unmet requirements and genuine conflicts; preserve unique
   rationale and provenance. A selected stage-wide assessment covers the full
   stage and interactions, including unchanged affected documents/code, rather
   than only edited lines. Record overlap instead of repeating matching evidence.
3. For assigned Design documentation, run the full recursive link/metadata audit:
   `python .agents/skills/TA-audit-vault-links/scripts/audit_vault.py --vault-links --frontmatter`.
   Read the [vault schema and semantic criteria](../TA-audit-vault-links/references/vault-metadata.md)
   and assess contextual links and metadata meaning, including empty tag/link
   sets. Reuse overlapping current canonical structural evidence only where it
   applies. This Design audit does not replace the wider canonical frontmatter
   gate, which includes ignored local Reference books. Return actual counts,
   broken paths, metadata findings and limits; do not edit source-book metadata.
   Record actionable findings and evidence gaps without repairing them.
4. Return the scoped verdict, inspected paths/inputs, findings, actual evidence and
   limitations in text and JSON. Identify cross-lane issues without delegating them.

Stop after the assigned assessment. Do not select another lane, dispatch reviewers,
edit source/tests/documents, start a verification profile or treat findings as
repair authorization. Matching prior evidence can be retained under the common
contract; an incomplete assessment cannot approve.
