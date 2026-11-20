---
name: TA-implement-change
description: Implement one approved change and its agreed tests with bounded focused verification.
---

# Implement the approved scope

Apply [the invocation contract](../../references/workflow-contract.md).
Role: `TA-implementer`; result kind: `implementation`.
Inputs: approved change/test plans or explicitly assigned equivalent requirements,
candidate, write scope, relevant specifications and focused checks.

1. Read relevant advance requirements, nearby implementations and agreed test
   scenarios. Applicable review standards remain binding without loading all lanes.
   Check the linked design against the active plan, current code and existing
   tests and reuse the design where it still applies. Keep planned observations
   distinct from executed results. Recheck any
   [individual approval](../../references/workflow-guidance.md#record-individual-approvals)
   at the point of edit.
   For a defect, load [defect resolution](../../references/defect-resolution.md)
   and establish the original failure before changing behavior.
2. Implement the assigned behavior and agreed tests, migrating affected internal
   callers completely. Preserve unrelated source/index versions and protected
   references. Technical fixture/assertion choices within agreed scenarios belong
   to this implementation; a new scenario outside them needs caller selection.
3. Run the assigned focused checks and review the diff against authorized scope.
   Each implementation/correction pass plus its checks is one round. Announce the
   reason before a further pass and retain successful still-applicable evidence.
4. When this invocation completes a selected plan for delivery, capture the
   [completed source](../../references/workflow-guidance.md#record-the-source-at-implementation-completion)
   and bind its receipt before returning the result pair. Return changes,
   addressed scenario/finding IDs, commands, results and remaining gaps.
   Distinguish implementation from verification and batch readiness.

Stop at the assigned result or contract boundary. Do not begin coverage measurement,
broad review, documentation synchronization, staging, commits or another skill.
Required unresolved gaps remain blockers; report them for an explicit next step.
