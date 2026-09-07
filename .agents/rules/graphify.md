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
- Do NOT run `graphify update .` automatically without user intent. Updating the graph is a manual step. The agent should remind the user to run `graphify update .` if relevant files were modified during the session.
- When the user asks to "start graphify", "run graphify", or "update graphify":
  - Provide the exact terminal command for the user to execute directly in their IDE terminal so they can observe the live AST analysis and graph generation in real time:
    ```powershell
    graphify update .
    ```
  - Alternatively, if the user asks the agent to run it directly, execute `graphify update .` via `run_command`.
