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
3. Within Plan Change, put the scenarios, expected observations and acceptance
   criteria in the same plan file at the selected `.agent/tasks/` path, or in
   the same conversational draft when saving was not requested. For standalone
   test design, use the selected output and cite the exact input revision when
   it comes from a saved plan. When amending a plan, retain unaffected scenario
   IDs and identify superseded expectations. Record existing protection, scenario IDs,
   proposed assertions, independent expectations and open decisions in the
   structured result. Report any newly identified
   [protected effect](../../references/protected-verification.md#protected-verification).

Stop after the test plan. Do not write code, execute implementation, measure
coverage or dispatch a critique. Mark blocked scenarios and return `blocked`
for missing behavior or approval that affects the assignment.
