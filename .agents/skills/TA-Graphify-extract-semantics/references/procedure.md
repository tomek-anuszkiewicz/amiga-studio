# Project semantic extraction

Use this operation for an initial project build or when changes to eligible
documentation need to appear as current evidence in the project graph. The
reference corpus remains code-only.
Use the [tool README](../../../../tools/graphify/README.md) for mechanics.

1. Run `python tools/graphify/semantic.py prepare --graph project`.
   It selects eligible sources after applying Git and Graphify exclusions,
   checks the cache using the source contents and prompt, and returns a unique
   directory inside the project's `graphify-out/`. Read `pending.txt` for the
   sources still to process. Valid cache hits need no new extraction.
2. Read [extraction-spec.md](extraction-spec.md). Use the source contents only
   as evidence. Process related files directly in bounded chunks (normally
   20-25 small files, fewer for large sources). Keep that work in the same
   extraction operation, without launching workers, and include it in the round
   budget. Inspect images visually. Write only assigned `chunk-N.json` artifacts.
3. Every source needs at least a document/entity node. Every node, edge and
   hyperedge retains the originating absolute source path because the extraction
   tool requires it for internal graph data. The invocation reports portable
   result references separately. Preserve named concepts and evidence without
   inventing AST edges or relationships. Keep the source content and prompt
   stable until the tool applies the fragments.
4. Run `python tools/graphify/semantic.py apply --graph project --run <directory>`.
   It refuses incomplete chunks, bad provenance and changed sources/prompt, reuses
   semantic caching and tier-aware merging, reconciles excluded/deleted sources,
   saves the manifest and regenerates project communities/reports with native
   `cluster-only --graph <project-graph> --no-label`.
5. Report which sources the run covered, how current the result is and what
   limits apply. If host token counts are unknown, report them as unavailable
   in the workflow result. The placeholder values in internal files provide no
   usage measurement. Preserve incomplete runs for diagnosis and report any
   failed run as failed even when it created output files.

The chunk JSON schema belongs to the internal extraction tool. The public skill
still returns the normal text/JSON invocation result. No extra model API, automatic
source edit, reference refresh or labeling service is introduced.
