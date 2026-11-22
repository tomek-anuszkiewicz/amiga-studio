# Structural refresh

The project and reference datasets remain independent. For source roots,
interpreter selection, ownership of outputs and cache files, file copying and
failure handling, see the [tool README](../../../../tools/graphify/README.md).

Use the existing producer to refresh the explicitly selected graph from its
stated source root. For a project refresh, run it in the primary checkout or
the selected task checkout and report which sources it covered. The refresh
requires neither publication nor a language receipt. During task closure,
refresh only the project graph in the primary checkout on final master. Do
this after the final tests and review succeed and any corrective commits are
in place.
Task worktrees use copied graphs and direct source inspection if needed.
The reference refresh remains an explicitly selected manual operation. Bootstrap
`-Graphify` and the Graphify stage of `-All` explicitly produce both datasets;
this does not authorize a query or ordinary preparation to create reference data.

Structural refresh carries forward valid project semantic content. If that
content needs updating, select the extraction operation separately. Reference
graphs are code-only, with no semantic extraction.
Do not add a merged graph, watch service, labeling backend or optional upstream
pipeline. Preserve tracked `ref_src/.graphifyignore` during worktree provisioning
and the repository-owned skill when the installed package changes.
