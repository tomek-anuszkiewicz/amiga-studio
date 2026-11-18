---
name: TA-design-tests
description: Design meaningful scenarios and observations for one approved software change without writing tests or production code.
---

# Design the selected tests

Apply [the invocation contract](../../references/workflow-contract.md).
Role: `TA-test-designer`; result kind: `test_design`.
Inputs: requirements/change plan, exact starting candidate, interactions at risk,
existing evidence, authorized test-plan path and round budget.

1. Read the behavior contract and nearby tests. Apply the relevant
   [testing criteria](../../references/testing-criteria.md) and
   [testing policy](../../rules/unit-testing-policy.md), including domain fixture
   limits and independent expected results.
2. Map requirements and interactions to reusable tests and concrete gaps. Select
   a few meaningful scenarios, suitable levels, expected observations and fixture
   needs. Explain why each scenario matters; do not create per-function quotas,
   exhaustive catalogs or speculative fixture frameworks.
3. Write the assigned test plan under `.agent/tasks/`. Record scenario IDs,
   existing protection, proposed assertions, independent expectations and open
   decisions in the structured result.

Stop after the test plan. Do not write code, execute implementation, measure
coverage or dispatch a critique. Missing behavior/approval returns `needs_input`.

