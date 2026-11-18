---
name: TA-review-spec-compliance
description: Assess one complete intended candidate against approved specifications and valuable-knowledge requirements without editing it.
---

# Review scoped specification compliance

Apply [the invocation contract](../../references/workflow-contract.md),
[independent review contract](../../references/review-common.md) and
[documentation criteria](../../references/documentation-criteria.md).
Role: `TA-code-reviewer`; result kind: `review`; lane: `spec_compliance`.
Inputs: exact baseline/candidate, selected paths/new files, proposed partition,
requirement/approval evidence and relevant verification.

1. Establish the complete intended snapshot. Include both rename/delete endpoints,
   explicitly selected untracked files and integration changes relative to recorded
   `B`. Do not use unrelated worktree edits as evidence for an index candidate.
2. Select affected specifications through `tracked_paths`, explicitly changed
   requirements and direct links, using the shared criteria's exact path-boundary
   rules. Report unmapped scope and inspect changed requirements' unchanged callers.
3. Compare approved requirements, behavior/tests and valuable knowledge in both
   directions. Inspect previous wording and authorization for changed contracts.
   Classify compatible enrichment, unmet requirements, unsupported claims and
   conflicts without silently accepting divergence.
4. Return reviewed snapshot/requirements, findings and evidence gaps. A matching
   broader semantic review can satisfy this scope; record the overlap rather than
   repeating it.

Stop after assessment. No document/code/test repairs, staging, verification
dispatch or automatic next step. Unresolved conflicts, valuable knowledge gaps and
unverified changed requirements block readiness even when static checks pass.
