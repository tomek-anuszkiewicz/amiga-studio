---
name: TA-close-task
description: Conditionally publish one already sealed and accepted task candidate to local master without preparing or repairing it.
---

# Publish the supplied ready task

Apply [the invocation contract](../../references/workflow-contract.md),
[publication policy](../../rules/git-merge-commits.md) and the
[existing tool protocol](../../../tools/verification/README.md#local-integration).
Executor: main session; result kind: `publication`.
Inputs: one selected task/source, target, current immutable `B`, final `C`,
publication authorization and an existing sealed readiness record.

1. Inspect lightweight validity: clean unchanged source, `B` included in its
   history, exact accepted tree/commit, committed closure content and retained
   evidence. A supplied confirmed-tested record is admissible only under its
   precise existing eligibility/approval rules; do not construct or select one.
2. Return `needs_input` for stale/missing prerequisites, with reasons such as
   `master_changed`, `candidate_changed`, `review_stale` or
   `verification_missing`. Do not repair them or collect new approval here.
3. Publish through
   `python tools/verification/verify.py integrate publish record <path>`.
   Preserve its reference locks and conditional exact `B -> C` transition.
   Do not substitute a target-side merge or a prior SHA comparison.
4. Return actual publication outcome and surviving control-record identity.
   After interruption, report uncertain state for separately selected recovery.

Stop after publication/refusal. No synchronization, repairs, tests, reviews, new
commits, cleanup or push. Advanced multi-source assembly remains at its existing
rule/tool owner; this V1 public operation does not publish sources successively.

