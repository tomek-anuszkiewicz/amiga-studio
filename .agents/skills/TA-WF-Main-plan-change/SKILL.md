---
name: TA-WF-Main-plan-change
description: Create or amend a change plan and its test design together, with scope, acceptance criteria, risks and decisions; no implementation.
---

# Plan the selected change

The main session assigns `TA-planner` to create or amend the plan. It supplies the
current requirements and decisions and brings unresolved choices back to the
user. The assigned planner carries out the procedure directly and designs tests
in the same context, without delegating the operation again. See the
[subagent policy](../../../AGENTS.md#subagent-assignment-and-model-selection).

Apply the [result contract](../../references/workflow-contract.md), shared
[information hierarchy](../../references/information-hierarchy.md),
[specification compliance](../../references/spec-compliance.md) and genre-appropriate
[prose criteria](../../references/narrative-criteria.md). Internal API/schema changes
also use the [migration contract](../../references/internal-migrations.md).

Read the selected requirements and relevant architecture. Before defining acceptance
criteria and tests, search the [review decisions register](../../references/review-decisions.md)
by subject and owning requirement. Check applicable entries against the current
requirements and plan inputs. Identify the starting state, intended behavior,
necessary changes, acceptance criteria and unresolved choices. Preserve existing
decisions and unfinished work when amending a plan. Separate required work from
optional ideas.

On every creation or amendment, apply
[TA-WF-Step-design-tests](../TA-WF-Step-design-tests/SKILL.md) to the resulting
plan within this operation. Reconcile scenarios, expected observations and existing
test protection with the current requirements. Preserve unaffected scenario IDs
and identify superseded expectations. If an editorial amendment leaves the test
design valid, retain it and report that assessment instead of duplicating it.
The planning operation includes designing the tests. Writing and running the
tests are later steps.

Use a conversational draft unless saving a plan is requested. Save formal plans
in English at the authorized `.agent/tasks/` path. Keep the test design in the
plan or update its existing linked test-design document. For conversational drafts,
include the scenarios in the draft. If you save the plan, report where it and the test design are stored. Return
the summary, risks, decisions and any saved locations in text and JSON. List
unresolved questions about behavior and missing authorization among the open
decisions. Planning requires no hashed input record, round history,
readiness record or telemetry startup.

Stop after the plan and its reconciled test design. Do not implement or create a
worktree without that scope.
