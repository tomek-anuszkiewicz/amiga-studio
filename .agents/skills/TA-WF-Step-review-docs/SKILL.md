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
Assign the `documentation` review to `TA-code-reviewer`. Supply the exact
baseline and candidate, assigned scope, relevant requirements and available
evidence. When preparing a batch, also supply the proposed grouping of changes.

For major milestone sign-off, assess the stage-level scope required by the
[milestone completion criteria](../../../Obsidian/Amiga/Design/Verification/Testing%20Strategy%20and%20Quality%20Assurance.md#roadmap-and-milestone-completion).
Reuse matching prior assessment for the work it already covers.

1. Check input identities and fresh reviewer context. Read only
   [this lane's criteria](references/documentation.md) plus relevant requirements.
2. Independently inspect the selected candidate and affected interactions.
   Use the shared exact path-boundary rules to identify affected requirements
   from Design `tracked_paths` and explicit requirement changes. Include
   selected new files and both rename/delete endpoints. Compare documentation
   against approved requirements, implementation/tests and source-confirmed
   hardware knowledge in both directions. Distinguish compatible additions,
   unsupported claims, unmet requirements and genuine conflicts. Preserve unique
   reasoning and provenance. A selected stage-wide assessment covers the full
   stage and its interactions, including unchanged affected documents and code.
   Record where existing evidence covers the same work.
3. For assigned Design documentation, run the full recursive link/metadata audit:
   `python .agents/skills/TA-WF-Step-review-docs/scripts/audit_vault.py --vault-links --frontmatter`.
   Read the [vault schema and semantic criteria](../../references/vault-metadata.md)
   and assess contextual links and metadata meaning, including empty tag/link
   sets. Reuse overlapping current canonical structural evidence only where it
   applies. The wider canonical frontmatter gate still applies and includes
   ignored local Reference books. Return actual counts, broken paths, metadata
   findings and limits. Leave source-book metadata edits and other repairs to
   separate assignments. Record actionable findings and gaps in the evidence.
4. Return the scoped verdict, inspected paths/inputs, findings, actual evidence and
   limitations in text and JSON. Identify cross-lane issues without delegating them.

Stop after the assigned assessment. Do not select another lane, dispatch reviewers,
edit source/tests/documents, start a verification profile or treat findings as
repair authorization. Matching prior evidence can be retained under the common
contract; an incomplete assessment cannot approve.
