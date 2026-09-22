---
name: index-amiga-rag
description: Prepare Amiga documentation for retrieval through the project-specific RAG MCP server.
---

# Recipe: Amiga RAG MCP Retrieval & Asset Preparation

This skill defines the Amiga project workflow for preparing technical references and diagrams for retrieval through the registered RAG MCP server. It does not invoke the standalone indexing implementation directly.

---

## 1. When to Trigger This Skill

- **Trigger:** Modifications, additions, or renames within [Obsidian/Amiga/Reference/](../../../Obsidian/Amiga/Reference/) (e.g. Motorola 68000 PRM, Commodore Amiga Hardware Reference Manual, Amiga Guru Book, and hardware architecture guides).
- **Goal:** Preserve accurate project documentation and sidecars so the Amiga RAG MCP can retrieve trustworthy context after its normal ingestion lifecycle.

---

## 2. Project Retrieval Interface

- **MCP Server:** [`tools/amiga-rag-mcp-server/`](../../../tools/amiga-rag-mcp-server/) provides `rag_search`, `rag_list_sources`, `rag_status`, and `rag_reindex`.
- **Collection:** The MCP server retrieves from the shared `projects_docs` collection.
- **Scope:** The Amiga project does not import, launch, or configure the standalone indexing implementation.

---

## 3. Step-by-Step Execution Workflow

### Step 1: Prepare Diagram & Asset Sidecars
If diagrams, pinouts, or circuit schematics were modified or added, generate or update each companion `<image_path>.txt` description sidecar per [`.agents/rules/asset-descriptions.md`](../../rules/asset-descriptions.md).

### Step 2: Verify Retrieval Through MCP
Run `rag_reindex()` to incrementally index the supported Amiga project scopes: `docs`, `Obsidian/Amiga/Design`, and `Obsidian/Amiga/Reference`. Use `rag_reindex(force=true)` only to rebuild those scopes while bypassing the shared SHA-256 cache. Then query a distinctive heading or phrase through `rag_search(query="<topic>", sources=["amiga"])`.

### Step 3: Verify MCP Health
Use `rag_status` and `rag_list_sources` to confirm the registered Amiga RAG MCP server can access the shared collection.
