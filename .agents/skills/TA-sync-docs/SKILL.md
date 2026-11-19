---
name: TA-sync-docs
description: Reconcile assigned documentation and selected metadata or link repairs with approved requirements and implementation.
---

# Reconcile the selected documentation

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session; result kind: `documents`.
Inputs: assigned documents/findings, exact candidate, approved requirements,
implementation/evidence and write scope.

1. Read [Documentation Value](../../references/documentation-criteria.md#documentation-value),
   [specification authority](../../rules/spec-compliance.md) and supplied findings.
   Reader-facing docs explain the project and its entry points. Keep operating
   procedures with their tools or skills and update their consumers when they move.
2. Reconcile only assigned descriptions and compatible enrichment. Preserve
   source-backed hardware explanations, independent requirements, provenance and
   unique rationale. Do not document a contradiction as accepted behavior.
3. Apply selected metadata/link repairs using the
   [vault schema](../TA-audit-vault-links/references/vault-metadata.md).
   Follow the separate [reference book schema](../../../tools/bootstrap/book-to-markdown/reference-book-metadata.md)
   and its approval requirement before editing source notes. Preserve approved
   intent, neighboring structure and links to implementation. Update dates and
   relevant status, tags or dependencies for substantive changes. Keep gaps as
   brief factual caveats linked to the owning [roadmap](../../../ROADMAP.md).
   Reuse shared specification owners and update General Architecture dependencies
   when crate relationships change.
4. Review the diff and relevant local links. Return changed documents, addressed
   IDs, actual checks, conflicts and missing evidence. Required final full vault
   checks remain explicit prerequisites when not already supplied.

Stop after document changes. Do not repair code/tests, perform an unrelated audit,
index documentation, begin integration, stage or commit.
