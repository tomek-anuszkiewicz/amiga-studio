---
name: TA-WF-Step-design-tests
description: Design meaningful scenarios and observations for agreed software behavior without writing tests or production code.
---

# Design the selected tests

Apply [the result contract](../../references/workflow-contract.md).
The main session assigns standalone requests to `TA-planner` under the
[subagent policy](../../../AGENTS.md#subagent-assignment-and-model-selection).
Within `TA-WF-Main-plan-change`, the assigned planner designs the scenarios and
expected observations as part of the same assignment. The implementer writes
the executable tests.
Start with the selected requirements or plan. Consider the affected interactions
and scenarios alongside existing test protection. Keep the design within the
authorized output. The user can include this operation in a combined planning
assignment.
Apply the plan/evidence ownership parts of the shared
[information hierarchy](../../references/information-hierarchy.md) to the assigned test design.
Use the shared [prose criteria](../../references/narrative-criteria.md) for clear
scenarios, observations and decisions without imposing an article structure.

1. Read the behavior contract and nearby tests. Apply the shared
   [testing criteria](../../references/testing-criteria.md), including test placement,
   parallel isolation, justified serial cases, domain fixture limits and independent
   expected results.
2. For each requirement and interaction, identify tests that can be reused and
   gaps those tests leave. Select a few meaningful scenarios. For each, explain
   why it matters, where to test it, what to observe and what fixtures it needs.
   Avoid per-function quotas, exhaustive catalogs and speculative fixture
   frameworks.
   Agree on the scenarios before the selected implementation begins. Ask the
   caller to decide on new scenarios outside the approved scope. A test-design
   assignment stops before implementation, and a code edit does not always
   require a separate test-design assignment.
3. For a saved plan, write the assigned test plan or owned section under
   `.agent/tasks/` and cite the exact input plan revision. For a conversational
   planning draft, return the scenarios with that draft without saving a file.
   When amending a plan, retain the IDs of unaffected scenarios and identify
   the expectations the amendment supersedes. Record scenario IDs, existing protection,
   proposed assertions, independent expectations and open decisions in the
   structured result. Report any newly identified
   [protected effect](../../references/protected-verification.md#protected-verification).

Stop after the test plan. Do not write code, execute implementation, measure
coverage or dispatch a critique. Mark blocked scenarios and return `blocked`
for missing behavior or approval that affects the assignment.
