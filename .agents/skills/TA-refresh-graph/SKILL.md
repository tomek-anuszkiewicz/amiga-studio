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
   Ordinary commit preparation selects the project graph. Worktree creation copies existing graphs.
2. Run `python tools/graphify/refresh.py --graph <selected-graph>` for that input.
   Await completion and inspect actual output/provenance.
3. Return command, changed graph artifacts, source identity and freshness limits.
   Preserved project semantic data is not a claim that changed meanings are current.

Stop after structural refresh. Do not extract semantics, refresh the other graph,
install/upgrade packages or start an indexing campaign.
