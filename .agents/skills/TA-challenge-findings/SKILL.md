---
name: TA-challenge-findings
description: Decide whether to uphold or reject each listed review finding on the same candidate in one independent pass.
---

# Challenge selected review findings

Apply the [invocation contract](../../references/workflow-contract.md) and
[receiving procedure](../../references/workflow-guidance.md#challenge-required-findings).
Executor: a fresh `TA-code-reviewer`. Result kind: `finding_challenge`. Limit: one pass.

Read the frozen candidate, round manifest, hashed original reports, selected IDs,
requirements, evidence and attributable decisions. A linked assessment supplies
the predecessor collection and report and identifies any newly selected claims.
Work in fresh context without author history.

For each supplied lane and ID, test the strongest counterexample. Trace how
supported use or a binding requirement covers the preconditions, check the probe
setup and recovery, and establish the consequence. For a workflow defect, apply
the [supported-sequence criterion](../../references/review-common.md#workflow-defects)
to the actual setup as well as the candidate. Reject the repair request if the
decisive transition only occurs in the setup. Uphold it when the supported path
and claimed failure are both established. Account for the scope of any recorded
risk decision. A rare trigger or an unsuccessful reproduction can coexist with
a conclusive source trace.

Return one final disposition per lane and ID. Use `uphold` when the evidence
supports the submitted repair and `reject` when it does not. Cite the evidence and
explain the decision. Link common causes while retaining all original IDs, lanes
and verification obligations. Preserve the primary reports and independent gate
evidence. Write only your result pair and stop after the listed claims.
