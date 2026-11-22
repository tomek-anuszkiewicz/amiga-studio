# Knowledge navigation and GUI evidence

Use these tools when they help answer the assigned question. Within the selected
scope, planners, test designers, implementers and reviewers can inspect existing
graphs, retrieve documents and capture supported GUI scenarios. A change to a
document or source file does not itself start an indexing, refresh or research
operation.

## Graphify

The project graph (`graphify-out/graph.json`) covers project code and allowed
documents. The independent reference graph (`ref_src/graphify-out/graph.json`)
covers imported emulator and FPGA sources. Choose the graph that contains the
sources relevant to your question and keep their provenance separate.

- [TA-Graphify-query](../skills/TA-Graphify-query/SKILL.md) answers relationship queries.
- [TA-Graphify-explain-symbol](../skills/TA-Graphify-explain-symbol/SKILL.md) explains a
  source-qualified symbol and its stored relationships.
- [TA-Graphify-find-path](../skills/TA-Graphify-find-path/SKILL.md) finds a relationship
  path between two supplied symbols.
- [TA-Graphify-refresh](../skills/TA-Graphify-refresh/SKILL.md) refreshes one selected
  graph's structure. Task closure includes the project refresh on verified master.
- [TA-Graphify-extract-semantics](../skills/TA-Graphify-extract-semantics/SKILL.md)
  updates selected project document semantics when separately selected.

Planners can trace affected callers and dependencies. Test designers can find
where components interact. Implementers and reviewers can find consumers and
identify which components own the relevant behavior. Hardware researchers use
the reference graph to navigate models.

Verify relevant source definitions before drawing conclusions. A parser that
supports a source format can still miss definitions or relationships. An edge
shows a stored relationship. Assess coverage and hardware behavior using the
relevant sources. When the graph is absent, stale or insufficient, use `rg`
and inspect the source directly. A read-only question
alone is no reason to refresh the graph. Assess how current the graph structure
is separately from how current its document semantics are. Commands and
dataset mechanics are in the [tool reference](../../tools/graphify/README.md).

## RAG and hardware sources

Planners and implementers can use the configured Amiga RAG to retrieve project
requirements and existing explanations. Test designers and reviewers can use it
to find independent expectations and supporting sources. Use the
[retrieval reference](developer-setup.md#5-domain-hardware-knowledge-local-vector-rag-amiga-rag)
for the CLI, available MCP tools and source filters. Verify supporting passages,
especially when a result came from another checkout or contains mixed sources.

[TA-Hardware-research](../skills/TA-Hardware-research/SKILL.md) distinguishes
manuals, original physical measurements and emulator/FPGA models. Project Design
notes and active implementation are not independent proof of physical behavior.

[TA-RAG-index-amiga](../skills/TA-RAG-index-amiga/SKILL.md) publishes explicitly
accepted documentation to the shared index. It is different from searching that
index. Knowing about this operation does not authorize publication; writing,
planning or reviewing documents alone does not select indexing.

## GUI screenshots

For assigned GUI work, use
[TA-capture-gui-screenshot](../skills/TA-capture-gui-screenshot/SKILL.md) when an
image helps establish the current layout, design observable scenarios, implement
a visual change or assess its result. This supporting capture is available to
the planner, test designer, implementer, test runner and reviewer; it does not
require a separate task stage. Choose a scenario supported by the existing
inspector and inspect the produced image when making a visual claim.

A successful capture establishes that the tool produced an image. Report the
scenario, dimensions and limits of what you observed. Evidence of interaction
correctness, complete GUI coverage or arbitrary runtime state requires more
than the success of a capture. For visual fixes,
compare the relevant original and corrected scenario under the shared
[GUI criteria](gui-criteria.md).
