---
name: TA-design-tests
description: Design meaningful scenarios and observations for agreed software behavior without writing tests or production code.
---

# Design the selected tests

Apply [the invocation contract](../../references/workflow-contract.md).
Role: `TA-test-designer`; result kind: `test_design`.
Inputs: saved plan revision and digest, exact starting candidate, affected
interactions and scenarios, existing protection, owned design output and round
budget. The main session may select this operation as part of
[saving a formal plan](../../references/workflow-guidance.md#save-a-formal-plan).

1. Read the behavior contract and nearby tests. Apply the relevant
   [testing criteria](../../references/testing-criteria.md) and
   [testing policy](../../rules/unit-testing-policy.md), including domain fixture
   limits and independent expected results.
2. Map requirements and interactions to reusable tests and concrete gaps. Select
   a few meaningful scenarios, suitable levels, expected observations and fixture
   needs. Explain why each scenario matters; do not create per-function quotas,
   exhaustive catalogs or speculative fixture frameworks.
3. Write the assigned test plan or owned section under `.agent/tasks/` and cite
   the exact input plan revision. On amendment retain unaffected scenario IDs
   and identify superseded expectations. Record scenario IDs, existing protection,
   proposed assertions, independent expectations and open decisions in the
   structured result. Report any newly identified
   [protected effect](../../references/workflow-guidance.md#record-individual-approvals).

Stop after the test plan. Do not write code, execute implementation, measure
coverage or dispatch a critique. Mark blocked scenarios and return `needs_input`
for missing behavior or approval that affects the assignment.
