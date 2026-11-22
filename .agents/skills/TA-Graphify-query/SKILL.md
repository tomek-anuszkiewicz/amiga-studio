---
name: TA-Graphify-query
description: Query relationships in one explicitly selected existing project or reference graph without changing it.
---

# Query the selected graph

Apply [the result contract](../../references/workflow-contract.md).
Executor: main session or assigned specialist.
Inputs: explicit graph file, query, scope and source snapshot/freshness if known.

1. Read [graph provenance and query mechanics](references/query.md). Confirm
   that the selected graph exists. If data is missing, report the resulting
   limit and stop before any refresh.
2. Run `python tools/graphify/read.py query "<query>" --graph <graph-file>`.
   Inspect which entities the graph returned and where their relationships came
   from. Read the relevant source definitions before making claims beyond what
   the graph helps you locate.
3. Identify the graph and sources used. Return the query, relevant entities
   and relationships, and evidence from the command. Report limits due to stale
   or truncated data or parser omissions.

Stop after the query. Do not select path/explain, refresh/extract, install tools or
write feedback, vocabulary, query logs or graph data.
