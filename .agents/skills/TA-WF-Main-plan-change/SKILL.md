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

Use a conversational draft with the scenarios unless the user asks to save
the plan. Save a plan and its test design together in one English file at the
authorized `.agent/tasks/` path. On amendment, incorporate applicable scenarios
from any older linked test-design document into the plan. Include expected
observations and acceptance criteria in either format.

The planner returns the summary, risks, decisions and any saved location to the
main session. List unresolved behavior and missing authorization among the open
decisions. The main session reports the result using the shared result contract.
Planning ends with the plan and reconciled scenarios. It requires no telemetry
registration, checkout provisioning or implementation.
