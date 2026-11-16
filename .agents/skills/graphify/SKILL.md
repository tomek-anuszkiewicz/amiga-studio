---
name: graphify
description: Navigate project or imported-source relationships with independent Graphify datasets, refresh project code, and extract project documentation semantics through the host agent.
---

# Select a graph for the question

| Question | Graph | Scan root | Graph file |
| --- | --- | --- | --- |
| Our callers, dependencies, ownership and allowed project documents | `project` | Repository | `graphify-out/graph.json` |
| Relationships in imported implementations | `reference` | `ref_src/` | `ref_src/graphify-out/graph.json` |

Compare implementations by querying both separately and attributing each result.
Never merge the datasets or identify entities solely by their symbol names.
Confirm relevant source definitions after navigation. Manuals and measurements
establish hardware claims; reference emulators remain models.

## Read existing data

Read [query.md](references/query.md) for read-only native traversal through `read.py` and optional
directory filtering. Every query, path, explanation and inline traversal selects
a graph explicitly. The pinned raw CLI writes query stamps; use the repository adapter, which suppresses
that writer and query logs without changing the package. Missing or stale graphs
are a stated limitation; use direct
source inspection. Reading requires no installation, extraction, feedback,
vocabulary, reflection or graph writes. Hardware researchers use this route.

## Produce current data

Read [update.md](references/update.md). During preparation when indexed code
changed, refresh only `project` once for the stable candidate and await success
before freeze. Worktree provisioning copies independent graph data and refreshes
only project code. Bootstrap `-Graphify` and the Graphify stage of `-All` explicitly
create or refresh both graphs. A dedicated manual reference operation is also
available. Queries and ordinary preparation never create a missing reference graph.

Structural refresh preserves valid project semantic content but does not update
its meaning. For initial project construction and changed documentation needed
as current evidence, use the host-agent semantic procedure in
[update.md](references/update.md), with [extraction-spec.md](references/extraction-spec.md).
Otherwise state the semantic freshness limit. Reference operations are code only
and never dispatch semantic extraction. Unsupported formats and parser omissions
remain visible; in particular vAmigaTS assembly is excluded.

Command mechanics, roots, cache ownership and failure handling belong to the
[tool README](../../../tools/graphify/README.md). Keep this repository-owned skill
when installing/upgrading the one Graphify package; do not reinstall an upstream
skill over it. No global skill or installed package changes are needed. Optional
upstream ingestion, watch, merged graphs and export pipelines are outside these
two maintained datasets; do not run them against either output implicitly.
