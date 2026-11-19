---
name: TA-refresh-graph
description: Structurally refresh one explicitly selected graph using the existing producer without semantic extraction.
---

# Refresh the selected graph structure

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session; result kind: `graph`.
Inputs: explicitly selected project/reference graph, source root, trigger and output
scope. Use [the structural procedure](references/update.md).

1. Confirm the selected producer scope and relevant existing graph/configuration.
   For ordinary task closure, select the project graph in the primary checkout
   on local `master` after publication and completion of the language stage.
   Bind the operation to the final SHA, including any closure vocabulary commit.
   Verify that the project sources correspond to that SHA and that a successful
   refresh for the same closure and SHA has not already been recorded.
   Worktree creation copies existing graphs.
2. For ordinary closure, run
   `python tools/verification/verify.py integrate closure-graph record <surviving-record>`.
   It invokes the project producer for that input and records the SHA and output
   identity. A blocked refresh leaves the completed language result available.
   For an explicitly selected bootstrap or reference refresh, run
   `python tools/graphify/refresh.py --graph <selected-graph>`. Await completion
   and inspect output and provenance.
3. Return command, changed graph artifacts, source identity and freshness limits.
   Preserved project semantic data is not a claim that changed meanings are current.

Stop after structural refresh. Do not extract semantics, refresh the other graph,
install/upgrade packages or start an indexing campaign.
