---
name: index-amiga-rag
description: Re-index Amiga hardware manuals and Obsidian design notes into local Qdrant vector database
---

# Workflow: Index Amiga RAG Knowledge Base

Use this workflow to execute incremental vector ingestion of Amiga hardware manuals and technical reference documentation into the local Qdrant `projects_docs` collection under the `amiga` source tag.

---

## 1. Zero-Parameter Run (`/index-amiga-rag`)
When invoked without parameters:
1. **Execute Incremental Vector Ingestion:**
   - Sets the CLI state file and runs incremental ingestion (skipping unchanged Markdown hashes):
     ```powershell
     $env:RAG_INDEX_JSON = "<shared-rag-index-state-file>"
     rag_qdrant docs --source amiga --index-json $env:RAG_INDEX_JSON
     rag_qdrant "Obsidian/Amiga/Design" --source amiga --index-json $env:RAG_INDEX_JSON
     rag_qdrant "Obsidian/Amiga/Reference" --source amiga --index-json $env:RAG_INDEX_JSON
     ```
2. **Verify Database Health & Status:**
   - Checks Qdrant collection vector counts and status:
     ```powershell
     rag_qdrant --status --index-json $env:RAG_INDEX_JSON --json
     rag_qdrant --list-sources --index-json $env:RAG_INDEX_JSON --json
     ```

---

## 2. Targeted Ingestion Commands
- **Index Amiga Design and Reference Documents:**
  ```powershell
  rag_qdrant "Obsidian/Amiga/Design" --source amiga --index-json $env:RAG_INDEX_JSON
  rag_qdrant "Obsidian/Amiga/Reference" --source amiga --index-json $env:RAG_INDEX_JSON
  ```
  The two explicit scopes exclude `.obsidian` without relying on a parent-root
  exclusion rule.
- **Check Collection Status:**
  ```powershell
  rag_qdrant --status --index-json $env:RAG_INDEX_JSON --json
  ```
- **State Management:** The CLI state file records hashes, sources, and chunk
  counts. It is required on every command and the CLI has no forced-reindex mode.

---

## 3. Execution Runbook
Follow the operational procedure in [`.agents/skills/index-amiga-rag/SKILL.md`](../skills/index-amiga-rag/SKILL.md):
1. **Incremental Indexing:** Run the three canonical `rag_qdrant` commands to ingest changed Markdown into Qdrant under the `amiga` source.
2. **Status Query:** Confirm health, source counts, and retrieval of a distinctive phrase through `rag_qdrant`.

---

## 4. Output Contract
Conclude with the standardized summary report:
```markdown
### 📚 Amiga RAG Ingestion Report
- **Target Knowledge Source:** `amiga` (Commodore HRM, M68000 PRM, Obsidian specs)
- **Files Re-indexed / Modified:** <count> files
- **Unchanged Files Skipped:** <count> files (SHA256 cache)
- **Qdrant Collection Status:** [HEALTHY | <points_count> vectors]
```
