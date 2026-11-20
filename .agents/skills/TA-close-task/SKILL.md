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
publication authorization, an existing sealed readiness record and the passing
[full English receipt](../../../tools/verification/README.md#scan-the-sealed-candidate) for that record. For ordinary
teardown, include the exact task and execution IDs from the existing capture
registration. For a selected delivery also supply its recorded selection
artifact, which binds the original plan inputs and authorized downstream scope.

1. Inspect lightweight validity: clean unchanged source, `B` included in its
   history, exact accepted tree/commit, committed closure content and retained
   evidence, including the full English receipt for `C`. The committed closure records delivered scope, decisions, actual
   verification and limits. Transfer remaining work to an active plan or
   roadmap unless the user explicitly waived it. Keep the original plan
   through committed closure and preserve the status of failed or unrun checks.
   A supplied confirmed-tested record is admissible only under its
   precise existing eligibility/approval rules; do not construct or select one.
2. Return `needs_input` for stale/missing prerequisites, with reasons such as
   `master_changed`, `candidate_changed`, `review_stale`, `verification_missing`
   or missing or stale English evidence. Do not repair them or collect new approval here.
3. Publish through
   `python tools/verification/verify.py integrate publish record <path> task <task-id> execution <execution-id> delivery <selection-path>`.
   Supply `delivery` for the selected delivery even if it has no push or
   cleanup. Omit it only when the operation has no recorded delivery selection.
   Preserve its reference locks and conditional exact `B -> C` transition.
   Do not substitute a target-side merge or a prior SHA comparison.
4. Return actual publication outcome and surviving control-record identity.
   After interruption, report uncertain state for separately selected recovery.
   On success, hand the surviving record and published SHA to the main session for
   the separate [post-publication stages](../../../tools/verification/README.md#post-publication-closure).
   After the project graph, the same record identifies ordinary
   [file preservation](../../../tools/workflow/capture/README.md#preserve-selected-files-at-task-closure).
   A successful publication is retained if a later closure stage is blocked.

Stop after publication/refusal. No synchronization, repairs, tests, reviews, new
commits, cleanup or push. Advanced multi-source assembly remains at its existing
rule/tool owner; this V1 public operation does not publish sources successively.
