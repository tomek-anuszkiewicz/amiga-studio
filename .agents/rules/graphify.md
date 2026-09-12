---
trigger: always_on
description: Consult the graphify knowledge graph at graphify-out/ for codebase and architecture questions.
---

## graphify

This project has a graphify knowledge graph at graphify-out/.

Rules:
- For codebase or architecture questions, when `graphify-out/graph.json` exists, first run `graphify query "<question>"` (CLI) or `query_graph` (MCP). Use `graphify path "<A>" "<B>"` / `shortest_path` for relationships and `graphify explain "<concept>"` / `get_node` for focused concepts. These return a scoped subgraph, usually much smaller than `GRAPH_REPORT.md` or raw grep output.
- If graphify-out/wiki/index.md exists, navigate it instead of reading raw files
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context
- **Scoped Incremental Subtree Re-indexing (Mandatory Subtree Isolation):**
  - When changes occur in `crates/`: Execute scoped incremental update restricted strictly to crates:
    ```powershell
    graphify update crates/
    ```
  - When changes occur in `ref_src/`: Execute scoped incremental update restricted strictly to reference sources:
    ```powershell
    graphify update ref_src/
    ```
  - **Prohibition of Full-Workspace Crawls on Code Edits:** Never run `graphify update .` automatically across the entire repository during localized code modifications. Re-indexing must remain strictly scoped to the modified directory tree.
- When the user asks to "start graphify", "run graphify", or "update graphify" for the entire repository:
  - Provide the exact terminal command for the user to execute directly in their IDE terminal so they can observe live AST analysis:
    ```powershell
    graphify update .
    ```
  - Alternatively, if the user asks the agent to run it directly, execute `graphify update .` via `run_command`.

