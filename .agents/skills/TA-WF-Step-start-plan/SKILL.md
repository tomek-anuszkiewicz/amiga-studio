---
name: TA-WF-Step-start-plan
description: Start a selected plan in its authorized task worktree using the existing provisioning script.
---

# Start the selected plan

Apply the [result contract](../../references/workflow-contract.md).
Inputs: selected plan, source checkout, target branch/base and provisioning scope.

Inspect existing branches, worktrees and pending work. Resume an existing plan in
its recorded checkout. For a new selected task worktree use the
[provisioning command](references/provisioning.md#create-a-task-worktree) directly.
The script owns local asset copying. Report a missing input or branch/path
collision rather than changing the destination silently.

Copy the selected plan and necessary linked inputs to their relative locations
when requested. Keep the task identity and identify the authoritative working
plan. Return checkout, branch, plan path, actual commands and partial failures.
No source fingerprints, completion receipts or delivery declarations are required.

Stop after task start. Do not implement, stage or commit unless included in the
same assignment. Telemetry registration remains a separate selected operation.
