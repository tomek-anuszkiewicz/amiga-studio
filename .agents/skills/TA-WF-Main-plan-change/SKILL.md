---
name: TA-WF-Main-plan-change
description: Create or amend a change plan and its test design together, with scope, acceptance criteria, risks and decisions; no implementation.
---

# Plan the selected change

Executor: `TA-planner`, assigned by the main session using the
[subagent policy](../../../AGENTS.md#subagent-assignment-and-model-selection).
This compound operation covers both creating and amending a plan. In the assigned
planner context, execute the procedure directly, including test design in the same
instance; do not delegate this operation again. The main session supplies current
requirements and decisions and returns unresolved choices to the user.

Apply the [result contract](../../references/workflow-contract.md), shared
[information hierarchy](../../references/information-hierarchy.md),
[specification compliance](../../references/spec-compliance.md) and genre-appropriate
[prose criteria](../../references/narrative-criteria.md). Internal API/schema changes
also use the [migration contract](../../references/internal-migrations.md).

Read the selected requirements and relevant architecture. Identify the starting
state, intended behavior, necessary changes, acceptance criteria and unresolved
choices. Preserve existing decisions and unfinished work when amending a plan.
Separate required work from optional ideas.

On every creation or amendment, apply
[TA-WF-Step-design-tests](../TA-WF-Step-design-tests/SKILL.md) to the resulting
plan within this operation. Reconcile scenarios, expected observations and existing
test protection with the current requirements. Preserve unaffected scenario IDs
and identify superseded expectations. If an editorial amendment leaves the test
design valid, retain it and report that assessment instead of duplicating it.
The user does not need to select the test-design step separately. This step
designs tests; implementation and execution remain outside planning.

Use a conversational draft unless saving a plan is requested. Save formal plans
in English at the authorized `.agent/tasks/` path. Keep the test design in the
plan or update its existing linked test-design document. For conversational drafts,
include the scenarios in the draft. Return the plan/test-design locations when
saved, summary, risks and decisions in text and JSON. Missing behavior or
authorization remains an explicit open decision. No hashed input record, round history, readiness
record or telemetry startup is required for planning.

Stop after the plan and its reconciled test design. Do not implement or create a
worktree without that scope.
