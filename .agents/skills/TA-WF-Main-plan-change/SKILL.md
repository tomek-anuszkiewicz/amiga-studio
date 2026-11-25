---
name: TA-WF-Main-plan-change
description: Prepare or update a change plan in .agent/tasks/ with its test design, scope, acceptance criteria, risks and decisions.
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
[prose criteria](../../references/narrative-criteria.md).

When planning changes to internal APIs, schemas, configuration or generated
formats, apply the [migration contract](../../references/internal-migrations.md)
before defining scope. Plan for the new version. Include backward compatibility,
migration of existing data or upgrade instructions only when the user explicitly
requests that work or a documented external contract requires it. Record the
effect on existing user data and preserve approved decisions about handling it.

Read the selected requirements and relevant architecture. Before defining acceptance
criteria and tests, search the [review decisions register](../../references/review-decisions.md)
by subject and owning requirement. Check applicable entries against the current
requirements and plan inputs. Identify the starting state, intended behavior,
necessary changes, acceptance criteria and unresolved choices. Preserve existing
decisions and unfinished work when amending a plan. List optional ideas as
separate proposals. Approval of the plan leaves them pending until the user
explicitly selects them under the
[scope approval rule](../../../AGENTS.md#approval-for-scope-changes).

On every creation or amendment, apply
[TA-WF-Step-design-tests](../TA-WF-Step-design-tests/SKILL.md) to the resulting
plan within this operation. Reconcile scenarios, expected observations and existing
test protection with the current requirements. Preserve unaffected scenario IDs
and identify superseded expectations. If an editorial amendment leaves the test
design valid, retain it and report that assessment instead of duplicating it.
The planning operation includes designing the tests. Writing and running the
tests are later steps.

A request to prepare a plan authorizes saving it. Use the requirements and
decisions from the discussion. Write each requested plan and its test design
together in one English Markdown file under `.agent/tasks/`. When updating an
existing plan, edit its file and incorporate applicable scenarios from any
older linked test-design document. If the user explicitly asks for a chat-only
draft, present the plan and scenarios in the conversation.

The planner returns the summary, risks, decisions and path of each saved plan
to the main session. List unresolved behavior and missing authorization among
the open decisions. The main session links each saved plan in its reply.
Planning ends with the plan and reconciled scenarios. It requires no telemetry
registration, checkout provisioning or implementation.
