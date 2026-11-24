---
name: TA-WF-Step-challenge-findings
description: Independently assess selected review findings and explain whether each is supported, without repairs.
---

# Challenge the selected findings

Apply the [result contract](../../references/workflow-contract.md).
Inputs: selected findings, source locations, cited criteria and relevant evidence.

In a fresh context, check each claim against the source and the shared
[criteria for reporting findings](../../references/review-common.md#which-findings-to-report).
Uphold it when the evidence establishes the claimed defect, gap in a check or
unmet requirement. Otherwise reject it and identify which part of the claim
the evidence fails to establish. Preserve the original finding ID, actual
check outcomes and any missing required evidence. Record each decision and
its reason in the common JSON result and accompanying explanation.

Stop after assessment. Do not repair, dispatch a reviewer, reserve another round,
change acceptance criteria or infer commit authority.
