---
name: TA-WF-Step-review-behavior
description: Review behavior, specification compliance and affected interactions in a scoped change or a selected whole-stage semantic assessment.
---

# Assess the behavior lane

Apply [the result contract](../../references/workflow-contract.md) and
[independent review contract](../../references/review-common.md).
The main session assigns the `behavior` review to `TA-code-reviewer` and supplies
the exact baseline and candidate, assigned scope, relevant requirements and
available evidence. When preparing a batch, it also supplies the proposed
grouping of changes.

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
   Include both rename/delete endpoints and selected new files. When reviewing
   a candidate from the index, use the selected staged contents as evidence.
   Results that depend on unrelated working files leave the staged candidate
   unverified.
   Trace changed requirements into all affected callers and tests, including
   unchanged ones. Compare approved behavior and tests against requirements
   and source evidence. Approved requirements remain in force regardless of
   how the current code behaves.
   Report unmet requirements, unsupported claims and specification conflicts.
   For an assigned minor-roadmap stage, major-milestone stage or parity
   assessment of an entire subsystem, establish the full scope and its
   interactions, including unchanged code. If stage scope is missing, report
   the missing input and leave the choice of chip open.
   Run the existing workspace-wide static comparison without narrowing it:
   `python tools/verification/checks/audit_semantic_sync.py --semantic-sync`.
   Retain supplied evidence when it matches the current candidate. Also assess
   behavior, timing, ownership and interactions across the stage against the
   approved specifications. This assessment is needed even if the script
   passes. Identify documentation and provenance gaps for the documentation
   lane and record overlap with any scoped compliance assessment.
3. In text and JSON, report the verdict for the assigned scope, the paths and
   inputs inspected, findings, actual evidence and limitations. Identify issues
   that affect other review lanes without delegating them.

Stop after the assigned assessment. Do not select another lane, dispatch reviewers,
edit source/tests/documents, start a verification profile or treat findings as
repair authorization. Matching prior evidence can be retained under the common
contract; an incomplete assessment cannot approve.
