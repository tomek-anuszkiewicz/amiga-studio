# Read-only graph provenance

Select the corpus explicitly. The project graph is `graphify-out/graph.json`
under the repository. The reference graph is `ref_src/graphify-out/graph.json`
under the imported sources, and its paths resolve inside `ref_src/`. To compare
results, select an operation for each corpus and attribute each result to its
source. Keep the graph data separate.

For ambiguous symbols, use labels or IDs that identify their source and confirm
their definitions. Resolve graph paths against the intended checkout when
running the tool from elsewhere. Graph relationships help locate evidence in
source files. Claims about the hardware need their own evidence. The graph's
structure and semantic data can become stale independently. Flag missing, stale
or truncated data and parser omissions. The parser supports imported C/C++ and
Verilog and may omit some relationships. vAmigaTS assembly and JSON vectors are
excluded.

Use the repository `tools/graphify/read.py` adapter. Its native traversal suppresses
bytecode, query logging and the pinned raw CLI's query-stamp writer in-process.
The installed package remains unchanged. Raw CLI reads can write
`cache/last_query_stamp` and are unsuitable for this read-only operation.

The caller chooses query, path or explain and invokes the corresponding skill.
The adapter runs the selected operation with explicit `--graph <file>`. If
native traversal fails, report the failure. You may still read the relevant part
of the same graph JSON directly to inspect its stored labels and links. Keep
the selected corpus and record native traversal as failed even if the direct
read succeeds.

For an explicitly assigned scoped query, the existing
`tools/graphify/query.py "<query>" --graph <project|reference> --scope <directory>`
creates a disposable filtered view under `.agent/tmp/`. The command writes that
view for diagnosis and leaves the stored graphs intact.
Use `read.py` when writes are prohibited. Never refresh, install, extract,
reflect, or write feedback/vocabulary as part of a graph read.
