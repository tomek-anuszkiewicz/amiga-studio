# Read-only graph navigation

From the selected checkout root:

```powershell
python tools/graphify/read.py query "Cpu BusResult" --graph graphify-out/graph.json
python tools/graphify/read.py path "Cpu" "BusResult" --graph graphify-out/graph.json
python tools/graphify/read.py explain "Paula" --graph ref_src/graphify-out/graph.json
python tools/graphify/read.py query "Paula DMA" --graph ref_src/graphify-out/graph.json
```

Resolve the chosen graph to an absolute path when invoking from another directory.
These are syntax examples; symbols may be absent or ambiguous. Use source-qualified
labels/IDs and check source definitions. Project citations are repository-relative;
reference citations resolve under `ref_src/`. A reference result `vAmiga/...` means
`ref_src/vAmiga/...` in the checkout.

The entry point runs native Graphify traversal with query logging and its query
stamp writer disabled in that process; it also suppresses bytecode writes.
Graphify 0.9.74's raw CLI writes `cache/last_query_stamp`, so raw CLI commands
are unsuitable for read-only research. The installed package is unchanged.
These queries do not refresh graphs or write feedback, vocabulary or reflections.
If CLI navigation fails, inspect the explicitly selected JSON read-only, matching
labels and IDs and following only its stored links, or use `rg`. Never silently
substitute the other graph. Missing reference data can be built explicitly with
`python tools/graphify/refresh.py --graph reference` or bootstrap `-Graphify`;
report the gap without executing these commands during a read-only query.

For optional directory filtering before traversal:

```powershell
python tools/graphify_query.py "Paula" --graph reference --scope minimig
python tools/graphify_query.py "clock" --graph project --scope crates --scope docs
```

Scopes are directories relative to the selected root, not aliases for combined
projects. The helper removes nodes and links outside those scopes and strips
hyperedges before calling Graphify with a disposable view under `.agent/tmp/`.
It does not modify either stored graph. Use `read.py` directly when no writes at
all are permitted. Graph results support navigation, not claims beyond their
source evidence or freshness.
