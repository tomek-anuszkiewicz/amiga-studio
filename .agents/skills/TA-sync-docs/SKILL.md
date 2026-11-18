---
name: TA-sync-docs
description: Reconcile assigned documentation and selected metadata or link repairs with approved requirements and implementation.
---

# Reconcile the selected documentation

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session; result kind: `documents`.
Inputs: assigned documents/findings, exact candidate, approved requirements,
implementation/evidence and write scope.

1. Read relevant [documentation policy](../../rules/docs-maintenance.md),
   [specification authority](../../rules/spec-compliance.md) and supplied findings.
   Select necessary owners using the shared
   [documentation criteria](../../references/documentation-criteria.md).
2. Reconcile only assigned descriptions and compatible enrichment. Preserve
   source-backed hardware explanations, independent requirements, provenance and
   unique rationale. Do not document a contradiction as accepted behavior.
3. Apply selected metadata/link repairs using the
   [vault schema](../TA-audit-vault-links/references/vault-metadata.md).
   Reference-book changes retain their concrete approval boundary. Accurate
   documents/dates remain unchanged; update metadata only with substantive edits.
4. Review the diff and relevant local links. Return changed documents, addressed
   IDs, actual checks, conflicts and missing evidence. Required final full vault
   checks remain explicit prerequisites when not already supplied.

Stop after document changes. Do not repair code/tests, perform an unrelated audit,
index documentation, begin integration, stage or commit.
