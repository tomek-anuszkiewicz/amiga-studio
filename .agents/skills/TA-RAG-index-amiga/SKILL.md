---
name: TA-RAG-index-amiga
description: Incrementally index explicitly accepted Amiga documentation through the existing CLI and verify health and retrieval.
---

# Index the accepted documentation

Apply [the result contract](../../references/workflow-contract.md).
Executor: main session.
Inputs: explicit indexing authorization, accepted validated supported roots,
configured shared state path and a retrieval probe. Editing/auditing alone does not
authorize publishing content to the shared index.

1. Confirm `rag_qdrant` and authorized `RAG_INDEX_JSON` configuration. Missing
   configuration is a gap; do not invent private paths, install tools or repair
   documents. The supported roots are `docs/`, `Obsidian/Amiga/Design/` and
   `Obsidian/Amiga/Reference/`; never select their parent to broaden scope.
2. For each explicitly accepted root, run
   `rag_qdrant <absolute-root> --repo-root <absolute-repo> --source amiga --index-json <configured-state>`.
   Preserve normal incremental behavior and actual selected scope.
3. Verify with `rag_qdrant --status --index-json <state> --json`,
   `rag_qdrant --list-sources --index-json <state> --json`, and
   `rag_qdrant search "<probe>" --source amiga --limit 2 --index-json <state> --json`.
4. Return actual indexing/skipped counts, health/retrieval evidence and limitations.

The index identifies each `amiga` document by its repository-relative path,
so worktrees share one indexed version of that document. The most recent content
that the tool indexes successfully replaces the previous version. The indexer
replaces vectors for changed files and removes entries for files deleted under
selected roots. It may skip an unchanged nonempty document that has one fragment
left in the index. In that case, the skip shows only that the indexer left the
entry as it was. The tool
has no force-reindex mode. State that still uses absolute paths requires a
separately authorized rebuild.

Stop after indexing/health checks. Do not index `devnotes`, clear shared state,
repair sources, broaden accepted roots or launch further workflow steps.
