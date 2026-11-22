---
name: TA-WF-Step-challenge-findings
description: Independently assess selected review findings and explain whether each is supported, without repairs.
---

# Challenge the selected findings

Apply the [result contract](../../references/workflow-contract.md) and
[independent review criteria](../../references/review-common.md).
Inputs: selected findings, source locations, cited criteria and relevant evidence.

Use a fresh context. For each finding, identify what would trigger the issue,
which behavior it affects and which requirement it cites. Determine whether
the reported consequence can occur under conditions the project supports.
If a probe forces a state, its result establishes only what happened under
that imposed condition. Preserve supplied
finding IDs. Explain why you uphold or reject each claim and identify any
missing evidence. Record the decisions in the findings and summary of the
common JSON result and in the accompanying explanation.

Stop after assessment. Do not repair, dispatch a reviewer, reserve another round,
change acceptance criteria or infer commit authority.
