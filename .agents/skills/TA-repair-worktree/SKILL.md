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
2. Follow the selected-scope [physical-copy procedure](../TA-start-plan/references/provisioning.md).
   Use the existing full `tools/worktree.ps1 sync` only for the supplied full
   asset-repair scope; a limited repair copies only named missing assets.
3. Validate copied source manifests and applicable project-graph refresh outcome.
   Return repaired paths, source provenance, command results and unresolved gaps.

Stop after asset repair. Do not create another task worktree, merge Git histories,
change shared/global configuration or begin implementation.
