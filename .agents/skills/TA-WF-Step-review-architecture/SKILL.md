---
name: TA-WF-Step-review-architecture
description: Review architecture, authoritative definitions, visibility and complete internal migrations in one candidate.
---

# Assess the architecture lane

Apply [the result contract](../../references/workflow-contract.md) and
[independent review contract](../../references/review-common.md).
Role: `TA-code-reviewer`; lane: `architecture`.
Inputs: exact baseline/candidate, assigned scope, relevant requirements, available
evidence and proposed partition when preparing a batch.

1. Check input identities and fresh reviewer context. Read only
   [this lane's criteria](references/architecture.md) plus relevant requirements.
   For Rust changes, assess the shared [Rust best practices](../../references/rust-best-practices.md).
   For GUI changes, also assess [egui best practices](../../references/egui-best-practices.md),
   focusing on ownership, data preparation, allocation and host/core boundaries.
   For replaced APIs, schemas or formats, assess complete consumers and necessary
   documentation against the shared [internal migration contract](../../references/internal-migrations.md).
   Use the shared [prose criteria](../../references/narrative-criteria.md) when
   assessing necessary architecture documentation, within this lane's scope.
2. Independently inspect the selected candidate and affected interactions. Record
   actionable findings and evidence gaps; do not repair them.
3. Return the scoped verdict, inspected paths/inputs, findings, actual evidence and
   limitations in text and JSON. Identify cross-lane issues without delegating them.

Stop after the assigned assessment. Do not select another lane, dispatch reviewers,
edit source/tests/documents, start a verification profile or treat findings as
repair authorization. Matching prior evidence can be retained under the common
contract; an incomplete assessment cannot approve.
