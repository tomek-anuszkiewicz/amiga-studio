---
name: TA-WF-Step-challenge-findings
description: Independently assess selected review findings and explain whether each is supported, without repairs.
---

# Challenge the selected findings

Apply the [result contract](../../references/workflow-contract.md) and
[independent review criteria](../../references/review-common.md).
Inputs: selected findings, source locations, cited criteria and relevant evidence.

Use fresh context. Inspect the actual trigger, affected behavior and cited
requirement for each finding. Establish whether the reported consequence is
reachable under supported conditions; a forced state alone proves only that case.
Preserve supplied finding IDs. Explain which claims you uphold or reject and why,
and identify any missing evidence. Return the decisions in the findings and summary
of the common JSON result and in the textual explanation.

Stop after assessment. Do not repair, dispatch a reviewer, reserve another round,
change acceptance criteria or infer commit authority.
