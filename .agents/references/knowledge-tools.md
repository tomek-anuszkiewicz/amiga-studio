# Knowledge navigation and GUI evidence

Use these capabilities when they help answer the assigned question. Planning,
test design, implementation and review can inspect existing graphs, retrieve
documents and capture supported GUI scenarios within their selected scope.
This reference makes the tools discoverable; it does not start an indexing,
refresh or research operation merely because a document or source file changed.

## Graphify

Choose the project graph (`graphify-out/graph.json`) for project code and allowed
documents, or the independent reference graph (`ref_src/graphify-out/graph.json`)
for imported emulator and FPGA sources. Never combine their provenance.

- [TA-Graphify-query](../skills/TA-Graphify-query/SKILL.md) answers relationship queries.
- [TA-Graphify-explain-symbol](../skills/TA-Graphify-explain-symbol/SKILL.md) explains a
  source-qualified symbol and its stored relationships.
- [TA-Graphify-find-path](../skills/TA-Graphify-find-path/SKILL.md) finds a relationship
  path between two supplied symbols.
- [TA-Graphify-refresh](../skills/TA-Graphify-refresh/SKILL.md) refreshes one selected
  graph's structure. Task closure includes the project refresh on verified master.
- [TA-Graphify-extract-semantics](../skills/TA-Graphify-extract-semantics/SKILL.md)
  updates selected project document semantics when separately selected.

Planning can trace affected callers and dependencies; test design can locate
interaction boundaries; implementation and reviewers can follow consumers and
ownership. Hardware research uses the reference graph for model navigation.
Verify relevant source definitions before drawing conclusions. Parser support
and a graph edge do not prove complete coverage or hardware behavior. When the
graph is absent, stale or insufficient, use `rg` and direct source inspection.
Do not refresh it merely to answer a read-only question. Structural freshness
and document-semantic freshness are separate observations. The
[tool reference](../../tools/graphify/README.md) owns commands and dataset mechanics.

## RAG and hardware sources

Planning and implementation can retrieve project requirements and existing
explanations through the configured Amiga RAG; test design and review can use it
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

A successful capture only establishes that the image was produced. It does not
establish interaction correctness, complete GUI coverage or arbitrary runtime
state. Report the scenario, dimensions and observation limits. For visual fixes,
compare the relevant original and corrected scenario under the shared
[GUI criteria](gui-criteria.md).
