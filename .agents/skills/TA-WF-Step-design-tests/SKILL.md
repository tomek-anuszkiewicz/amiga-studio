---
name: TA-WF-Step-design-tests
description: Design meaningful scenarios and observations for agreed software behavior without writing tests or production code.
---

# Design the selected tests

Apply [the result contract](../../references/workflow-contract.md).
Executor: `TA-planner`. A standalone request is assigned by the main session under
the [subagent policy](../../../AGENTS.md#subagent-assignment-and-model-selection).
Within `TA-WF-Main-plan-change`, the existing planner performs this step directly
in the same assignment. Design scenarios and observations; the implementer writes
the executable tests.
Inputs: selected requirements or plan, affected interactions and scenarios,
existing protection and authorized test-design output. The user can include this
operation in a combined planning assignment.
Apply the plan/evidence ownership parts of the shared
[information hierarchy](../../references/information-hierarchy.md) to the assigned test design.
Use the shared [prose criteria](../../references/narrative-criteria.md) for clear
scenarios, observations and decisions without imposing an article structure.

1. Read the behavior contract and nearby tests. Apply the shared
   [testing criteria](../../references/testing-criteria.md), including test placement,
   parallel isolation, justified serial cases, domain fixture limits and independent
   expected results.
2. Map requirements and interactions to reusable tests and concrete gaps. Select
   a few meaningful scenarios, suitable levels, expected observations and fixture
   needs. Explain why each scenario matters; do not create per-function quotas,
   exhaustive catalogs or speculative fixture frameworks.
   Establish agreed scenarios before the selected implementation. Report new
   scenarios outside the approved scope for a caller decision; do not start
   implementation or require this separate design operation for every code edit.
3. For a saved plan, write the assigned test plan or owned section under
   `.agent/tasks/` and cite the exact input plan revision. For a conversational
   planning draft, return the scenarios with that draft without saving a file.
   On amendment retain unaffected scenario IDs
   and identify superseded expectations. Record scenario IDs, existing protection,
   proposed assertions, independent expectations and open decisions in the
   structured result. Report any newly identified
   [protected effect](../../references/protected-verification.md#protected-verification).

Stop after the test plan. Do not write code, execute implementation, measure
coverage or dispatch a critique. Mark blocked scenarios and return `blocked`
for missing behavior or approval that affects the assignment.
