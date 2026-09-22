---
name: index-amiga-rag
description: Prepare and incrementally index Amiga documentation through the project RAG CLI.
---

# Recipe: Amiga RAG CLI Indexing & Asset Preparation

This skill defines the Amiga project workflow for indexing technical Markdown through the `rag_qdrant` command-line tool. The MCP server is an optional adapter for agent retrieval; it is not the indexing authority.

---

## 1. When to Trigger This Skill

- **Trigger:** Immediately after a successful `audit-docs-quality` run under the same conditions: major milestone completion, specification updates, or a documentation/governance review.
- **Order:** Repair and validate the documentation first; then run this skill to index the resulting accepted Markdown.
- **Goal:** Preserve accurate project Markdown so the Amiga RAG MCP can retrieve trustworthy `amiga` context after its normal ingestion lifecycle.

---

## 2. Project Indexing Interface

- **CLI:** `rag_qdrant` is the canonical indexing, health, and search interface. It must be available on `PATH`.
- **Collection:** The CLI operates on the shared `projects_docs` collection.
- **MCP Server:** [`tools/amiga-rag-mcp-server/`](../../../tools/amiga-rag-mcp-server/) delegates to the same CLI when MCP retrieval is available.
- **External Notes Boundary:** `devnotes` belongs to a separate project and is retrieval-only here. This skill never indexes it.

---

## 3. Step-by-Step Execution Workflow

### Step 1: Incrementally Index the Supported Scopes
Set `RAG_INDEX_JSON` to the shared state file, then run all three commands from
the repository root. The CLI recursively scans Markdown and processes only
files whose SHA-256 hash changed. Index the Design and Reference roots
explicitly; do not index the `Obsidian/Amiga` parent directory.

```powershell
$env:RAG_INDEX_JSON = "<shared-rag-index-state-file>"
rag_qdrant docs --source amiga --index-json $env:RAG_INDEX_JSON
rag_qdrant "Obsidian/Amiga/Design" --source amiga --index-json $env:RAG_INDEX_JSON
rag_qdrant "Obsidian/Amiga/Reference" --source amiga --index-json $env:RAG_INDEX_JSON
```

There is no forced-reindex mode; changed files replace their old vectors and
files removed below an indexed root are removed from the collection.

### Step 2: Verify Index Health and Retrieval

```powershell
rag_qdrant --status --index-json $env:RAG_INDEX_JSON --json
rag_qdrant --list-sources --index-json $env:RAG_INDEX_JSON --json
rag_qdrant search "<distinctive heading or phrase>" --source amiga --limit 2 --index-json $env:RAG_INDEX_JSON --json
```
