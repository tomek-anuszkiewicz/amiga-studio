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
   - Ensure every modified or added diagram has an accurate Git-tracked `<image>.txt` sidecar.
2. **Execute Incremental Vector Ingestion:**
   - Runs incremental ingestion using the local SHA256 hash cache (skipping unchanged content instantly):
     ```powershell
     rag_qdrant . --source amiga --include-dirs docs
     rag_qdrant "Obsidian/Amiga" --source amiga --include-dirs Design Reference
     ```
3. **Verify Database Health & Status:**
   - Checks Qdrant collection vector counts and status:
     ```powershell
     rag_qdrant --status
     rag_qdrant --list-sources
     ```

---

## 2. Targeted Ingestion Commands
- **Index Amiga Design and Reference Documents:**
  ```powershell
  rag_qdrant "Obsidian/Amiga" --source amiga --include-dirs Design Reference
  ```
- **Check Collection Status:**
  ```powershell
  rag_qdrant --status
  ```
- **Force a Scoped Rebuild (exceptional only):**
  ```powershell
  rag_qdrant . --source amiga --include-dirs docs --reindex
  rag_qdrant "Obsidian/Amiga" --source amiga --include-dirs Design Reference --reindex
  ```

---

## 3. Execution Runbook
Follow the operational procedure in [`.agents/skills/index-amiga-rag/SKILL.md`](../skills/index-amiga-rag/SKILL.md):
1. **Sidecar Verification:** Ensure circuit diagrams and timing charts have accompanying `<image>.txt` technical sidecars per [`.agents/rules/asset-descriptions.md`](../rules/asset-descriptions.md).
2. **Incremental Indexing:** Run the two canonical `rag_qdrant` commands to ingest changed documents and sidecars into Qdrant under the `amiga` source.
3. **Status Query:** Confirm health, source counts, and retrieval of a distinctive phrase through `rag_qdrant`.

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
