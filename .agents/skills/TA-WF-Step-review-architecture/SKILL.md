---
name: TA-WF-Step-review-architecture
description: Review architecture, authoritative definitions, visibility and complete internal migrations in one candidate.
---

# Assess the architecture lane

Apply [the result contract](../../references/workflow-contract.md) and
[independent review contract](../../references/review-common.md).
A `TA-code-reviewer` performs the `architecture` review. The main session
supplies the exact baseline and candidate, assigned scope, relevant requirements
and available evidence. For batch preparation, it also supplies the proposed
grouping of changes.

1. Confirm the exact inputs and that the reviewer has a fresh context. Read only
   [this lane's criteria](references/architecture.md) plus relevant requirements.
   For Rust changes, assess the shared [Rust best practices](../../references/rust-best-practices.md).
   For GUI changes, also assess [egui best practices](../../references/egui-best-practices.md),
   focusing on ownership, data preparation, allocation and host/core boundaries.
   When an API, schema or format is replaced, check that the migration covers
   all consumers and necessary documentation. Use the shared
   [internal migration contract](../../references/internal-migrations.md).
   Use the shared [prose criteria](../../references/narrative-criteria.md) when
   assessing necessary architecture documentation, within this lane's scope.
2. Independently inspect the selected candidate and affected interactions.
   Record actionable findings and gaps in the evidence. Leave repairs to a
   separate assignment.
3. In text and JSON, report the verdict for the assigned scope, the paths and
   inputs inspected, findings, actual evidence and limitations. Identify issues
   that affect another review lane without delegating them.

Stop after the assigned assessment. Do not select another lane, dispatch reviewers,
edit source/tests/documents, start a verification profile or treat findings as
repair authorization. Matching prior evidence can be retained under the common
contract; an incomplete assessment cannot approve.
