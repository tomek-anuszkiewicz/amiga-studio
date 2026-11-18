---
name: TA-start-plan
description: Start the selected plan in a new isolated task worktree and return its authoritative working location.
---

# Start the selected plan

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session; result kind: `worktree`.
Inputs: selected plan/task ID, source checkout, authorized branch/base and provisioning
scope. Invocation explicitly selects a new worktree; do not ask that choice again.

1. Inspect the current branch, immutable base, worktrees and selected plan identity.
   Preserve pending work and refuse an ambiguous branch/path collision.
2. Follow [physical provisioning](references/provisioning.md) with
   `tools/worktree.ps1 add <branch> -Base <base>` from the surviving checkout.
   Verify required source manifests before and after copying.
3. Copy only the selected plan to the new checkout, preserving its stable task ID.
   Record source/base, branch, copied plan identity, authoritative working copy and
   actual provisioning results using portable references.
4. Return checkout/branch/base, plan and evidence, including any partial provisioning.

Stop after task start. Do not implement, synchronize master, install a workflow,
stage, commit or silently repair failed provisioning with a different operation.
