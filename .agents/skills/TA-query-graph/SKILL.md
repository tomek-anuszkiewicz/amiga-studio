---
name: TA-query-graph
description: Query relationships in one explicitly selected existing project or reference graph without changing it.
---

# Query the selected graph

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session or assigned specialist; result kind: `graph`.
Inputs: explicit graph file, query, scope and source snapshot/freshness if known.

1. Read [graph provenance and query mechanics](references/query.md). Confirm the
   selected graph exists; missing data returns its limitation without refresh.
2. Run `python tools/graphify/read.py query "<query>" --graph <graph-file>`.
   Inspect the returned identities and source-qualified relationships. Read relevant
   source definitions before making claims beyond graph navigation.
3. Return graph/source identity, query, relevant entities/relationships, command
   evidence and stale/truncated/parser limits.

Stop after the query. Do not select path/explain, refresh/extract, install tools or
write feedback, vocabulary, query logs or graph data.
