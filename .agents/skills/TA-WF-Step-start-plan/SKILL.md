---
name: TA-WF-Step-start-plan
description: Start a selected plan in its authorized task worktree using the existing provisioning script.
---

# Start the selected plan

Apply the [result contract](../../references/workflow-contract.md).
Use the selected plan, source checkout, target branch and base, and provisioning
scope.

Inspect existing branches, worktrees and pending work. Resume an existing plan in
its recorded checkout. For a new selected task worktree use the
[provisioning command](references/provisioning.md#create-a-task-worktree) directly.
The script copies local assets. If an input is missing or the branch or path
already exists, report the obstacle. Do not silently switch destinations.

When the assignment requests a copy, copy the selected plan and its required
linked inputs while keeping their relative locations. Preserve the task
identity and always tell the user which plan governs the work. Report the
checkout, branch, plan path, commands you ran and any partial failures.
No source fingerprints, completion receipts or delivery declarations are required.

A standalone provisioning request ends with the checkout, branch and plan
result. When Implement Task calls this skill, the main session supplies
confirmed execution context and uses the prepared checkout for the assignment.
