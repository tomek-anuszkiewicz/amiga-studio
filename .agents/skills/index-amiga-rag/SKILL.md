---
name: index-amiga-rag
description: Prepare and incrementally index Amiga documentation through the project RAG CLI.
---

# Recipe: Amiga RAG CLI Indexing & Asset Preparation

This skill defines the Amiga project workflow for preparing technical references and diagrams for retrieval through the `rag_qdrant` command-line tool. The MCP server is an optional adapter for agent retrieval; it is not the indexing authority.

---

## 1. When to Trigger This Skill

- **Trigger:** Modifications, additions, or renames within [Obsidian/Amiga/Reference/](../../../Obsidian/Amiga/Reference/) (e.g. Motorola 68000 PRM, Commodore Amiga Hardware Reference Manual, Amiga Guru Book, and hardware architecture guides).
- **Goal:** Preserve accurate project documentation and sidecars so the Amiga RAG MCP can retrieve trustworthy context after its normal ingestion lifecycle.

---

## 2. Project Indexing Interface

- **CLI:** `rag_qdrant` is the canonical indexing, health, and search interface. It must be available on `PATH`.
- **Collection:** The CLI operates on the shared `projects_docs` collection.
- **MCP Server:** [`tools/amiga-rag-mcp-server/`](../../../tools/amiga-rag-mcp-server/) delegates to the same CLI when MCP retrieval is available.

---

## 3. Step-by-Step Execution Workflow

### Step 1: Prepare Diagram & Asset Sidecars
If diagrams, pinouts, or circuit schematics were modified or added, generate or update each companion `<image_path>.txt` description sidecar per [`.agents/rules/asset-descriptions.md`](../../rules/asset-descriptions.md).

### Step 2: Incrementally Index the Supported Scopes
Run both commands from the repository root. They reuse the SHA-256 cache and therefore process only changed files:

```powershell
rag_qdrant . --source amiga --include-dirs docs
rag_qdrant "Obsidian/Amiga" --source amiga --include-dirs Design Reference
```

Use `--reindex` on both commands only for a complete scoped rebuild that intentionally bypasses the cache.

### Step 3: Verify Index Health and Retrieval

```powershell
rag_qdrant --status
rag_qdrant --list-sources
rag_qdrant search "<distinctive heading or phrase>" --source amiga --limit 2 --json
```
