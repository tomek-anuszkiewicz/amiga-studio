---
trigger: model_decision
description: Consult the graphify knowledge graph at graphify-out/ for codebase and architecture questions; execute incremental updates via graphify update ..
---

## graphify

This project has a graphify knowledge graph at graphify-out/.

Rules:
- **Mandatory Code Navigation Precedence (Zero Whole-File Scanning)**:
  - **AST Query Before File View**: Whenever investigating functions, structs, call hierarchies, or symbol relationships across `crates/`, you **MUST FIRST** run `graphify query "<symbol>"` or `graphify explain "<symbol>"` to pinpoint the exact definition site, owning file, and call dependencies.
  - **Prohibition of Whole-File Exploration**: Never open entire multi-hundred-line source files via `view_file` to search for functions or browse code. Once Graphify identifies the location, read only the targeted line slice (`StartLine` and `EndLine`, maximum $\le 50$ lines around the symbol).
  - **Prohibition of Blind Grep for Symbols**: Do not run broad `grep_search` across `crates/` to locate symbols or callers—use `graphify query` or `graphify path`.
- For codebase or architecture questions, when `graphify-out/graph.json` exists, first run `graphify query "<question>"` (CLI) or `query_graph` (MCP). Use `graphify path "<A>" "<B>"` / `shortest_path` for relationships and `graphify explain "<concept>"` / `get_node` for focused concepts. These return a scoped subgraph, usually much smaller than `GRAPH_REPORT.md` or raw grep output.
- If graphify-out/wiki/index.md exists, navigate it instead of reading raw files.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- **Incremental Knowledge Graph Updates (`graphify update .`):**
  - Graphify maintains an AST-level incremental cache and parses only modified files in 1–2 seconds without LLM calls.
  - Always execute incremental updates for the unified codebase graph from the repository root:
    ```powershell
    graphify update .
    ```
  - This preserves a single, cohesive knowledge graph in `graphify-out/` connecting active emulator crates (`crates/`) and clean-room reference code (`ref_src/vAmiga`).
- When the user asks to "start graphify", "run graphify", or "update graphify":
  - Provide the exact terminal command for the user to execute directly in their IDE terminal so they can observe live AST analysis:
    ```powershell
    graphify update .
    ```
  - Alternatively, if the user asks the agent to run it directly, execute `graphify update .` via `run_command`.

