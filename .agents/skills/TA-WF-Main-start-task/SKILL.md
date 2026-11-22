---
name: TA-WF-Main-start-task
description: Initialize a selected task with live telemetry registration, a plan, test design and a provisioned task worktree; stop before implementation.
---

# Initialize the selected task

When the user selects task initialization, the main session runs the steps
below. It uses the requested outcome or existing plan, relevant requirements,
primary checkout, task or branch name and the actual identity of the session
or thread that initiated the task. The operation ends before implementation and
later delivery stages. Apply the [result contract](../../references/workflow-contract.md).

1. Inspect the current checkout, existing worktrees and supplied plan. Reuse a
   resumed task's location and identity to avoid creating a duplicate. Select a
   task ID from the plan or task name and use the observed session identity.
   Do not invent a thread ID. Register and confirm live telemetry through
   [TA-WF-Step-register-execution](../TA-WF-Step-register-execution/SKILL.md).
   If configuration or confirmation is missing, report exactly what is missing.
   Keep the gap visible without substituting a synthetic event or installing a
   service.
2. Prepare or amend the English plan and its test design through
   [TA-WF-Main-plan-change](../TA-WF-Main-plan-change/SKILL.md). That operation includes
   the test-design step; do not run it a second time for the same plan revision.
   Reuse supplied approved plans/scenarios where applicable. Resolve material
   behavior choices before dependent provisioning; do not infer approval for an
   undecided requirement.
   The main session assigns that combined planning operation to `TA-planner`
   under the [subagent policy](../../../AGENTS.md#subagent-assignment-and-model-selection).
   The planner prepares both the change plan and the test design. The main
   session handles telemetry registration and worktree provisioning.
3. Use [TA-WF-Step-start-plan](../TA-WF-Step-start-plan/SKILL.md) and the existing
   `tools/worktree.ps1` to provision the selected isolated worktree and copy the
   plan and needed inputs. The script handles copies of ignored assets and
   configuration. If provisioning fails, stop and preserve the partial result.
4. In text and JSON, return the authoritative working plan, the test-design
   location, the task worktree and branch, and the actual locations of telemetry
   registration and confirmation. Pass the existing task and execution context to
   the next operation so it can report its work in the same context.

Stop after initialization. Do not implement, commit, merge or select the next
stage. The user can invoke the component operations separately.
