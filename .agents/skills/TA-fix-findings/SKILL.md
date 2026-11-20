---
name: TA-fix-findings
description: Repair explicitly selected finding IDs within an assigned candidate and verify the demonstrated failures.
---

# Repair the selected findings

Apply [the invocation contract](../../references/workflow-contract.md).
Role: `TA-implementer`; result kind: `implementation`.
Inputs: selected stable finding IDs, their evidence/requirements, current candidate,
write scope, approved scenarios and focused checks. No selected IDs means
`needs_input`, not a general cleanup pass.

1. Read each selected finding and its cited criterion. Follow
   [defect resolution](../../references/defect-resolution.md) before mutation.
   Confirm the failure and mechanism; preserve unsupported causes as hypotheses.
2. Repair only the selected scope, including necessary consumers and agreed tests.
   For confirmed unused-code findings, use the
   [pruning procedure](references/pruning.md). A scanner candidate alone is not
   proof that code or its tests can be deleted.
3. Run the original reproduction and relevant focused regressions. Count each
   repair/check pass; do not hide additional rounds in retries or delegation.
4. Preserve IDs and report `repaired_pending_verification` until evidence
   verifies the expected behavior. Keep unselected, deferred and unresolved
   findings visible with their existing disposition.
   When this invocation completes the current implementation for delivery,
   attach the [completed source receipt](../../references/workflow-guidance.md#record-the-source-at-implementation-completion)
   before returning the result pair.

Stop after these repairs. Findings do not authorize unrelated changes, new test
scenarios, weakened checks, broad verification or automatic reviewer dispatch.
