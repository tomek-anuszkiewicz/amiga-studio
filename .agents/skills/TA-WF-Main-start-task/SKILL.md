---
name: TA-WF-Main-start-task
description: Initialize a selected task with live telemetry registration, a plan, test design and a provisioned task worktree; stop before implementation.
---

# Initialize the selected task

Apply the [result contract](../../references/workflow-contract.md). This is a
main-session compound operation selected by the user. Its assignment covers the
steps below, not implementation or a later delivery stage.
Inputs: requested outcome or existing plan, relevant requirements, primary
checkout, task/branch name and the actual initiating session/thread identity.

1. Inspect the current checkout, existing worktrees and supplied plan. Reuse a
   resumed task's location and task identity; do not create a duplicate. Select a
   task ID from the plan/name and use the observed session identity, never a made-up
   thread ID. Register and confirm live telemetry through
   [TA-WF-Step-register-execution](../TA-WF-Step-register-execution/SKILL.md). Missing configuration
   or confirmation returns its actual gap; do not silently replace live evidence
   with a synthetic event or install a service.
2. Prepare or amend the English plan and its test design through
   [TA-WF-Main-plan-change](../TA-WF-Main-plan-change/SKILL.md). That operation includes
   the test-design step; do not run it a second time for the same plan revision.
   Reuse supplied approved plans/scenarios where applicable. Resolve material
   behavior choices before dependent provisioning; do not infer approval for an
   undecided requirement.
   The main session assigns that combined planning operation to `TA-planner`
   under the [subagent policy](../../../AGENTS.md#subagent-assignment-and-model-selection).
   One planner owns both the change plan and test design; the main session retains
   telemetry registration and worktree provisioning.
3. Use [TA-WF-Step-start-plan](../TA-WF-Step-start-plan/SKILL.md) and the existing
   `tools/worktree.ps1` to provision the selected isolated worktree and copy the
   plan and needed inputs. The script owns ignored asset/configuration copying.
   Preserve any partial provisioning and stop on failure rather than replacing it.
4. Return the authoritative working plan, test-design location, task worktree,
   branch and actual telemetry registration/confirmation locations in text and
   JSON. Give the next operation the existing task/execution context for reports.

Stop after initialization. Do not implement, commit, merge or select the next
stage. The user can invoke the component operations separately.
