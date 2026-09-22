---
name: index-amiga-rag
description: Prepare and incrementally index Amiga documentation through the project RAG CLI.
---

# Recipe: Amiga RAG CLI Indexing & Asset Preparation

This skill defines the Amiga project workflow for indexing technical Markdown through the `rag_qdrant` command-line tool. The MCP server is an optional adapter for agent retrieval; it is not the indexing authority.

---

## 1. When to Trigger This Skill

- **Trigger:** Modifications, additions, or renames within [Obsidian/Amiga/Reference/](../../../Obsidian/Amiga/Reference/) (e.g. Motorola 68000 PRM, Commodore Amiga Hardware Reference Manual, Amiga Guru Book, and hardware architecture guides).
- **Goal:** Preserve accurate project Markdown so the Amiga RAG MCP can retrieve trustworthy context after its normal ingestion lifecycle.

---

## 2. Project Indexing Interface

- **CLI:** `rag_qdrant` is the canonical indexing, health, and search interface. It must be available on `PATH`.
- **Collection:** The CLI operates on the shared `projects_docs` collection.
- **MCP Server:** [`tools/amiga-rag-mcp-server/`](../../../tools/amiga-rag-mcp-server/) delegates to the same CLI when MCP retrieval is available.

---

## 3. Step-by-Step Execution Workflow

### Step 1: Prepare Diagram & Asset Sidecars
If diagrams, pinouts, or circuit schematics were modified or added, generate or update each companion `<image_path>.txt` description sidecar per [`.agents/rules/asset-descriptions.md`](../../rules/asset-descriptions.md). The current CLI does not index those sidecars; index any changed Markdown that describes or embeds them.

### Step 2: Incrementally Index the Supported Scopes
Set `RAG_INDEX_JSON` to the shared state file, then run both commands from the
repository root. The CLI recursively scans Markdown and processes only files
whose SHA-256 hash changed:

```powershell
$env:RAG_INDEX_JSON = "<shared-rag-index-state-file>"
rag_qdrant docs --source amiga --index-json $env:RAG_INDEX_JSON
rag_qdrant "Obsidian/Amiga" --source amiga --index-json $env:RAG_INDEX_JSON
```

There is no forced-reindex mode; changed files replace their old vectors and
files removed below an indexed root are removed from the collection.

### Step 3: Verify Index Health and Retrieval

```powershell
rag_qdrant --status --index-json $env:RAG_INDEX_JSON --json
rag_qdrant --list-sources --index-json $env:RAG_INDEX_JSON --json
rag_qdrant search "<distinctive heading or phrase>" --source amiga --limit 2 --index-json $env:RAG_INDEX_JSON --json
```
