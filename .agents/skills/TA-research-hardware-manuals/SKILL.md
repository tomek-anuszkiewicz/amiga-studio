---
name: TA-research-hardware-manuals
description: Answer one hardware question from Reference manuals with traceable claims and uncertainty, without switching source families.
---

# Research the selected manual question

Apply [the invocation contract](../../references/workflow-contract.md) and
[hardware provenance](references/provenance.md).
Role: `TA-hardware-researcher`; result kind: `hardware_research`.
Inputs: hardware question, selected manual scope, evidence already available and
round budget. The caller selects manual-first research.

1. Use existing evidence when it resolves the question. Otherwise query
   `rag_search_reference` with a focused question, never the mixed general
   `rag_search` corpus as hardware-manual evidence.
2. Verify returned paths remain in `Obsidian/Amiga/Reference/`. Read targeted
   passages/diagrams when snippets are ambiguous. If retrieval is unavailable or
   insufficient, search the same Reference corpus locally and report that route.
3. Return supported claims, exact source paths/headings or locations, disagreements,
   provenance and unresolved questions.

Stop after this source-family investigation. Missing evidence does not start a
reference-model or measurement operation. No project/source/document edits,
indexing, graph refresh or nested delegation.

