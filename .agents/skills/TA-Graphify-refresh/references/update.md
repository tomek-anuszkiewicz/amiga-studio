# Structural refresh

The [tool README](../../../../tools/graphify/README.md) owns roots, interpreter
selection, output/cache ownership, physical copying and failure handling.
Project and reference datasets remain independent.

Refresh the explicitly selected graph from its stated source root using the
existing producer. A project refresh can use the primary or selected task
checkout; report which sources it covered. No publication or language receipt
is required. Task closure refreshes only the project graph in the primary checkout
on final master after successful final tests/review and any corrective commits.
Task worktrees use copied graphs and direct source inspection if needed.
The reference refresh remains an explicitly selected manual operation. Bootstrap
`-Graphify` and the Graphify stage of `-All` explicitly produce both datasets;
this does not authorize a query or ordinary preparation to create reference data.

Structural refresh preserves valid project semantic content but does not update
its meaning. Needed project semantics use a separately selected extraction
operation. Reference graphs are code-only, with no semantic extraction.
Do not add a merged graph, watch service, labeling backend or optional upstream
pipeline. Preserve tracked `ref_src/.graphifyignore` during worktree provisioning
and the repository-owned skill when the installed package changes.
