## Amiga RAG Knowledge Base (Amiga Docs + Obsidian)

You have access to a local knowledge base via the tools `rag_search`, `rag_list_sources`, and `rag_status`.

Knowledge Sources Configuration:
- The Qdrant database hosts the unified `amiga` collection containing two knowledge sources:
  - `amiga`: Official technical documentation, Commodore hardware reference manuals, and chip specifications.
  - `obsidian`: General knowledge and personal research notes.
- For this project, ALWAYS search across both sources: `sources=["amiga", "obsidian"]`.

Division of Responsibility between RAG and Graphify:
- **Use Graphify** (`graphify query`, `graphify path`, `graphify explain`): For questions about code structure, AST, relationships between source files in this repository, call hierarchies, and architecture.
- **Use RAG** (`rag_search`): For domain knowledge, hardware specifications (OCS/ECS/AGA), register definitions, AmigaOS libraries (Exec, Graphics, Intuition), data formats, and your personal Obsidian research notes.
- **Use Both**: When implementing or debugging a feature — first consult RAG to understand the hardware/library specs, then consult Graphify to locate and navigate the corresponding code in this repository.
- If search results include diagram or image file paths, reference them or use `view_file` when helpful.
- When the user asks about the RAG database state, call `rag_status` or `rag_list_sources`.

Tooling, Reindexing & Infrastructure:
- **Tools & MCP Server**: The ingestion pipeline, CLI (`amiga_rag`), and FastMCP server reside in this repository under [`tools/rag/`](tools/rag/) (incremental cache configured in `.env` via `RAG_CACHE_FILE`).
- **Vector Database**: Connects to the local Qdrant instance (`http://localhost:6333`, collection: `amiga`).
- To reindex:
  - Run `.\tools\rag\bin\amiga_rag.bat . --source amiga` (indexes repository technical documentation)
  - Run `.\tools\rag\bin\amiga_rag.bat <PATH_TO_VAULT> --source obsidian` (indexes general knowledge notes)
