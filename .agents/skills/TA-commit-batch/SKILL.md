---
name: TA-commit-batch
description: Freeze a fully prepared and accepted batch, create only its authorized mechanical commits and seal integration readiness where applicable.
---

# Commit the prepared candidate

Apply [the invocation contract](../../references/workflow-contract.md),
[batch policy](../../rules/git-commits.md) and applicable
[integration protocol](../../../tools/verification/README.md#local-integration).
Executor: main session; result kind: `commits`.
Inputs: commit authority, exact candidate/partition/messages, prepared diary/closure
content, complete current canonical evidence and four applicable independent approvals.

1. Validate prerequisites and current candidate/index/HEAD identities. Missing,
   failed or stale evidence returns its blocker; do not obtain it here. Retain
   accepted scoped review comparisons and verifier evidence rules.
2. Freeze candidate, partition, messages and evidence together. For integration,
   use the existing `integrate freeze review <path> partition <path> target <branch>`
   protocol with the already prepared immutable source list where applicable.
3. Create only the prepared mechanical commits. Use exact trees, parents and
   messages; preserve unrelated index entries. Make no content edits between
   commits. On interference or partial failure, stop and report completed commits
   and pending work without reset or rewriting history.
4. Verify the resulting final tree and sequence. For integration, seal the frozen
   record with `integrate seal record <path>`; no expensive rerun is needed solely
   because those exact mechanical commits changed HEAD.
5. Return commit IDs, final identity, evidence and seal outcome. Acceptance covers
   the combined candidate, not independently tested intermediate commits.

Stop after commits/sealing. Do not repair prerequisites, edit content, publish,
push or clean up. Confirmed-tested shortcut records are prepared by the caller
under their existing policy, not selected as a fallback by this skill.

