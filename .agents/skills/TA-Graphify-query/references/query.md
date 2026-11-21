# Read-only graph provenance

Select the corpus explicitly: project uses `graphify-out/graph.json` under the
repository; reference uses `ref_src/graphify-out/graph.json` under imported sources.
Reference paths resolve inside `ref_src/`. Compare results from two corpora only
through separately selected operations and explicit attribution, never merged data.

Use source-qualified labels/IDs for ambiguous symbols and confirm source definitions.
Resolve graph paths against the intended checkout when invoking from elsewhere.
Graph relationships are navigation evidence, not hardware truth. Structural and
semantic freshness are separate; missing/stale/truncated data and parser omissions
remain visible. Imported C/C++ and Verilog are supported with possible omissions;
vAmigaTS assembly and JSON vectors are excluded.

Use the repository `tools/graphify/read.py` adapter. Its native traversal suppresses
bytecode, query logging and the pinned raw CLI's query-stamp writer in-process.
The installed package remains unchanged. Raw CLI reads can write
`cache/last_query_stamp` and are unsuitable for this read-only operation.

The caller selects query, path or explain before invoking its corresponding skill.
The adapter accepts that fixed operation with explicit `--graph <file>`.
If native traversal fails, a bounded direct read of the same selected JSON can
inspect stored labels/links and report the failure; do not silently substitute
another corpus or claim native traversal passed.

For an explicitly assigned scoped query, the existing
`tools/graphify/query.py "<query>" --graph <project|reference> --scope <directory>`
creates a disposable filtered view under `.agent/tmp/`. It is not strictly
zero-write execution: it leaves stored graphs intact but creates diagnostics.
Use `read.py` when writes are prohibited. Never refresh, install, extract,
reflect, or write feedback/vocabulary as part of a graph read.
