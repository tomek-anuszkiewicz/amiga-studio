---
name: index-amiga-rag
description: Re-index Amiga hardware manuals and Obsidian design notes into local Qdrant vector database
---

# Workflow: Index Amiga RAG Knowledge Base

Use this workflow to execute incremental vector ingestion of Amiga hardware manuals, technical reference documentation, and visual diagram sidecars into the local Qdrant vector database (`amiga` collection).

---

## 1. Zero-Parameter Run (`/index-amiga-rag`)
When invoked without parameters:
1. **Audit Diagram Sidecars:**
   - Detects any unindexed images or circuit schematics in `Obsidian/Amiga`:
     ```powershell
     python tools/rag/rag_qdrant/assets_manager.py "Obsidian/Amiga" --list-unindexed
     ```
2. **Execute Incremental Vector Ingestion:**
   - Runs incremental ingestion using the local SHA256 hash cache (skipping unchanged content instantly):
     ```powershell
     .\tools\rag\bin\amiga_rag.ps1 . --source amiga
     ```
3. **Verify Database Health & Status:**
   - Checks Qdrant collection vector counts and status:
     ```powershell
     python tools/rag/rag_qdrant/indexer.py --status
     ```

---

## 2. Targeted Ingestion Commands
- **Index Specific Vault Directory:**
  ```powershell
  python tools/rag/rag_qdrant/indexer.py "Obsidian/Amiga/Reference" --source amiga
  ```
- **Check Collection Status:**
  ```powershell
  python tools/rag/rag_qdrant/indexer.py --status
  ```
- **Check Unindexed Diagram Assets:**
  ```powershell
  python tools/rag/rag_qdrant/assets_manager.py "Obsidian/Amiga" --list-unindexed
  ```

---

## 3. Execution Runbook
Follow the operational procedure in [`.agents/skills/index-amiga-rag/SKILL.md`](../skills/index-amiga-rag/SKILL.md):
1. **Sidecar Verification:** Ensure circuit diagrams and timing charts have accompanying `<image>.txt` technical sidecars per [`.agents/rules/asset-descriptions.md`](../rules/asset-descriptions.md).
2. **Incremental Indexing:** Ingest changed documents and sidecars into Qdrant (`http://localhost:6333`, collection: `amiga`).
3. **Status Query:** Confirm points count matches indexed chunks.

---

## 4. Output Contract
Conclude with the standardized summary report:
```markdown
### 📚 Amiga RAG Ingestion Report
- **Target Knowledge Source:** `amiga` (Commodore HRM, M68000 PRM, Obsidian specs)
- **Files Re-indexed / Modified:** <count> files
- **Unchanged Files Skipped:** <count> files (SHA256 cache)
- **Diagram Sidecars Ingested:** <count> sidecars
- **Qdrant Collection Status:** [HEALTHY | <points_count> vectors]
```
