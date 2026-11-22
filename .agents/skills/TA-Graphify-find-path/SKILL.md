---
name: TA-Graphify-find-path
description: Find a relationship path between two supplied symbols in one explicitly selected existing graph.
---

# Find the selected relationship path

Apply [the result contract](../../references/workflow-contract.md).
Executor: main session or assigned specialist.
Inputs: explicit graph file and source-qualified start/end identities.

1. Read [graph provenance](../TA-Graphify-query/references/query.md). Confirm the
   requested graph and the identities of both symbols. Report missing or
   ambiguous input.
2. Run `python tools/graphify/read.py path "<start>" "<end>" --graph <graph-file>`.
   Inspect the returned path and relevant definitions. Keep track of where each
   relationship came from.
3. Return the path or report that the search found none. Identify the graph and
   candidate symbols and state what limited the search.

Stop after path discovery. Do not change graph, switch to another graph or another
read operation, refresh data, install tools or dispatch research.
