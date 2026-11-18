---
name: TA-review-behavior
description: Review behavior, state transitions and affected interactions in one exact candidate.
---

# Assess the behavior lane

Apply [the invocation contract](../../references/workflow-contract.md) and
[independent review contract](../../references/review-common.md).
Role: `TA-code-reviewer`; result kind: `review`; lane: `behavior`.
Inputs: exact baseline/candidate, assigned scope, relevant requirements, available
evidence and proposed partition when preparing a batch.

1. Check input identities and fresh reviewer context. Read only
   [this lane's criteria](references/behavior.md) plus relevant requirements.
2. Independently inspect the selected candidate and affected interactions. Record
   actionable findings and evidence gaps; do not repair them.
3. Return the scoped verdict, inspected paths/inputs, findings, actual evidence and
   limitations in text and JSON. Identify cross-lane issues without delegating them.

Stop after the assigned assessment. Do not select another lane, dispatch reviewers,
edit source/tests/documents, start a verification profile or treat findings as
repair authorization. Matching prior evidence can be retained under the common
contract; an incomplete assessment cannot approve.

