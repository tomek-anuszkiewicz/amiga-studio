---
name: TA-repair-worktree
description: Restore explicitly identified missing local assets, configuration or graph seeds in an existing worktree.
---

# Repair the selected local assets

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session; result kind: `worktree`.
Inputs: existing target checkout, validated source checkout and exact missing assets
authorized for repair. This operation does not merge master.

1. Inspect the identified missing paths and target ownership; preserve existing
   tracked configuration and unrelated work.
2. Copy only the named missing assets from the validated checkout as independent
   physical copies. Use this operation only for an existing worktree with a
   separately selected repair, never to resume failed creation.
3. Validate the copied contents against the source. Graph repair copies existing files.
   Return repaired paths, source provenance, command results and unresolved gaps.

Stop after asset repair. Do not create another task worktree, merge Git histories,
change shared/global configuration or begin implementation.
