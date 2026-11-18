---
name: TA-audit-vault-links
description: Audit the full applicable Design link and metadata set with existing tooling and report findings without repairs.
---

# Audit vault links and metadata

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: documentation reviewer or main session; result kind: `review`;
lane: `vault_links`.
Inputs: final document set or explicit audit scope, exact candidate and supplied
canonical frontmatter evidence.

1. Read the [vault schema and semantic criteria](references/vault-metadata.md).
   Inspect applicable metadata meaning and contextual/reference relationships.
2. Run the full recursive Design audit:
   `python .agents/skills/TA-audit-vault-links/scripts/audit_vault.py --vault-links --frontmatter`.
   Reuse overlapping canonical structural evidence only where it actually applies.
   Empty Design tags, links and semantic relationships still require this coverage.
3. Return audited scope, actual counts/commands/results, broken paths, metadata
   findings and evidence limits. The canonical frontmatter gate includes ignored
   local Reference books; do not claim this Design audit replaces that wider scope.

Stop after reporting. Do not edit notes or reference-book metadata. Selected repairs
belong to a writer invocation with required concrete approvals already supplied.

