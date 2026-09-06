## Amiga RAG Knowledge Base (Amiga Docs + Obsidian)

You have access to a local knowledge base via the tools `rag_search`, `rag_list_sources`, and `rag_status`.

Knowledge Sources Configuration:
- For this project, ALWAYS search across both sources: `sources=["amiga", "obsidian"]`. This unifies official technical documentation with personal Obsidian notes and research.

Division of Responsibility between RAG and Graphify:
- **Use Graphify** (`graphify query`, `graphify path`, `graphify explain`): For questions about code structure, AST, relationships between source files in this repository, call hierarchies, and architecture.
- **Use RAG** (`rag_search`): For domain knowledge, hardware specifications (OCS/ECS/AGA), register definitions, AmigaOS libraries (Exec, Graphics, Intuition), data formats, and your personal Obsidian research notes.
- **Use Both**: When implementing or debugging a feature — first consult RAG to understand the hardware/library specs, then consult Graphify to locate and navigate the corresponding code in this repository.
- If search results include diagram or image file paths, reference them or use `view_file` when helpful.
- When the user asks about the RAG database state, call `rag_status` or `rag_list_sources`.

Tooling, Reindexing & Infrastructure:
- **Tools & MCP Server**: The ingestion pipeline, CLI (`rag-qdrant-index`), and FastMCP server reside in this repository under [`tools/rag/`](file:///D:/Programowanie/Amiga/tools/rag/).
- **Vector Database Host**: The Qdrant Docker container, persistent vector storage (`qdrant_storage/`), and SHA256 cache (`rag_cache.json`) are hosted at `D:\GoogleDrive\AI\qdrant\`.
- To reindex manuals or notes:
  - Run `.\tools\rag\bin\rag-qdrant-index.bat D:\Programowanie\Amiga --source amiga`
  - Run `.\tools\rag\bin\rag-qdrant-index.bat D:\GoogleDrive\AI\Obsidian --source obsidian`
