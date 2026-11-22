---
name: TA-Graphify-explain-symbol
description: Explain one source-qualified symbol using an explicitly selected existing graph and relevant definitions.
---

# Explain the selected graph symbol

Apply [the result contract](../../references/workflow-contract.md).
Executor: main session or assigned specialist.
Inputs: explicit graph file and source-qualified symbol identity.

1. Read [graph provenance](../TA-Graphify-query/references/query.md) and confirm
   which graph and symbol the caller selected. Report missing data or ambiguity
   within that operation.
2. Run `python tools/graphify/read.py explain "<symbol>" --graph <graph-file>`.
   Read relevant source definitions to interpret the stored relationships.
3. Explain the symbol and describe the relationships that support the
   explanation. Identify the graph and sources used. Report any limits due
   to stale data or parser omissions.

Stop after explanation. Do not query another operation, switch corpora, refresh,
extract semantics or write graph/query state.
