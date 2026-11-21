---
name: TA-Graphify-refresh
description: Structurally refresh one explicitly selected graph using the existing producer without semantic extraction.
---

# Refresh the selected graph structure

Apply the [result contract](../../references/workflow-contract.md).
Inputs: explicitly selected project/reference graph, source root and output scope.
Selected task closure includes the project refresh on final verified master;
reference refresh remains a separate explicit operation.
Use [the structural procedure](references/update.md).

Confirm the selected existing graph/configuration and source scope. Run
`python tools/graphify/refresh.py --graph <selected-graph>`. Await completion and
inspect output and provenance. Return the command, changed graph artifacts and
freshness limits. Preserved semantic data does not establish that changed meanings
are current. Worktree provisioning copies existing graphs through its own script.

Stop after structural refresh. Do not extract semantics, refresh another graph,
install packages or start an indexing campaign without that scope.
