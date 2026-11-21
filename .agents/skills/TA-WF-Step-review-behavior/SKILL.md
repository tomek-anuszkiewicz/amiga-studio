---
name: TA-WF-Step-review-behavior
description: Review behavior, specification compliance and affected interactions in a scoped change or a selected whole-stage semantic assessment.
---

# Assess the behavior lane

Apply [the result contract](../../references/workflow-contract.md) and
[independent review contract](../../references/review-common.md).
Role: `TA-code-reviewer`; lane: `behavior`.
Inputs: exact baseline/candidate, assigned scope, relevant requirements, available
evidence and proposed partition when preparing a batch.

1. Check input identities and fresh reviewer context. Read only
   [this lane's criteria](references/behavior.md) plus relevant requirements.
   For Rust changes, assess execution semantics against the shared
   [Rust best practices](../../references/rust-best-practices.md). For GUI changes,
   apply [egui best practices](../../references/egui-best-practices.md) and
   [GUI criteria](../../references/gui-criteria.md) to interactions, retained
   inspection state, responsiveness, framebuffer scaling and visual evidence.
   Apply the shared [specification compliance guidance](../../references/spec-compliance.md)
   to requirement conflicts and approved behavior changes.
2. Independently inspect the selected candidate and affected interactions. Record
   actionable findings and evidence gaps; do not repair them.
   Select affected specifications through Design `tracked_paths`, explicit
   requirement changes and the exact path-boundary rules in the shared
   [documentation criteria](../../references/documentation-criteria.md#select-the-complete-intended-scope).
   Include both rename/delete endpoints and selected new files; unrelated working
   edits cannot satisfy an index candidate. Trace changed requirements into
   affected unchanged callers/tests. Compare approved behavior and tests against
   requirements and source evidence; code agreement does not waive a requirement.
   Report unmet requirements, unsupported claims and specification conflicts.
   For an assigned minor-roadmap/major-milestone stage or whole-subsystem parity
   assessment, establish the complete stage and interactions, including unchanged
   code. Missing stage scope is an input gap, not permission to choose a chip.
   Run the existing workspace-wide static comparison without narrowing it:
   `python tools/verification/checks/audit_semantic_sync.py --semantic-sync`.
   Retain matching current evidence when supplied. A script pass alone is not
   semantic proof. Assess stage-wide behavior, timing, ownership and interactions
   against approved specifications; identify documentation/provenance gaps for
   the documentation lane and record overlap with a scoped compliance assessment.
3. Return the scoped verdict, inspected paths/inputs, findings, actual evidence and
   limitations in text and JSON. Identify cross-lane issues without delegating them.

Stop after the assigned assessment. Do not select another lane, dispatch reviewers,
edit source/tests/documents, start a verification profile or treat findings as
repair authorization. Matching prior evidence can be retained under the common
contract; an incomplete assessment cannot approve.
