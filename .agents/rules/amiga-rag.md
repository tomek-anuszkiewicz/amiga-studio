---
trigger: always_on
description: Amiga RAG knowledge base tools and mandatory pre-task conceptual retrieval (source = "obsidian").
---

## Amiga RAG Knowledge Base (Amiga Docs + Obsidian)

You have access to a local knowledge base through the `rag_qdrant` command-line tool. When registered, the Amiga RAG MCP server exposes an optional adapter with `rag_search`, `rag_list_sources`, `rag_status`, and `rag_reindex`.

Knowledge Sources Configuration:
- The Qdrant database hosts the unified `projects_docs` collection containing two distinct knowledge sources:
  - `amiga`: Official Amiga technical documentation, Commodore hardware reference manuals, and chip specifications.
  - `obsidian`: General architecture guidelines, systems design philosophy, operator mental models, and personal research notes.

Mandatory Pre-Task Conceptual Retrieval (`source = "obsidian"`):
- **Task Inception & Planning Rule**: Whenever starting a new feature, refactoring, architectural decision, or non-trivial task (during the research, planning, or design deliberation phase before writing code):
  1. **Query Obsidian Architecture Knowledge**:
     - Actively query the local RAG knowledge base targeting the user's architectural knowledge vault:
       - Via the Amiga RAG MCP tool: `rag_search(query="<task-topic-or-architecture-concept>", sources=["obsidian"])`
     - For tasks involving hardware chipsets, query both or combine queries (`sources=["amiga", "obsidian"]`).
  2. **Context Integration**:
     - Evaluate the retrieved context snippets for relevant architectural principles, operator heuristics, systems design guidance, or ergonomics.
     - Weave applicable insights directly into the reasoning, implementation plan (`implementation_plan.md`), or design approach.
  3. **Graceful Fallback**:
     - If the local RAG service is temporarily offline or yields no matching records for a specific query, proceed cleanly based on repository specifications without inventing citations.

Division of Responsibility between RAG and Graphify:
- **Use Graphify** (`graphify query`, `graphify path`, `graphify explain`): For questions about code structure, AST, relationships between source files in this repository, call hierarchies, and architecture.
- **Use RAG** (MCP `rag_search`): For domain knowledge, hardware specifications (OCS/ECS/AGA), register definitions, AmigaOS libraries (Exec, Graphics, Intuition), data formats, and personal Obsidian research notes.
- **Use Both**: When implementing or debugging a feature — first consult RAG to understand the hardware/library specs and design principles, then consult Graphify to locate and navigate the corresponding code in this repository.
- If search results include diagram or image file paths, reference them or use `view_file` when helpful.
- When the user asks about the RAG database state, call `rag_status` or `rag_list_sources`.

Mandatory Knowledge Retrieval Precedence (Zero Raw Manual Scanning):
- **Pre-Search Mandate**: Whenever investigating hardware registers, chip timing (Agnus, Denise, Paula), memory maps, or custom chip architecture, **MUST FIRST** query the Amiga RAG MCP: `rag_search(query="<query>", sources=["amiga"])`.
- **Prohibition of Direct Manual Crawling**: Never open large reference manuals under `Obsidian/Amiga/Reference/` via `view_file` or perform wide `grep_search` across manuals without first querying the Amiga RAG MCP. Use `view_file` only on the specific targeted section or snippet identified by RAG.

Project CLI Infrastructure:
- **Amiga Integration**: `rag_qdrant` is the canonical interface to the shared `projects_docs` collection. Every operational call requires `--index-json $env:RAG_INDEX_JSON`, naming the shared local state file. After documentation changes, incrementally index the `amiga` source with `rag_qdrant docs --source amiga --index-json $env:RAG_INDEX_JSON`, `rag_qdrant "Obsidian/Amiga/Design" --source amiga --index-json $env:RAG_INDEX_JSON`, and `rag_qdrant "Obsidian/Amiga/Reference" --source amiga --index-json $env:RAG_INDEX_JSON`. Do not index the `Obsidian/Amiga` parent directory. The CLI hashes every included Markdown file and has no forced-reindex mode. The MCP server in [`tools/amiga-rag-mcp-server/`](../../tools/amiga-rag-mcp-server/) is a convenience adapter that delegates to this CLI.



