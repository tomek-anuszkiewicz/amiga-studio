---
name: index-amiga-rag
description: Prepare and incrementally index Amiga documentation through the project RAG CLI.
---

# Recipe: Amiga RAG CLI Indexing & Asset Preparation

This skill defines the Amiga project workflow for indexing technical Markdown through the `rag_qdrant` command-line tool. The MCP server is an optional adapter for agent retrieval; it is not the indexing authority.

---

## 1. When to Trigger This Skill

- **Trigger:** An explicit request to index accepted, validated project documentation. A specification edit, commit, milestone, or documentation audit alone does not trigger indexing.
- **Order:** Repair and validate the documentation first; then run this skill to index the accepted Markdown.
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
the repository root. The CLI recursively scans Markdown and processes changed
files or unchanged nonempty documents with no fragment in Qdrant. One remaining
fragment is enough to skip an unchanged document; this is not a completeness
check. Index the Design and Reference roots
explicitly; do not index the `Obsidian/Amiga` parent directory.

```powershell
$env:RAG_INDEX_JSON = "<shared-rag-index-state-file>"
$RepoRoot = (Get-Location).Path
rag_qdrant (Join-Path $RepoRoot "docs") --repo-root $RepoRoot --source amiga --index-json $env:RAG_INDEX_JSON
rag_qdrant (Join-Path $RepoRoot "Obsidian/Amiga/Design") --repo-root $RepoRoot --source amiga --index-json $env:RAG_INDEX_JSON
rag_qdrant (Join-Path $RepoRoot "Obsidian/Amiga/Reference") --repo-root $RepoRoot --source amiga --index-json $env:RAG_INDEX_JSON
```

There is no forced-reindex mode; changed files replace their old vectors and
files removed below the selected scan directory are removed from the collection.
Both directory arguments are absolute. Cache keys and result paths are relative
to the repository root, so worktrees sharing `amiga` share one document version.
The last successfully indexed content replaces that document across worktrees.
Old absolute-path state requires a separately authorized rebuild; do not clear
the shared collection or state as part of an ordinary indexing command.

### Step 2: Verify Index Health and Retrieval

```powershell
rag_qdrant --status --index-json $env:RAG_INDEX_JSON --json
rag_qdrant --list-sources --index-json $env:RAG_INDEX_JSON --json
rag_qdrant search "<distinctive heading or phrase>" --source amiga --limit 2 --index-json $env:RAG_INDEX_JSON --json
```


## 4. Output Contract
Conclude with the standardized summary report:
```markdown
### 📚 Amiga RAG Ingestion Report
- **Target Knowledge Source:** `amiga` (Commodore HRM, M68000 PRM, Obsidian specs)
- **Files Re-indexed / Modified:** <count> files
- **Unchanged Files Skipped:** <count> files (matching SHA-256 and a stored fragment, or an intentionally empty document)
- **Qdrant Collection Status:** [HEALTHY | <points_count> vectors]
```
