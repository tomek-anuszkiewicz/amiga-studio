---
name: TA-Graphify-refresh
description: Structurally refresh one explicitly selected graph using the existing producer without semantic extraction.
---

# Refresh the selected graph structure

Apply the [result contract](../../references/workflow-contract.md).
Use the explicitly selected project or reference graph, source root and output
scope. When task closure is selected, refresh the project graph on final
verified master. The reference graph requires a separate explicit selection.
Use [the structural procedure](references/update.md).

Confirm the selected existing graph/configuration and source scope. Run
`python tools/graphify/refresh.py --graph <selected-graph>`. Wait for it to
finish, then inspect its output and where the data came from. Report the command,
the graph files that changed and any limits on how current they are. If the
refresh preserves stored semantic data, its presence alone cannot show whether
the graph reflects changed meanings. The worktree provisioning script copies
existing graphs when it prepares a worktree.

Stop after structural refresh. Do not extract semantics, refresh another graph,
install packages or start an indexing campaign without that scope.
