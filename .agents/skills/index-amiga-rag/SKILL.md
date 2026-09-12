---
name: index-amiga-rag
description: >-
  Use this skill to perform incremental vector re-indexing of the local Qdrant knowledge base (amiga collection), triggered whenever reference manuals, chip documentation, or hardware assets under Obsidian/Amiga/Reference/ are modified or added.
---

# Recipe: Incremental Amiga RAG Indexing & Asset Ingestion

This skill defines the standardized procedure for incrementally updating the local Qdrant vector database (`amiga` collection) whenever technical reference documentation or visual diagrams under [Obsidian/Amiga/Reference/](../../../Obsidian/Amiga/Reference/) are added, updated, or reorganized per [`.agents/rules/amiga-rag.md`](../../rules/amiga-rag.md).

---

## 1. When to Trigger This Skill

- **Mandatory Trigger:** Modifications, additions, or renames within [Obsidian/Amiga/Reference/](../../../Obsidian/Amiga/Reference/) (e.g. Motorola 68000 PRM, Commodore Amiga Hardware Reference Manual, Amiga Guru Book, and hardware architecture guides).
- **Goal:** Keep the local Qdrant vector search (`rag_search` MCP tool) 100% synchronized with upstream reference documentation without requiring cloud APIs.

---

## 2. Ingestion Tooling & Infrastructure

- **Vector Database:** Local Qdrant instance (`http://localhost:6333`, collection: `amiga`).
- **CLI Runner:** [`tools/rag/bin/amiga_rag.ps1`](../../../tools/rag/bin/amiga_rag.ps1) (PowerShell entry point).
- **Core Indexer:** `tools/rag/rag_qdrant/indexer.py` (handles chunking, token estimation, embedding generation, and Qdrant upserts).
- **Cache File:** Configured in `.env` via `RAG_CACHE_FILE` (tracks SHA256 hashes for fast incremental skips).

---

## 3. Step-by-Step Execution Workflow

### Step 1: Audit Diagram & Asset Sidecars (If Images Touched)
If diagrams, pinouts, or circuit schematics were modified or added:
1. Run the asset manager to detect unindexed images:
   ```powershell
   python tools/rag/rag_qdrant/assets_manager.py "Obsidian/Amiga" --list-unindexed
   ```
2. For any unindexed image, generate or update its companion `<image_path>.txt` description sidecar file per [`.agents/rules/asset-descriptions.md`](../../rules/asset-descriptions.md).

### Step 2: Execute Incremental RAG Ingestion
Run the incremental indexing script:
```powershell
.\tools\rag\bin\amiga_rag.ps1 . --source amiga
```
Or directly target the reference vault:
```powershell
python tools/rag/rag_qdrant/indexer.py "Obsidian/Amiga/Reference" --source amiga
```
- **Incremental Skip Invariant:** Unchanged markdown files and images matching cached hashes are skipped instantly. Only new or modified chunks are re-embedded and upserted.

### Step 3: Verify Status & Collection Health
Verify that the collection is healthy and vector counts reflect the updates:
```powershell
python tools/rag/rag_qdrant/indexer.py --status
```
Or query via MCP: `rag_status` / `rag_list_sources`.
