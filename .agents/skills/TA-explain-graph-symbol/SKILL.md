---
name: TA-explain-graph-symbol
description: Explain one source-qualified symbol using an explicitly selected existing graph and relevant definitions.
---

# Explain the selected graph symbol

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session or assigned specialist; result kind: `graph`.
Inputs: explicit graph file and source-qualified symbol identity.

1. Read [graph provenance](../TA-query-graph/references/query.md). Confirm graph and
   symbol; report ambiguity or missing data without changing the operation.
2. Run `python tools/graphify/read.py explain "<symbol>" --graph <graph-file>`.
   Read relevant source definitions to interpret the stored relationships.
3. Return the explanation, graph/source identities, supporting relationships and
   actual freshness/parser limits.

Stop after explanation. Do not query another operation, switch corpora, refresh,
extract semantics or write graph/query state.
