---
name: TA-start-plan
description: Start the selected plan in an isolated task worktree and return its authoritative working location.
---

# Start the selected plan

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session; result kind: `worktree`.
Inputs: selected plan/task ID, source checkout, authorized branch/base and provisioning
scope, linked design inputs and recorded amendments, if any.
[Starting a saved plan](../../references/workflow-guidance.md#start-or-deliver-a-saved-plan)
selects this location without a second choice.

1. Confirm the [telemetry startup prerequisite](../../references/workflow-contract.md#telemetry-startup-prerequisite).
   Inspect the current branch, immutable base, worktrees and selected plan and
   design identities before provisioning. On resume use the start record to
   resolve the recorded checkout and working copy, then recheck approved inputs
   and [proposals](../../references/workflow-guidance.md#amend-an-active-plan).
   Preserve pending work. A missing input, ambiguous checkout or branch/path
   collision blocks new provisioning.
2. Run the [worktree creation command](references/provisioning.md#create-a-task-worktree)
   directly from the surviving checkout. Proceed once it succeeds.
3. Copy the exact selected plan, linked design inputs and applicable amendment
   records at their relative locations. Preserve the task ID and compare SHA-256
   hashes to the selected inputs before handing them off. An ignored plan can
   remain ignored. Record source/base, branch, copied input identities,
   authoritative working copy and actual provisioning results using portable
   references.
4. Return checkout, branch, base, plan and evidence, including any partial
   provisioning. Give the caller a link that opens the authoritative working plan
   in its checkout.

Stop after task start. Do not implement, synchronize master, install a workflow,
stage, commit or silently repair failed provisioning with a different operation.
