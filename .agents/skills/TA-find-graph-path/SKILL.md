---
name: TA-find-graph-path
description: Find a relationship path between two supplied symbols in one explicitly selected existing graph.
---

# Find the selected relationship path

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session or assigned specialist; result kind: `graph`.
Inputs: explicit graph file and source-qualified start/end identities.

1. Read [graph provenance](../TA-query-graph/references/query.md). Confirm the
   requested graph and identities; missing or ambiguous input is reported.
2. Run `python tools/graphify/read.py path "<start>" "<end>" --graph <graph-file>`.
   Inspect the returned path and relevant definitions, retaining edge provenance.
3. Return the path or actual no-path result, graph/candidate identities and limits.

Stop after path discovery. Do not change graph, switch to another graph or another
read operation, refresh data, install tools or dispatch research.

